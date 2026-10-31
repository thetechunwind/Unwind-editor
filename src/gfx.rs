//! Drawing engine: a small canvas-style API on top of tiny-skia, plus the text layout used by every animation.
use ab_glyph::{Font, FontRef, OutlineCurve, VariableFont};
use std::collections::HashMap;
use tiny_skia::*;

pub fn cl(v: f32) -> f32 { v.clamp(0.0, 1.0) }
pub fn sg(t: f32, a: f32, b: f32) -> f32 { cl((t - a) / (b - a)) }
pub fn eo(p: f32) -> f32 { 1.0 - (1.0 - p).powi(3) }
pub fn eio(p: f32) -> f32 { if p < 0.5 { 4.0 * p * p * p } else { 1.0 - (-2.0 * p + 2.0).powi(3) / 2.0 } }
pub fn ebk(p: f32) -> f32 { let c = 1.70158; 1.0 + (c + 1.0) * (p - 1.0).powi(3) + c * (p - 1.0).powi(2) }

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Col(pub u8, pub u8, pub u8);
pub const LIME: Col = Col(0xC8, 0xF2, 0x3A);
pub const GREEN: Col = Col(0x2F, 0x8A, 0x63);
pub const MOSS: Col = Col(0x1E, 0x60, 0x46);
pub const FOREST: Col = Col(0x13, 0x44, 0x30);
pub const DEEP: Col = Col(0x0D, 0x2B, 0x1D);
pub const INK: Col = Col(0x07, 0x17, 0x0F);
pub const PAPER: Col = Col(0xF2, 0xF5, 0xEA);
pub const SMALL: Col = Col(0x9B, 0xAD, 0x9C);

impl Col {
    pub fn parse(s: &str) -> Option<Col> {
        let s = s.trim().strip_prefix('#')?;
        if s.len() != 6 { return None; }
        let p = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
        Some(Col(p(0)?, p(2)?, p(4)?))
    }
    pub fn hex(&self) -> String { format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2) }
    pub fn lum(&self) -> f32 { (0.299 * self.0 as f32 + 0.587 * self.1 as f32 + 0.114 * self.2 as f32) / 255.0 }
    fn color(&self, a: f32) -> Color { Color::from_rgba8(self.0, self.1, self.2, (cl(a) * 255.0).round() as u8) }
}

#[derive(Clone, Debug, Default)]
pub struct Params {
    pub tpl: String, pub tag: String, pub num: String, pub head: String, pub sub: String, pub data: String,
    pub col: HashMap<String, Col>, pub bgm: String,
}
#[derive(Clone, Copy)]
pub struct Theme { pub bg: Col, pub head: Col, pub hl: Col, pub sm: Col }
impl Params {
    pub fn theme(&self) -> Theme {
        let g = |k: &str, d: Col| *self.col.get(k).unwrap_or(&d);
        Theme { bg: g("bg", DEEP), head: g("head", PAPER), hl: g("hl", LIME), sm: g("sm", SMALL) }
    }
    /// Rows of a "a | b | c" data field (empty first cells are dropped), like the web editor's dt().
    pub fn rows(&self) -> Vec<Vec<String>> {
        self.data.split('\n').map(|s| s.split('|').map(|v| v.trim().to_string()).collect::<Vec<_>>()).filter(|r| !r[0].is_empty()).collect()
    }
}
pub fn cell(r: &[String], i: usize) -> &str { r.get(i).map(|s| s.as_str()).unwrap_or("") }

#[derive(Clone, Copy, PartialEq)]
pub enum Al { Left, Center, Right }
#[derive(Clone, Copy)]
pub struct TO { pub fam: char, pub w: u16, pub ls: f32, pub al: Al, pub a: f32 }
impl Default for TO { fn default() -> Self { TO { fam: 'A', w: 800, ls: 0.0, al: Al::Left, a: 1.0 } } }
impl TO {
    pub fn w(mut self, w: u16) -> Self { self.w = w; self }
    pub fn ls(mut self, v: f32) -> Self { self.ls = v; self }
    pub fn al(mut self, v: Al) -> Self { self.al = v; self }
    pub fn a(mut self, v: f32) -> Self { self.a = v; self }
    pub fn fam(mut self, v: char) -> Self { self.fam = v; self }
}

static ARCHIVO: &[u8] = include_bytes!("../assets/Archivo.ttf");
static NEWS: &[u8] = include_bytes!("../assets/Newsreader.ttf");

struct Fonts { map: HashMap<(char, u16), FontRef<'static>>, glyphs: HashMap<(char, u16, char), Option<Path>> }
impl Fonts {
    fn new() -> Self { Fonts { map: HashMap::new(), glyphs: HashMap::new() } }
    fn font(&mut self, fam: char, wt: u16) -> &FontRef<'static> {
        self.map.entry((fam, wt)).or_insert_with(|| {
            if fam == 'N' {
                let mut f = FontRef::try_from_slice(NEWS).expect("font");
                f.set_variation(b"wght", wt as f32);
                f.set_variation(b"opsz", 36.0);
                f
            } else {
                let mut f = FontRef::try_from_slice(ARCHIVO).expect("font");
                f.set_variation(b"wght", wt as f32);
                f.set_variation(b"wdth", 125.0);
                f
            }
        })
    }
    fn advance(&mut self, fam: char, wt: u16, ch: char, size: f32) -> f32 {
        let f = self.font(fam, wt);
        let upe = f.units_per_em().unwrap_or(1000.0);
        f.h_advance_unscaled(f.glyph_id(ch)) * size / upe
    }
    fn glyph(&mut self, fam: char, wt: u16, ch: char) -> Option<Path> {
        if let Some(p) = self.glyphs.get(&(fam, wt, ch)) { return p.clone(); }
        let f = self.font(fam, wt).clone();
        let mut pb = PathBuilder::new();
        let mut any = false;
        if let Some(o) = f.outline(f.glyph_id(ch)) {
            let mut last: Option<(f32, f32)> = None;
            for c in &o.curves {
                let (p0, _) = curve_ends(c);
                if last.map_or(true, |l| (l.0 - p0.0).abs() > 0.01 || (l.1 - p0.1).abs() > 0.01) {
                    if last.is_some() { pb.close(); }
                    pb.move_to(p0.0, p0.1);
                }
                match c {
                    OutlineCurve::Line(_, b) => pb.line_to(b.x, b.y),
                    OutlineCurve::Quad(_, b, c2) => pb.quad_to(b.x, b.y, c2.x, c2.y),
                    OutlineCurve::Cubic(_, b, c2, d) => pb.cubic_to(b.x, b.y, c2.x, c2.y, d.x, d.y),
                }
                last = Some(curve_ends(c).1);
                any = true;
            }
            if any { pb.close(); }
        }
        let p = if any { pb.finish() } else { None };
        self.glyphs.insert((fam, wt, ch), p.clone());
        p
    }
}
fn curve_ends(c: &OutlineCurve) -> ((f32, f32), (f32, f32)) {
    match c {
        OutlineCurve::Line(a, b) => ((a.x, a.y), (b.x, b.y)),
        OutlineCurve::Quad(a, _, c2) => ((a.x, a.y), (c2.x, c2.y)),
        OutlineCurve::Cubic(a, _, _, d) => ((a.x, a.y), (d.x, d.y)),
    }
}

#[derive(Clone)]
struct St { ts: Transform, alpha: f32, clip: Option<Mask> }

pub struct Cv {
    pub pm: Pixmap, pub w: f32, pub h: f32, pub u: f32,
    st: Vec<St>, cur: St, fonts: Fonts,
}

pub struct Item { pub w: String, pub h: bool, pub g: bool, pub tw: f32 }
pub struct Block { pub l: Vec<Vec<Item>>, pub s: f32, pub lh: f32, pub sp: f32, pub fam: char, pub wt: u16, pub ls: f32, pub th: f32 }
#[derive(Clone, Copy)]
pub struct BO { pub w: f32, pub h: f32, pub s0: f32, pub s1: f32, pub fam: char, pub wt: u16, pub lh: f32, pub sp: f32, pub up: bool }

impl Cv {
    pub fn new(w: u32, h: u32) -> Cv {
        Cv { pm: Pixmap::new(w, h).unwrap(), w: w as f32, h: h as f32, u: (w.min(h) as f32) / 1080.0,
             st: vec![], cur: St { ts: Transform::identity(), alpha: 1.0, clip: None }, fonts: Fonts::new() }
    }
    pub fn bo(&self, w: f32, h: f32, s0: f32) -> BO { BO { w, h, s0, s1: 34.0 * self.u, fam: 'A', wt: 900, lh: 0.98, sp: -0.035, up: true } }
    pub fn clear(&mut self) { self.pm.fill(Color::TRANSPARENT); }
    pub fn save(&mut self) { self.st.push(self.cur.clone()); }
    pub fn restore(&mut self) { if let Some(s) = self.st.pop() { self.cur = s; } }
    pub fn alpha(&mut self, a: f32) { self.cur.alpha = a; }
    pub fn translate(&mut self, x: f32, y: f32) { self.cur.ts = self.cur.ts.pre_translate(x, y); }
    pub fn scale(&mut self, x: f32, y: f32) { self.cur.ts = self.cur.ts.pre_scale(x, y); }
    pub fn rotate(&mut self, deg: f32) { self.cur.ts = self.cur.ts.pre_rotate(deg); }

    fn paint(&self, c: Col, a: f32) -> Paint<'static> {
        let mut p = Paint::default();
        p.set_color(c.color(a * self.cur.alpha));
        p.anti_alias = true;
        p
    }
    pub fn fill_path(&mut self, path: &Path, c: Col) {
        let p = self.paint(c, 1.0);
        self.pm.fill_path(path, &p, FillRule::Winding, self.cur.ts, self.cur.clip.as_ref());
    }
    pub fn fill_path_a(&mut self, path: &Path, c: Col, a: f32) {
        let p = self.paint(c, a);
        self.pm.fill_path(path, &p, FillRule::Winding, self.cur.ts, self.cur.clip.as_ref());
    }
    pub fn stroke_path(&mut self, path: &Path, c: Col, a: f32, width: f32, cap: LineCap, join: LineJoin, dash: Option<(Vec<f32>, f32)>) {
        let p = self.paint(c, a);
        let mut s = Stroke { width, line_cap: cap, line_join: join, ..Stroke::default() };
        if let Some((arr, ph)) = dash {
            if arr.iter().all(|v| *v >= 0.0) && arr.iter().sum::<f32>() > 0.0 {
                let tot: f32 = arr.iter().sum();
                s.dash = StrokeDash::new(arr, ph.rem_euclid(tot));
            }
        }
        self.pm.stroke_path(path, &p, &s, self.cur.ts, self.cur.clip.as_ref());
    }
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Col) {
        if w <= 0.0 || h <= 0.0 { return; }
        if let Some(r) = Rect::from_xywh(x, y, w, h) { let p = PathBuilder::from_rect(r); self.fill_path(&p, c); }
    }
    pub fn fill_rect_a(&mut self, x: f32, y: f32, w: f32, h: f32, c: Col, a: f32) {
        if w <= 0.0 || h <= 0.0 { return; }
        if let Some(r) = Rect::from_xywh(x, y, w, h) { let p = PathBuilder::from_rect(r); self.fill_path_a(&p, c, a); }
    }
    pub fn rrect_path(x: f32, y: f32, w: f32, h: f32, r: [f32; 4]) -> Option<Path> {
        let m = (w.min(h) / 2.0).max(0.0);
        let r = [r[0].min(m), r[1].min(m), r[2].min(m), r[3].min(m)];
        let k = 0.5522847;
        let mut pb = PathBuilder::new();
        pb.move_to(x + r[0], y);
        pb.line_to(x + w - r[1], y);
        pb.cubic_to(x + w - r[1] * (1.0 - k), y, x + w, y + r[1] * (1.0 - k), x + w, y + r[1]);
        pb.line_to(x + w, y + h - r[2]);
        pb.cubic_to(x + w, y + h - r[2] * (1.0 - k), x + w - r[2] * (1.0 - k), y + h, x + w - r[2], y + h);
        pb.line_to(x + r[3], y + h);
        pb.cubic_to(x + r[3] * (1.0 - k), y + h, x, y + h - r[3] * (1.0 - k), x, y + h - r[3]);
        pb.line_to(x, y + r[0]);
        pb.cubic_to(x, y + r[0] * (1.0 - k), x + r[0] * (1.0 - k), y, x + r[0], y);
        pb.close();
        pb.finish()
    }
    pub fn fill_rrect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Col) {
        if w <= 0.0 || h <= 0.0 { return; }
        if let Some(p) = Cv::rrect_path(x, y, w, h, [r; 4]) { self.fill_path(&p, c); }
    }
    pub fn fill_rrect4(&mut self, x: f32, y: f32, w: f32, h: f32, r: [f32; 4], c: Col, a: f32) {
        if w <= 0.0 || h <= 0.0 { return; }
        if let Some(p) = Cv::rrect_path(x, y, w, h, r) { self.fill_path_a(&p, c, a); }
    }
    pub fn fill_circle(&mut self, cx: f32, cy: f32, r: f32, c: Col) {
        if r <= 0.0 { return; }
        if let Some(p) = PathBuilder::from_circle(cx, cy, r) { self.fill_path(&p, c); }
    }
    pub fn stroke_circle(&mut self, cx: f32, cy: f32, r: f32, c: Col, a: f32, w: f32, dash: Option<(Vec<f32>, f32)>) {
        if r <= 0.0 { return; }
        if let Some(p) = PathBuilder::from_circle(cx, cy, r) { self.stroke_path(&p, c, a, w, LineCap::Butt, LineJoin::Miter, dash); }
    }
    pub fn stroke_arc(&mut self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32, c: Col, a: f32, w: f32, cap: LineCap) {
        if r <= 0.0 || (a1 - a0).abs() < 1e-4 { return; }
        let n = (((a1 - a0).abs() / (std::f32::consts::PI / 48.0)).ceil() as usize).max(2);
        let mut pb = PathBuilder::new();
        for i in 0..=n {
            let t = a0 + (a1 - a0) * i as f32 / n as f32;
            let (x, y) = (cx + r * t.cos(), cy + r * t.sin());
            if i == 0 { pb.move_to(x, y) } else { pb.line_to(x, y) }
        }
        if let Some(p) = pb.finish() { self.stroke_path(&p, c, a, w, cap, LineJoin::Round, None); }
    }
    pub fn clip_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let r = Rect::from_xywh(x, y, w.max(0.01), h.max(0.01)).unwrap_or(Rect::from_xywh(0.0, 0.0, 0.01, 0.01).unwrap());
        let path = PathBuilder::from_rect(r);
        match self.cur.clip.as_mut() {
            Some(m) => m.intersect_path(&path, FillRule::Winding, true, self.cur.ts),
            None => {
                let mut m = Mask::new(self.pm.width(), self.pm.height()).unwrap();
                m.fill_path(&path, FillRule::Winding, true, self.cur.ts);
                self.cur.clip = Some(m);
            }
        }
    }
    /// Soft green glow behind the content (the brand backdrop).
    pub fn glow(&mut self, cx: f32, cy: f32, r: f32, c: Col, a0: f32) {
        let stops = vec![GradientStop::new(0.0, c.color(a0)), GradientStop::new(1.0, c.color(0.0))];
        if let Some(sh) = RadialGradient::new(Point::from_xy(cx, cy), Point::from_xy(cx, cy), r, stops, SpreadMode::Pad, Transform::identity()) {
            let mut p = Paint::default();
            p.shader = sh;
            if let Some(rc) = Rect::from_xywh(0.0, 0.0, self.w, self.h) {
                self.pm.fill_rect(rc, &p, self.cur.ts, self.cur.clip.as_ref());
            }
        }
    }
    pub fn fill_image(&mut self, path: &Path, img: &Pixmap, local: Transform, opacity: f32) {
        let mut p = Paint::default();
        p.shader = Pattern::new(img.as_ref(), SpreadMode::Pad, FilterQuality::Bicubic, cl(opacity * self.cur.alpha), local);
        p.anti_alias = true;
        self.pm.fill_path(path, &p, FillRule::Winding, self.cur.ts, self.cur.clip.as_ref());
    }
    /// Draw another pixmap with the current transform, applying alpha.
    pub fn draw_pixmap(&mut self, pm: &Pixmap, x: f32, y: f32, alpha: f32) {
        let pp = PixmapPaint { opacity: cl(alpha * self.cur.alpha), blend_mode: BlendMode::SourceOver, quality: FilterQuality::Bilinear };
        self.pm.draw_pixmap(0, 0, pm.as_ref(), &pp, self.cur.ts.pre_translate(x, y), self.cur.clip.as_ref());
    }

    // ---------- text ----------
    pub fn measure(&mut self, s: &str, fam: char, wt: u16, size: f32, ls: f32) -> f32 {
        s.chars().map(|c| self.fonts.advance(fam, wt, c, size) + ls).sum()
    }
    fn raw_text(&mut self, s: &str, x: f32, y: f32, size: f32, col: Col, fam: char, wt: u16, ls: f32) {
        let upe = { let f = self.fonts.font(fam, wt); f.units_per_em().unwrap_or(1000.0) };
        let k = size / upe;
        let mut pen = x;
        let paint = self.paint(col, 1.0);
        for ch in s.chars() {
            let adv = self.fonts.advance(fam, wt, ch, size);
            if !ch.is_whitespace() {
                if let Some(p) = self.fonts.glyph(fam, wt, ch) {
                    let local = Transform::from_row(k, 0.0, 0.0, -k, pen, y);
                    self.pm.fill_path(&p, &paint, FillRule::Winding, self.cur.ts.pre_concat(local), self.cur.clip.as_ref());
                }
            }
            pen += adv + ls;
        }
    }
    /// Like canvas fillText with an absolute alpha that resets to 1 afterwards.
    pub fn tx(&mut self, s: &str, x: f32, y: f32, size: f32, col: Col, o: TO) {
        if s.is_empty() { return; }
        let w = self.measure(s, o.fam, o.w, size, o.ls);
        let x0 = match o.al { Al::Left => x, Al::Center => x - w / 2.0, Al::Right => x - w };
        self.cur.alpha = o.a;
        self.raw_text(s, x0, y, size, col, o.fam, o.w, o.ls);
        self.cur.alpha = 1.0;
    }
    pub fn fz(&mut self, s: &str, z: f32, mw: f32, wt: u16) -> f32 {
        let w = self.measure(s, 'A', wt, z, 0.0);
        if w > mw { z * mw / w } else { z }
    }

    fn toks(t: &str, up: bool) -> Vec<(String, bool, bool)> {
        let mut o = vec![];
        let (mut h, mut pw) = (false, true);
        for p in t.split('*') {
            let g = !pw && !p.is_empty() && !p.chars().next().unwrap().is_whitespace();
            for (j, w) in p.split_whitespace().enumerate() {
                o.push((if up { w.to_uppercase() } else { w.to_string() }, h, j == 0 && g));
            }
            if !p.is_empty() { pw = p.chars().last().unwrap().is_whitespace(); }
            h = !h;
        }
        o
    }
    fn wrap(&mut self, toks: Vec<(String, bool, bool)>, max_w: f32, fam: char, wt: u16, s: f32, ls: f32) -> Vec<Vec<Item>> {
        let sp = self.measure(" ", fam, wt, s, ls);
        let (mut lines, mut cur, mut w) = (vec![], Vec::<Item>::new(), 0.0f32);
        for (word, h, g) in toks {
            let tw = self.measure(&word, fam, wt, s, ls);
            let mut gap = if !cur.is_empty() && !g { sp } else { 0.0 };
            if !cur.is_empty() && !g && w + gap + tw > max_w {
                lines.push(std::mem::take(&mut cur));
                w = 0.0; gap = 0.0;
            }
            w += gap + tw;
            cur.push(Item { w: word, h, g, tw });
        }
        if !cur.is_empty() { lines.push(cur); }
        lines
    }
    pub fn block(&mut self, text: &str, o: BO) -> Block {
        let toks = Cv::toks(text, o.up);
        let mut s = o.s0;
        let mut l = vec![];
        while s >= o.s1 {
            l = self.wrap(toks.clone(), o.w, o.fam, o.wt, s, o.sp * s);
            if (l.len() as f32) * s * o.lh <= o.h { break; }
            s -= 2.0;
        }
        s = s.max(o.s1);
        if l.is_empty() { l = self.wrap(toks, o.w, o.fam, o.wt, s, o.sp * s); }
        let sp = self.measure(" ", o.fam, o.wt, s, o.sp * s);
        let th = l.len() as f32 * s * o.lh;
        Block { l, s, lh: o.lh, sp, fam: o.fam, wt: o.wt, ls: o.sp * s, th }
    }
    /// Draw the lines of a block. `f(i)` returns (alpha, dy) for line i. `center` aligns around X.
    pub fn lines(&mut self, b: &Block, x: f32, y: f32, col: Col, hc: Col, mut f: impl FnMut(usize) -> (f32, f32), center: bool) {
        for (li, l) in b.l.iter().enumerate() {
            let (a, dy) = f(li);
            if a <= 0.0 { continue; }
            let ws: Vec<f32> = l.iter().map(|k| k.tw).collect();
            let spc = b.sp;
            let mut tw = 0.0;
            for (j, k) in l.iter().enumerate() { tw += ws[j] + if j > 0 && !k.g { spc } else { 0.0 }; }
            let mx = self.w - 2.0 * 40.0 * self.u;
            let sc = (mx / tw).min(1.0);
            let top = y + li as f32 * b.s * b.lh;
            let px = if center { x - tw * sc / 2.0 } else { x };
            self.save();
            self.clip_rect(px - 40.0, top, tw * sc + 80.0, b.s * b.lh * 1.02);
            self.translate(px, top + b.s * 0.82 + dy);
            self.scale(sc, sc);
            self.cur.alpha = a;
            let mut cx = 0.0;
            for (j, k) in l.iter().enumerate() {
                if j > 0 && !k.g { cx += spc; }
                self.raw_text(&k.w, cx, 0.0, b.s, if k.h { hc } else { col }, b.fam, b.wt, b.ls);
                cx += ws[j];
            }
            self.restore();
        }
    }

    // ---------- brand pieces ----------
    pub fn mark(&mut self, x: f32, y: f32, z: f32, bg: Option<Col>, fg: Col, p: f32, a: Option<f32>) {
        let k = z / 100.0;
        self.save();
        self.cur.alpha = a.unwrap_or(1.0);
        self.translate(x, y - 2.0 * k);
        self.scale(k, k);
        if let Some(b) = bg {
            let mut pb = PathBuilder::new();
            pb.move_to(0.0, 24.0); pb.cubic_to(0.0, 11.86, 9.86, 2.0, 22.0, 2.0); pb.line_to(78.0, 2.0);
            pb.cubic_to(90.14, 2.0, 100.0, 11.86, 100.0, 24.0); pb.line_to(100.0, 80.0);
            pb.cubic_to(100.0, 92.14, 90.14, 102.0, 78.0, 102.0); pb.line_to(22.0, 102.0);
            pb.cubic_to(9.86, 102.0, 0.0, 92.14, 0.0, 80.0); pb.close();
            if let Some(path) = pb.finish() { self.fill_path(&path, b); }
        }
        let mut pb = PathBuilder::new();
        pb.move_to(49.0, 26.0); pb.line_to(31.0, 26.0); pb.line_to(31.0, 58.0);
        pb.cubic_to(31.0, 68.49, 39.51, 77.0, 50.0, 77.0); pb.cubic_to(60.49, 77.0, 69.0, 68.49, 69.0, 58.0); pb.line_to(69.0, 46.0);
        if let Some(ud) = pb.finish() {
            if 190.0 * p > 0.0 { self.stroke_path(&ud, fg, 1.0, 13.0, LineCap::Butt, LineJoin::Miter, Some((vec![190.0 * p, 400.0], 0.0))); }
        }
        let mut pb = PathBuilder::new();
        pb.move_to(31.0, 26.0); pb.line_to(13.0, 26.0);
        if let Some(td) = pb.finish() {
            let d = 20.0 * cl(p * 3.0 - 2.0);
            if d > 0.0 { self.stroke_path(&td, fg, 1.0, 13.0, LineCap::Butt, LineJoin::Miter, Some((vec![d, 400.0], 0.0))); }
        }
        if p > 0.9 { self.fill_circle(69.0, 26.0, 8.5 * ebk(sg(p, 0.9, 1.0)), fg); }
        self.restore();
    }
    pub fn logo_tl(&mut self, pr: &Params, a: f32) {
        let t = pr.theme();
        let dark = pr.bgm != "brand" || t.bg.lum() <= 0.55;
        let u = self.u;
        self.mark(64.0 * u, 64.0 * u, 72.0 * u, Some(if dark { t.hl } else { DEEP }), if dark { INK } else { t.hl }, 1.0, Some(a));
    }
    pub fn deco(&mut self, pr: &Params, cx: Option<f32>, cy: Option<f32>, rr: f32, g: Option<(f32, f32)>) {
        if pr.bgm != "brand" { return; }
        let t = pr.theme();
        let r = self.w.max(self.h) * 0.62;
        let cx = cx.unwrap_or(self.w / 2.0);
        let cy = cy.unwrap_or(self.h * 0.42);
        let (gx, gy) = g.unwrap_or((cx, cy));
        self.glow(gx, gy, r, GREEN, 0.5);
        if rr > 0.0 { let u = self.u; self.stroke_circle(cx, cy, rr, t.hl, 0.3, 6.0 * u, None); }
    }
    pub fn pill(&mut self, s: &str, x: f32, y: f32, h: f32, f: Col, fg: Col, p: f32, a: f32) {
        if s.is_empty() { return; }
        let sz = h * 0.44;
        let w = self.measure(s, 'A', 800, sz, 0.0) + sz * 2.1;
        self.save();
        self.cur.alpha = a.max(0.0);
        self.translate(x, y + h / 2.0);
        self.scale(p.max(0.0), p.max(0.0));
        self.translate(0.0, -h / 2.0);
        self.fill_rrect(0.0, 0.0, w, h, h / 2.0, f);
        self.raw_text(s, sz * 1.05, h / 2.0 + sz * 0.35, sz, fg, 'A', 800, 0.0);
        self.restore();
    }
    pub fn stamp_rings(&mut self, cx: f32, cy: f32, r: f32, t: f32, a: f32, col: Col, e: f32) {
        let u = self.u;
        self.stroke_arc(cx, cy, r * 1.3, -std::f32::consts::FRAC_PI_2, -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * 2.0 * eo(sg(t, 0.6, 1.7)), col, 0.45 * a * self.cur.alpha, 5.0 * u, LineCap::Round);
        self.stroke_circle(cx, cy, r * 1.12 * e.max(0.0), col, 0.6 * a * self.cur.alpha, 5.0 * u, Some((vec![14.0 * u, 20.0 * u], -t * 26.0 * u)));
    }
}
