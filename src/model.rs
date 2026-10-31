//! Project data and the frame compositor.
use crate::anim;
use crate::gfx::*;
use std::sync::Arc;
use tiny_skia::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind { Anim, Text, Image, Audio }

pub enum AssetData { Image(Pixmap), Audio { sr: u32, ch: Vec<Vec<f32>> } }
pub struct Asset { pub name: String, pub data: AssetData }
impl Asset {
    pub fn duration(&self) -> f32 { match &self.data { AssetData::Audio { sr, ch } => ch.first().map_or(0.0, |c| c.len() as f32 / *sr as f32), _ => 0.0 } }
}

#[derive(Clone, Copy, Debug)]
pub struct Tf { pub x: f32, pub y: f32, pub s: f32, pub r: f32, pub o: f32 }
impl Default for Tf { fn default() -> Self { Tf { x: 0.0, y: 0.0, s: 1.0, r: 0.0, o: 1.0 } } }

#[derive(Clone, Debug)]
pub struct TextP { pub text: String, pub style: String, pub size: f32, pub color: Col, pub hl: Col, pub align: String, pub in_a: String, pub out_a: String }
impl Default for TextP {
    fn default() -> Self { TextP { text: "Your *headline* here".into(), style: "headline".into(), size: 9.0, color: PAPER, hl: LIME, align: "center".into(), in_a: "rise".into(), out_a: "fade".into() } }
}

#[derive(Clone)]
pub struct Clip {
    pub id: u32, pub kind: Kind, pub name: String, pub track: usize, pub start: f32, pub dur: f32, pub trim: f32,
    pub tf: Tf, pub kf: [Vec<(f32, f32)>; 5], pub lin: bool,
    pub p: Params, pub txt: TextP,
    pub asset: Option<Arc<Asset>>, pub file: String,
    pub fit: String, pub w: f32, pub shape: String, pub bw: f32, pub bc: Col, pub shadow: bool,
    pub vol: f32, pub mute: bool, pub fade_in: f32, pub fade_out: f32,
}
impl Clip {
    pub fn blank(kind: Kind) -> Clip {
        Clip { id: 0, kind, name: String::new(), track: 2, start: 0.0, dur: 4.0, trim: 0.0, tf: Tf::default(), kf: Default::default(), lin: false,
               p: Params::default(), txt: TextP::default(), asset: None, file: String::new(), fit: "free".into(), w: 40.0, shape: "none".into(), bw: 0.0,
               bc: LIME, shadow: false, vol: 1.0, mute: false, fade_in: 0.0, fade_out: 0.0 }
    }
    pub fn new_anim(id: &str) -> Clip {
        let t = anim::tpl(id).unwrap_or(&anim::TPLS[0]);
        let mut c = Clip::blank(Kind::Anim);
        c.p = Params { tpl: t.id.into(), tag: t.sample[0].into(), num: t.sample[1].into(), head: t.sample[2].into(), sub: t.sample[3].into(), data: t.sample[4].into(), col: Default::default(), bgm: if t.clear { "clear".into() } else { "brand".into() } };
        c.name = t.name.into(); c.dur = t.dur; c.track = if t.clear { 1 } else { 2 };
        c
    }
    pub fn new_text() -> Clip { let mut c = Clip::blank(Kind::Text); c.name = "Text".into(); c.dur = 3.5; c.track = 0; c }
    pub fn end(&self) -> f32 { self.start + self.dur }
    pub fn ev(&self, prop: usize, lt: f32) -> f32 {
        let a = &self.kf[prop];
        let base = [self.tf.x, self.tf.y, self.tf.s, self.tf.r, self.tf.o][prop];
        if a.is_empty() { return base; }
        if lt <= a[0].0 { return a[0].1; }
        for i in 1..a.len() {
            if lt <= a[i].0 {
                let (p, q) = (a[i - 1], a[i]);
                let mut f = (lt - p.0) / (if (q.0 - p.0).abs() < 1e-6 { 1.0 } else { q.0 - p.0 });
                if !self.lin { f = eio(f); }
                return p.1 + (q.1 - p.1) * f;
            }
        }
        a[a.len() - 1].1
    }
    pub fn add_key(&mut self, prop: usize, t: f32, v: f32) {
        let a = &mut self.kf[prop];
        if let Some(k) = a.iter_mut().find(|k| (k.0 - t).abs() < 0.02) { k.1 = v; } else { a.push((t, v)); a.sort_by(|m, n| m.0.partial_cmp(&n.0).unwrap()); }
    }
}

#[derive(Clone)]
pub struct Project { pub w: u32, pub h: u32, pub fps: u32, pub bg: String, pub clips: Vec<Clip>, pub nid: u32, pub assets: Vec<Arc<Asset>> }
impl Project {
    pub fn new() -> Project { Project { w: 1920, h: 1080, fps: 30, bg: "deep".into(), clips: vec![], nid: 1, assets: vec![] } }
    pub fn duration(&self) -> f32 { self.clips.iter().map(|c| c.end()).fold(0.0, f32::max) }
    pub fn overlaps(&self, c: &Clip) -> bool {
        self.clips.iter().any(|o| o.id != c.id && o.track == c.track && o.start < c.end() - 0.001 && o.end() > c.start + 0.001)
    }
    /// Move the clip later until it no longer overlaps a clip on its lane.
    pub fn free_start(&self, c: &mut Clip) {
        for _ in 0..50 {
            let mut moved = false;
            for o in &self.clips {
                if o.id != c.id && o.track == c.track && o.start < c.end() - 0.001 && o.end() > c.start + 0.001 { c.start = o.end(); moved = true; }
            }
            if !moved { break; }
        }
    }
    pub fn add(&mut self, mut c: Clip, prefer: &[usize]) -> u32 {
        c.id = self.nid; self.nid += 1;
        if !prefer.is_empty() {
            let mut chosen = prefer[0];
            for &t in prefer { c.track = t; if !self.overlaps(&c) { chosen = t; break; } }
            c.track = chosen;
        }
        self.free_start(&mut c);
        let id = c.id; self.clips.push(c); id
    }
    pub fn get(&self, id: u32) -> Option<&Clip> { self.clips.iter().find(|c| c.id == id) }
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Clip> { self.clips.iter_mut().find(|c| c.id == id) }
    pub fn find_asset(&self, name: &str) -> Option<Arc<Asset>> {
        let n = name.trim().to_lowercase();
        self.assets.iter().rev().find(|a| a.name.to_lowercase() == n).cloned()
    }
}

pub fn bg_color(mode: &str) -> Option<Col> {
    match mode { "deep" => Some(DEEP), "ink" => Some(INK), "black" => Some(Col(0, 0, 0)), "chroma" => Some(Col(0, 255, 0)), _ => None }
}

/// Renders frames of a project at a fixed pixel size.
pub struct Renderer { main: Cv, scratch: Cv }
impl Renderer {
    pub fn new(w: u32, h: u32) -> Renderer { Renderer { main: Cv::new(w, h), scratch: Cv::new(w, h) } }
    pub fn size(&self) -> (u32, u32) { (self.main.pm.width(), self.main.pm.height()) }
    pub fn pixmap(&self) -> &Pixmap { &self.main.pm }

    pub fn render(&mut self, prj: &Project, t: f32) -> &Pixmap {
        let (w, h) = (self.main.w, self.main.h);
        self.main.clear();
        match bg_color(&prj.bg) { Some(c) => self.main.fill_rect(0.0, 0.0, w, h, c), None => {} }
        for tr in (0..3).rev() {
            for c in prj.clips.iter().filter(|c| c.track == tr && c.kind != Kind::Audio) {
                if t >= c.start - 1e-6 && t < c.end() { self.draw_clip(c, t); }
            }
        }
        &self.main.pm
    }

    fn draw_clip(&mut self, c: &Clip, t: f32) {
        let (w, h) = (self.main.w, self.main.h);
        let lt = t - c.start;
        let tf = Tf { x: c.ev(0, lt), y: c.ev(1, lt), s: c.ev(2, lt), r: c.ev(3, lt), o: c.ev(4, lt) };
        if tf.o <= 0.0 { return; }
        match c.kind {
            Kind::Image => self.draw_image(c, &tf),
            Kind::Anim | Kind::Text => {
                let s = &mut self.scratch;
                s.clear();
                if c.kind == Kind::Anim {
                    match bg_color(match c.p.bgm.as_str() { "brand" => "deep", "chroma" => "chroma", _ => "" }) { Some(_) => { let bg = if c.p.bgm == "chroma" { Col(0, 255, 0) } else { c.p.theme().bg }; s.fill_rect(0.0, 0.0, w, h, bg); } None => {} }
                    anim::draw(s, &c.p, lt + c.trim, c.trim + c.dur);
                } else {
                    draw_text(s, c, lt);
                }
                let main = &mut self.main;
                main.save();
                main.translate(w / 2.0 + tf.x / 100.0 * w, h / 2.0 + tf.y / 100.0 * h);
                main.rotate(tf.r);
                main.scale(tf.s, tf.s);
                main.translate(-w / 2.0, -h / 2.0);
                main.draw_pixmap(&self.scratch.pm, 0.0, 0.0, tf.o);
                main.restore();
            }
            Kind::Audio => {}
        }
    }

    fn draw_image(&mut self, c: &Clip, tf: &Tf) {
        let Some(a) = &c.asset else { return };
        let AssetData::Image(img) = &a.data else { return };
        let (w, h) = (self.main.w, self.main.h);
        let u = self.main.u;
        let (sw, sh) = (img.width() as f32, img.height() as f32);
        let free = c.fit == "free";
        let (mut bw, mut bh) = if free { let bw = c.w / 100.0 * w; (bw, bw * sh / sw) } else {
            let k = if c.fit == "contain" { (w / sw).min(h / sh) } else { (w / sw).max(h / sh) };
            (sw * k, sh * k)
        };
        let shape = if free { c.shape.as_str() } else { "none" };
        if free && shape == "circle" { bh = bw; }
        let (bx, by) = (-bw / 2.0, -bh / 2.0);
        let rd = if shape == "circle" { bw / 2.0 } else if shape == "round" { bw.min(bh) * 0.1 } else { 0.0 };
        let (ba, ia) = (bw / bh, sw / sh);
        let (mut a0, mut b0, mut w0, mut h0) = (0.0, 0.0, sw, sh);
        if ia > ba { w0 = sh * ba; a0 = (sw - w0) / 2.0; } else { h0 = sw / ba; b0 = (sh - h0) / 2.0; }
        let m = &mut self.main;
        m.save();
        m.translate(w / 2.0 + tf.x / 100.0 * w, h / 2.0 + tf.y / 100.0 * h);
        m.rotate(tf.r);
        m.scale(tf.s, tf.s);
        let path = Cv::rrect_path(bx, by, bw, bh, [rd; 4]);
        if let Some(p) = &path {
            if c.shadow {
                for i in 0..8 { let k = i as f32; if let Some(sp) = Cv::rrect_path(bx - k * 3.0 * u, by - k * 3.0 * u + 14.0 * u, bw + k * 6.0 * u, bh + k * 6.0 * u, [rd + k * 3.0 * u; 4]) { m.fill_path_a(&sp, Col(0, 0, 0), 0.07 * tf.o); } }
            }
            let local = Transform::from_row(bw / w0, 0.0, 0.0, bh / h0, bx - a0 * bw / w0, by - b0 * bh / h0);
            m.fill_image(p, img, local, tf.o);
            if free && c.bw > 0.0 { m.stroke_path(p, c.bc, tf.o, c.bw * u, LineCap::Butt, LineJoin::Miter, None); }
        }
        m.restore();
    }
}

fn draw_text(s: &mut Cv, c: &Clip, lt: f32) {
    let o = &c.txt; let u = s.u; let (w, h) = (s.w, s.h);
    let serif = o.style == "serif"; let label = o.style == "label";
    let sz = o.size / 100.0 * h;
    let bo = BO { w: w * 0.84, h: h * 0.85, s0: sz, s1: (10.0 * u).max(sz * 0.3), fam: if serif { 'N' } else { 'A' },
        wt: if serif { 500 } else if label { 800 } else { 900 }, up: !serif && !label, sp: if serif || label { 0.0 } else { -0.035 }, lh: if serif { 1.28 } else if label { 1.15 } else { 0.98 } };
    let b = s.block(&o.text, bo);
    let y = (h - b.th) / 2.0; let center = o.align != "left"; let x = if center { w / 2.0 } else { w * 0.08 };
    let q = sg(lt, c.dur - 0.5, c.dur); let oa = if o.out_a == "fade" { 1.0 - q } else { 1.0 };
    if o.in_a == "words" {
        let mut i = 0.0f32;
        for (li, l) in b.l.iter().enumerate() {
            let mut tw = 0.0; for (j, k) in l.iter().enumerate() { tw += k.tw + if j > 0 && !k.g { b.sp } else { 0.0 }; }
            let mut px = if center { x - tw / 2.0 } else { x }; let by = y + li as f32 * b.s * b.lh + b.s * 0.82;
            for (j, k) in l.iter().enumerate() {
                if j > 0 && !k.g { px += b.sp; }
                let p = eo(sg(lt, i * 0.14, i * 0.14 + 0.4)); i += 1.0;
                s.tx(&k.w, px, by + (1.0 - p) * 14.0 * u, b.s, if k.h { o.hl } else { o.color }, TO { fam: b.fam, w: b.wt, ls: b.ls, al: Al::Left, a: p * oa });
                px += k.tw;
            }
        }
        return;
    }
    let (ina, bs, lh) = (o.in_a.clone(), b.s, 0.12f32);
    s.lines(&b, x, y, o.color, o.hl, |i| {
        let p = if ina == "none" { 1.0 } else { eo(sg(lt, i as f32 * lh, i as f32 * lh + 0.6)) };
        if ina == "rise" { (oa, (1.0 - p) * bs * 1.05) } else { (p * oa, 0.0) }
    }, center);
}
