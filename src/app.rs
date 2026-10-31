use crate::gfx::{Al, Col, DEEP, FOREST, INK, LIME, MOSS, PAPER};
use crate::{anim, assets, audio, export, model::*, spec};
use eframe::egui::{self, *};
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

const GUT: f32 = 92.0;
const RUL: f32 = 26.0;
const LH: f32 = 54.0;
const LANES: [&str; 4] = ["Layer 3 (top)", "Layer 2", "Layer 1 (base)", "Audio"];

#[derive(Clone, Copy)]
enum Drag { Seek, Move { id: u32, grab: f32, orig_start: f32, orig_track: usize }, ResizeL { id: u32, orig_start: f32, orig_dur: f32, orig_trim: f32 }, ResizeR { id: u32 } }
struct Loaded { at: Option<(f32, usize)>, res: Result<Arc<Asset>, String> }

pub struct App {
    prj: Project, hist: Vec<Project>, redo: Vec<Project>, last_hist: Instant, rev: u32,
    rend: Renderer, tex: Option<TextureHandle>, key: (u32, i64, (u32, u32)),
    ph: f32, playing: bool, last_tick: Instant,
    sel: Option<u32>, pps: f32, drag: Option<Drag>, drag_bad: bool, tl_org: Pos2, tl_rect: Rect,
    tpl_idx: usize,
    code: String, code_live: bool, code_dirty: bool, code_edit: Option<Instant>, code_status: String, code_err: bool, code_open: bool, code_rev: u32,
    msg: String,
    player: Option<audio::Player>, mix: Option<(u32, Arc<Vec<f32>>)>,
    tx: mpsc::Sender<Loaded>, rx: mpsc::Receiver<Loaded>, loading: u32,
    export: Option<export::Job>, export_msg: Option<String>, export_open: bool, export_png: bool,
}

fn style(ctx: &Context) {
    let mut v = Visuals::dark();
    let c = |c: Col| Color32::from_rgb(c.0, c.1, c.2);
    v.panel_fill = Color32::from_rgb(0x0a, 0x1f, 0x15);
    v.window_fill = Color32::from_rgb(0x0d, 0x2b, 0x1d);
    v.extreme_bg_color = Color32::from_rgb(0x06, 0x14, 0x0d);
    v.selection.bg_fill = c(MOSS);
    v.selection.stroke = Stroke::new(1.0, c(LIME));
    v.hyperlink_color = c(LIME);
    v.widgets.inactive.bg_fill = Color32::from_rgb(0x13, 0x44, 0x30);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0x13, 0x44, 0x30);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x1e, 0x60, 0x46);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(0x2f, 0x8a, 0x63);
    ctx.set_visuals(v);
    ctx.style_mut(|s| { s.spacing.item_spacing = vec2(8.0, 7.0); s.spacing.button_padding = vec2(10.0, 5.0); });
}
fn c32(c: Col) -> Color32 { Color32::from_rgb(c.0, c.1, c.2) }
fn mmss(t: f32) -> String { let t = t.max(0.0); format!("{:02}:{:05.2}", (t / 60.0) as u32, t % 60.0) }

impl App {
    pub fn new(cc: &eframe::CreationContext) -> App {
        style(&cc.egui_ctx);
        let (tx, rx) = mpsc::channel();
        let prj = Project::new();
        App {
            code: spec::export(&prj), prj, hist: vec![], redo: vec![], last_hist: Instant::now(), rev: 1,
            rend: Renderer::new(960, 540), tex: None, key: (0, -1, (0, 0)),
            ph: 0.0, playing: false, last_tick: Instant::now(),
            sel: None, pps: 60.0, drag: None, drag_bad: false, tl_org: Pos2::ZERO, tl_rect: Rect::NOTHING,
            tpl_idx: 0, code_live: true, code_dirty: false, code_edit: None, code_status: String::new(), code_err: false, code_open: false, code_rev: 1,
            msg: String::new(), player: audio::Player::new(), mix: None, tx, rx, loading: 0,
            export: None, export_msg: None, export_open: false, export_png: false,
        }
    }

    // ---------- state helpers ----------
    fn changed(&mut self) { self.rev += 1; }
    fn snapshot(&mut self) { self.hist.push(self.prj.clone()); if self.hist.len() > 60 { self.hist.remove(0); } self.redo.clear(); self.last_hist = Instant::now(); }
    fn snapshot_soft(&mut self) { if self.last_hist.elapsed() > Duration::from_millis(700) { self.snapshot(); } else { self.last_hist = Instant::now(); } }
    fn undo(&mut self) { if let Some(p) = self.hist.pop() { self.redo.push(std::mem::replace(&mut self.prj, p)); self.sel = None; self.changed(); } }
    fn redo(&mut self) { if let Some(p) = self.redo.pop() { self.hist.push(std::mem::replace(&mut self.prj, p)); self.sel = None; self.changed(); } }
    fn mix(&mut self) -> Arc<Vec<f32>> {
        if let Some((r, m)) = &self.mix { if *r == self.rev { return m.clone(); } }
        let m = Arc::new(audio::mixdown(&self.prj)); self.mix = Some((self.rev, m.clone())); m
    }
    fn set_play(&mut self, on: bool) {
        if on && self.ph >= self.prj.duration() - 0.01 { self.ph = 0.0; }
        self.playing = on; self.last_tick = Instant::now();
        if on { if audio::has_audio(&self.prj) { let m = self.mix(); let ph = self.ph; if let Some(p) = self.player.as_mut() { p.play(&m, ph); } } }
        else if let Some(p) = self.player.as_mut() { p.stop(); }
    }
    fn seek(&mut self, t: f32) { self.ph = t.clamp(0.0, self.prj.duration().max(0.0)); if self.playing { self.set_play(true); } }

    fn add_anim(&mut self) {
        self.snapshot();
        let mut c = Clip::new_anim(anim::TPLS[self.tpl_idx].id);
        c.start = self.ph;
        let pref: Vec<usize> = if c.track == 1 { vec![1, 2, 0] } else { vec![2, 1, 0] };
        self.sel = Some(self.prj.add(c, &pref)); self.changed();
    }
    fn add_text(&mut self) { self.snapshot(); let mut c = Clip::new_text(); c.start = self.ph; self.sel = Some(self.prj.add(c, &[0, 1, 2])); self.changed(); }
    fn load_paths(&mut self, paths: Vec<PathBuf>, at: Option<(f32, usize)>) {
        for p in paths {
            let tx = self.tx.clone(); self.loading += 1;
            std::thread::spawn(move || { let _ = tx.send(Loaded { at, res: assets::load(&p) }); });
        }
    }
    fn receive(&mut self) {
        while let Ok(l) = self.rx.try_recv() {
            self.loading = self.loading.saturating_sub(1);
            match l.res {
                Err(e) => self.msg = e,
                Ok(a) => {
                    self.snapshot();
                    self.prj.assets.push(a.clone());
                    let start = l.at.map(|x| x.0).unwrap_or(self.ph);
                    let mut c;
                    let pref: Vec<usize>;
                    match &a.data {
                        AssetData::Image(pm) => {
                            c = Clip::blank(Kind::Image);
                            let (nw, nh) = (pm.width() as f32, pm.height() as f32);
                            c.w = (0.6 * self.prj.h as f32 * (nw / nh) / self.prj.w as f32 * 100.0).clamp(12.0, 70.0).round();
                            c.dur = 4.0;
                            pref = match l.at { Some((_, lane)) if lane < 3 => { let mut v = vec![lane]; v.extend([1, 0, 2].iter().filter(|x| **x != lane)); v } _ => vec![1, 0, 2] };
                        }
                        AssetData::Audio { .. } => { c = Clip::blank(Kind::Audio); c.dur = a.duration(); c.track = 3; pref = vec![3]; }
                    }
                    c.name = a.name.clone(); c.file = a.name.clone(); c.asset = Some(a); c.start = start;
                    self.sel = Some(self.prj.add(c, &pref)); self.changed(); self.msg.clear();
                }
            }
        }
    }
    fn split(&mut self) {
        let Some(id) = self.sel else { return };
        let Some(c) = self.prj.get(id).cloned() else { return };
        let lt = self.ph - c.start;
        if lt < 0.1 || lt > c.dur - 0.1 { self.msg = "Move the playhead inside the clip to split it.".into(); return; }
        self.snapshot();
        let mut b = c.clone(); b.id = self.prj.nid; self.prj.nid += 1; b.start = self.ph; b.dur = c.dur - lt; b.trim = c.trim + lt;
        for k in b.kf.iter_mut() { for p in k.iter_mut() { p.0 -= lt; } k.retain(|p| p.0 >= 0.0); }
        if let Some(a) = self.prj.get_mut(id) { a.dur = lt; for k in a.kf.iter_mut() { k.retain(|p| p.0 <= lt); } }
        self.sel = Some(b.id); self.prj.clips.push(b); self.changed();
    }
    fn duplicate(&mut self) {
        let Some(id) = self.sel else { return };
        let Some(mut c) = self.prj.get(id).cloned() else { return };
        self.snapshot(); c.start = c.end(); let tr = c.track; self.sel = Some(self.prj.add(c, &[tr])); self.changed();
    }
    fn delete(&mut self) { if let Some(id) = self.sel.take() { self.snapshot(); self.prj.clips.retain(|c| c.id != id); self.changed(); } }

    // ---------- code ----------
    fn sync_code(&mut self, force: bool) {
        if force || (!self.code_dirty) { self.code = spec::export(&self.prj); self.code_dirty = false; self.code_rev = self.rev; if force { self.code_status = "The code now matches the timeline.".into(); self.code_err = false; } }
    }
    fn generate(&mut self) {
        let res = spec::parse(&self.code).and_then(|v| spec::compile(&self.prj, &v));
        match res {
            Err(e) => { self.code_status = e.text(); self.code_err = true; }
            Ok(c) => {
                self.snapshot_soft_code();
                self.prj.assets.extend(c.assets);
                self.prj.clips = c.clips; self.prj.nid = c.nid; self.sel = None;
                if let Some((w, h)) = c.canvas { self.prj.w = w; self.prj.h = h; }
                if let Some(f) = c.fps { self.prj.fps = f; }
                if let Some(b) = c.bg { self.prj.bg = b; }
                self.ph = self.ph.min(self.prj.duration());
                let mut m = format!("Generated {} clip{}, {:.1} seconds.", self.prj.clips.len(), if self.prj.clips.len() == 1 { "" } else { "s" }, self.prj.duration());
                if !c.warn.is_empty() { m += &format!("\n{}", c.warn.iter().take(3).cloned().collect::<Vec<_>>().join("\n")); if c.warn.len() > 3 { m += &format!("\n+ {} more notes", c.warn.len() - 3); } }
                self.code_status = m; self.code_err = false; self.code_dirty = false; self.changed(); self.code_rev = self.rev;
            }
        }
    }
    fn snapshot_soft_code(&mut self) { if self.last_hist.elapsed() > Duration::from_secs(2) || self.hist.is_empty() { self.snapshot(); } else { self.last_hist = Instant::now(); } }

    // ---------- preview ----------
    fn render_preview(&mut self, ctx: &Context) {
        let sc = (1000.0 / self.prj.w.max(self.prj.h) as f32).min(1.0);
        let (pw, ph) = (((self.prj.w as f32 * sc).round() as u32).max(2), ((self.prj.h as f32 * sc).round() as u32).max(2));
        let key = (self.rev, (self.ph * 1000.0) as i64, (pw, ph));
        if key == self.key && self.tex.is_some() { return; }
        if self.rend.size() != (pw, ph) { self.rend = Renderer::new(pw, ph); }
        let pm = self.rend.render(&self.prj, self.ph);
        let img = ColorImage::from_rgba_premultiplied([pw as usize, ph as usize], pm.data());
        match &mut self.tex { Some(t) => t.set(img, TextureOptions::LINEAR), None => self.tex = Some(ctx.load_texture("preview", img, TextureOptions::LINEAR)) }
        self.key = key;
    }

    // ---------- panels ----------
    fn left(&mut self, ui: &mut Ui) {
        ui.heading("Add to the timeline");
        ComboBox::from_id_salt("tpl").width(ui.available_width()).selected_text(anim::TPLS[self.tpl_idx].name).show_ui(ui, |ui| {
            for (i, t) in anim::TPLS.iter().enumerate() { ui.selectable_value(&mut self.tpl_idx, i, t.name); }
        });
        if ui.add_sized([ui.available_width(), 30.0], Button::new(RichText::new("Add animation").color(c32(INK)).strong()).fill(c32(LIME))).clicked() { self.add_anim(); }
        if ui.add_sized([ui.available_width(), 28.0], Button::new("Add text")).clicked() { self.add_text(); }
        ui.horizontal(|ui| {
            if ui.add_sized([(ui.available_width() - 8.0) / 2.0, 28.0], Button::new("Add images")).clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"]).pick_files() { self.load_paths(p, None); }
            }
            if ui.add_sized([ui.available_width(), 28.0], Button::new("Add audio")).clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("Audio", &["wav", "wave", "mp3", "m4a", "aac", "ogg", "oga", "flac", "aif", "aiff", "aifc", "caf", "mka", "mp4", "mov", "opus", "wma", "amr", "webm"]).add_filter("Any file", &["*"]).pick_files() { self.load_paths(p, None); }
            }
        });
        ui.label(RichText::new("You can also drag images and audio from your computer onto the window, or onto a lane of the timeline.").weak().small());
        if self.loading > 0 { ui.horizontal(|ui| { ui.spinner(); ui.label(format!("Loading {} file{}…", self.loading, if self.loading == 1 { "" } else { "s" })); }); }
        if !self.msg.is_empty() { ui.colored_label(Color32::from_rgb(255, 176, 176), &self.msg); }
        ui.separator();
        ui.heading("Project");
        let mut ch = false;
        let cur = format!("{}x{}", self.prj.w, self.prj.h);
        ComboBox::from_label("Canvas").selected_text(match cur.as_str() { "1920x1080" => "Landscape 1920×1080", "1080x1920" => "Vertical 1080×1920", "1080x1350" => "Portrait 1080×1350", _ => "Square 1080×1080" }).show_ui(ui, |ui| {
            for (v, l) in [((1920, 1080), "Landscape 1920×1080"), ((1080, 1920), "Vertical 1080×1920"), ((1080, 1350), "Portrait 1080×1350"), ((1080, 1080), "Square 1080×1080")] {
                if ui.selectable_label((self.prj.w, self.prj.h) == v, l).clicked() { self.prj.w = v.0; self.prj.h = v.1; ch = true; }
            }
        });
        ComboBox::from_label("Frame rate").selected_text(format!("{} fps", self.prj.fps)).show_ui(ui, |ui| { for f in [24, 30, 60] { if ui.selectable_label(self.prj.fps == f, format!("{} fps", f)).clicked() { self.prj.fps = f; ch = true; } } });
        ComboBox::from_label("Background").selected_text(&self.prj.bg).show_ui(ui, |ui| { for b in ["deep", "ink", "black", "clear"] { if ui.selectable_label(self.prj.bg == b, b).clicked() { self.prj.bg = b.into(); ch = true; } } });
        if ch { self.changed(); }
    }

    fn props(&mut self, ui: &mut Ui) {
        let Some(id) = self.sel else { ui.heading("Properties"); ui.label("Click a clip on the timeline to edit it."); return };
        let Some(mut c) = self.prj.get(id).cloned() else { return };
        let mut ch = false;
        ui.heading(match c.kind { Kind::Anim => "Animation", Kind::Text => "Text", Kind::Image => "Image", Kind::Audio => "Audio" });
        ui.horizontal(|ui| { ui.label("Name"); ch |= ui.text_edit_singleline(&mut c.name).changed(); });
        Grid::new("timing").num_columns(2).show(ui, |ui| {
            ui.label("Starts"); ch |= ui.add(DragValue::new(&mut c.start).speed(0.05).range(0.0..=3600.0).suffix(" s")).changed(); ui.end_row();
            ui.label("Length"); ch |= ui.add(DragValue::new(&mut c.dur).speed(0.05).range(0.2..=3600.0).suffix(" s")).changed(); ui.end_row();
            if matches!(c.kind, Kind::Anim | Kind::Audio) { ui.label("Trim"); ch |= ui.add(DragValue::new(&mut c.trim).speed(0.05).range(0.0..=3600.0).suffix(" s")).changed(); ui.end_row(); }
            if c.kind != Kind::Audio {
                ui.label("Layer");
                ComboBox::from_id_salt("layer").selected_text(LANES[c.track.min(2)]).show_ui(ui, |ui| { for i in 0..3 { if ui.selectable_label(c.track == i, LANES[i]).clicked() { c.track = i; ch = true; } } });
                ui.end_row();
            }
        });
        ui.separator();
        match c.kind {
            Kind::Anim => {
                let tp = anim::tpl(&c.p.tpl).unwrap_or(&anim::TPLS[0]);
                let fl: Vec<&str> = tp.fields.split(' ').collect();
                ui.label(RichText::new(tp.name).strong());
                for (k, lab) in [("tag", "Tag"), ("num", "Number"), ("head", "Headline (use *stars* to highlight)"), ("sub", "Small text")] {
                    if fl.contains(&k) {
                        ui.label(lab);
                        let s = match k { "tag" => &mut c.p.tag, "num" => &mut c.p.num, "head" => &mut c.p.head, _ => &mut c.p.sub };
                        ch |= if k == "head" { ui.add(TextEdit::multiline(s).desired_rows(2).desired_width(f32::INFINITY)).changed() } else { ui.add(TextEdit::singleline(s).desired_width(f32::INFINITY)).changed() };
                    }
                }
                if fl.contains(&"data") {
                    ui.label("Data"); if !tp.hint.is_empty() { ui.label(RichText::new(tp.hint).weak().small()); }
                    ch |= ui.add(TextEdit::multiline(&mut c.p.data).desired_rows(5).desired_width(f32::INFINITY).code_editor()).changed();
                }
                ui.label("Backdrop");
                ComboBox::from_id_salt("bgm").selected_text(&c.p.bgm).show_ui(ui, |ui| { for b in ["brand", "clear", "chroma"] { if ui.selectable_label(c.p.bgm == b, b).clicked() { c.p.bgm = b.into(); ch = true; } } });
                ui.label("Colours");
                let th = c.p.theme();
                ui.horizontal(|ui| {
                    for (k, lab, cur) in [("bg", "Background", th.bg), ("head", "Text", th.head), ("hl", "Highlight", th.hl), ("sm", "Small text", th.sm)] {
                        let mut rgb = [cur.0, cur.1, cur.2];
                        if ui.color_edit_button_srgb(&mut rgb).on_hover_text(lab).changed() { c.p.col.insert(k.into(), Col(rgb[0], rgb[1], rgb[2])); ch = true; }
                    }
                    if !c.p.col.is_empty() && ui.small_button("Reset").clicked() { c.p.col.clear(); ch = true; }
                });
            }
            Kind::Text => {
                ui.label("Text (use *stars* to highlight)");
                ch |= ui.add(TextEdit::multiline(&mut c.txt.text).desired_rows(3).desired_width(f32::INFINITY)).changed();
                Grid::new("txt").num_columns(2).show(ui, |ui| {
                    ui.label("Style"); ComboBox::from_id_salt("st").selected_text(&c.txt.style).show_ui(ui, |ui| { for s in ["headline", "serif", "label"] { if ui.selectable_label(c.txt.style == s, s).clicked() { c.txt.style = s.into(); ch = true; } } }); ui.end_row();
                    ui.label("Size"); ch |= ui.add(DragValue::new(&mut c.txt.size).speed(0.1).range(1.0..=60.0).suffix(" %")).changed(); ui.end_row();
                    ui.label("Align"); ComboBox::from_id_salt("al").selected_text(&c.txt.align).show_ui(ui, |ui| { for s in ["center", "left"] { if ui.selectable_label(c.txt.align == s, s).clicked() { c.txt.align = s.into(); ch = true; } } }); ui.end_row();
                    ui.label("Comes in"); ComboBox::from_id_salt("in").selected_text(&c.txt.in_a).show_ui(ui, |ui| { for s in ["rise", "fade", "words", "none"] { if ui.selectable_label(c.txt.in_a == s, s).clicked() { c.txt.in_a = s.into(); ch = true; } } }); ui.end_row();
                    ui.label("Goes out"); ComboBox::from_id_salt("out").selected_text(&c.txt.out_a).show_ui(ui, |ui| { for s in ["fade", "none"] { if ui.selectable_label(c.txt.out_a == s, s).clicked() { c.txt.out_a = s.into(); ch = true; } } }); ui.end_row();
                    ui.label("Colours");
                    ui.horizontal(|ui| {
                        for (lab, col) in [("Text", &mut c.txt.color), ("Highlight", &mut c.txt.hl)] { let mut rgb = [col.0, col.1, col.2]; if ui.color_edit_button_srgb(&mut rgb).on_hover_text(lab).changed() { *col = Col(rgb[0], rgb[1], rgb[2]); ch = true; } }
                    }); ui.end_row();
                });
            }
            Kind::Image => {
                Grid::new("img").num_columns(2).show(ui, |ui| {
                    ui.label("Fit"); ComboBox::from_id_salt("fit").selected_text(&c.fit).show_ui(ui, |ui| { for s in ["free", "cover", "contain"] { if ui.selectable_label(c.fit == s, s).clicked() { c.fit = s.into(); ch = true; } } }); ui.end_row();
                    if c.fit == "free" {
                        ui.label("Width"); ch |= ui.add(DragValue::new(&mut c.w).speed(0.5).range(1.0..=300.0).suffix(" %")).changed(); ui.end_row();
                        ui.label("Shape"); ComboBox::from_id_salt("shape").selected_text(&c.shape).show_ui(ui, |ui| { for s in ["none", "round", "circle"] { if ui.selectable_label(c.shape == s, s).clicked() { c.shape = s.into(); ch = true; } } }); ui.end_row();
                        ui.label("Border"); ch |= ui.add(DragValue::new(&mut c.bw).speed(0.2).range(0.0..=60.0)).changed(); ui.end_row();
                        ui.label("Shadow"); ch |= ui.checkbox(&mut c.shadow, "").changed(); ui.end_row();
                    }
                });
            }
            Kind::Audio => {
                Grid::new("aud").num_columns(2).show(ui, |ui| {
                    ui.label("Volume"); ch |= ui.add(Slider::new(&mut c.vol, 0.0..=2.0)).changed(); ui.end_row();
                    ui.label("Mute"); ch |= ui.checkbox(&mut c.mute, "").changed(); ui.end_row();
                    ui.label("Fade in"); ch |= ui.add(DragValue::new(&mut c.fade_in).speed(0.05).range(0.0..=30.0).suffix(" s")).changed(); ui.end_row();
                    ui.label("Fade out"); ch |= ui.add(DragValue::new(&mut c.fade_out).speed(0.05).range(0.0..=30.0).suffix(" s")).changed(); ui.end_row();
                });
            }
        }
        if c.kind != Kind::Audio {
            ui.separator();
            ui.label(RichText::new("Position and motion").strong());
            let lt = (self.ph - c.start).clamp(0.0, c.dur);
            Grid::new("tf").num_columns(3).show(ui, |ui| {
                let names = ["Across", "Down", "Size", "Turn", "Opacity"];
                for i in 0..5 {
                    ui.label(names[i]);
                    let v = match i { 0 => &mut c.tf.x, 1 => &mut c.tf.y, 2 => &mut c.tf.s, 3 => &mut c.tf.r, _ => &mut c.tf.o };
                    let r = match i { 0 | 1 => 0.0..=100.0f32, 2 => 0.05..=4.0, 3 => -360.0..=360.0, _ => 0.0..=1.0 };
                    let lo = if i < 2 { -100.0 } else { *r.start() };
                    ch |= ui.add(Slider::new(v, lo..=*r.end())).changed();
                    let val = *v;
                    ui.horizontal(|ui| {
                        if ui.small_button("Key").on_hover_text("Add a keyframe at the playhead").clicked() { c.add_key(i, lt, val); ch = true; }
                        if !c.kf[i].is_empty() && ui.small_button(format!("Clear {}", c.kf[i].len())).on_hover_text("Clear keyframes").clicked() { c.kf[i].clear(); ch = true; }
                    });
                    ui.end_row();
                }
            });
            ch |= ui.checkbox(&mut c.lin, "Linear movement between keyframes").changed();
        }
        if ch { self.snapshot_soft(); if let Some(slot) = self.prj.get_mut(id) { *slot = c; } self.changed(); }
    }

    fn code_panel(&mut self, ui: &mut Ui) {
        ui.heading("Code");
        ui.label(RichText::new("Paste code to build the video. Edit it here or on the timeline and the other side follows.").weak().small());
        let h = (ui.available_height() - 150.0).max(160.0);
        let r = ScrollArea::both().max_height(h).id_salt("codescroll").show(ui, |ui| {
            ui.add(TextEdit::multiline(&mut self.code).code_editor().desired_width(f32::INFINITY).desired_rows(24).lock_focus(false))
        }).inner;
        if r.changed() {
            self.code_dirty = true; self.code_edit = Some(Instant::now());
            self.code_status = if self.code_live { "Reading your code…".into() } else { "Changed. Press Generate video to apply.".into() }; self.code_err = false;
        }
        if self.code_live && self.code_dirty { if let Some(t) = self.code_edit { if t.elapsed() > Duration::from_millis(900) { self.code_edit = None; self.generate(); } else { ui.ctx().request_repaint_after(Duration::from_millis(200)); } } }
        if !self.code_status.is_empty() { ui.colored_label(if self.code_err { Color32::from_rgb(255, 176, 176) } else { Color32::from_rgb(0xE3, 0xF9, 0xA8) }, &self.code_status); }
        ui.horizontal_wrapped(|ui| {
            if ui.add(Button::new(RichText::new("Generate video").color(c32(INK)).strong()).fill(c32(LIME))).clicked() { self.generate(); }
            if ui.button("Read timeline").clicked() { self.sync_code(true); }
            if ui.add_enabled(!self.hist.is_empty(), Button::new("Undo")).clicked() { self.undo(); self.sync_code(true); }
            ui.checkbox(&mut self.code_live, "Live");
        });
        ui.collapsing("Names and fields", |ui| {
            ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                ui.label("Project: canvas, fps, background, clips");
                ui.label("Every clip: type, start, dur, layer (1 base to 3 top), x, y, scale, rotate, opacity, keys, ease. Leave start out and the clip begins when the one before it ends.");
                ui.label("text: text, style, size, color, highlight, align, in, out");
                ui.label("image: file, fit, width, shape, border, borderColor, shadow");
                ui.label("audio: file, trim, volume, mute, fadeIn, fadeOut");
                ui.label("Files can be a name you added, or a path on your computer.");
                ui.separator();
                for t in anim::TPLS {
                    ui.label(RichText::new(format!("{} · {}", t.id, t.name)).strong());
                    let f = t.fields.split(' ').filter(|x| !x.is_empty()).collect::<Vec<_>>().join(", ");
                    if !f.is_empty() { ui.label(f); }
                    if !t.hint.is_empty() { ui.label(RichText::new(t.hint).weak().small()); }
                }
            });
        });
    }

    // ---------- timeline ----------
    fn snap(&self, t: f32, skip: u32, len: f32) -> f32 {
        let thr = 8.0 / self.pps;
        let mut pts = vec![0.0, self.ph];
        for c in &self.prj.clips { if c.id != skip { pts.push(c.start); pts.push(c.end()); } }
        let mut best = (thr, t);
        for p in &pts {
            if (t - p).abs() < best.0 { best = ((t - p).abs(), *p); }
            if (t + len - p).abs() < best.0 { best = ((t + len - p).abs(), p - len); }
        }
        best.1.max(0.0)
    }
    fn timeline(&mut self, ui: &mut Ui) {
        let dur = (self.prj.duration() + 8.0).max(24.0);
        let width = GUT + dur * self.pps + 160.0;
        ScrollArea::horizontal().id_salt("tlscroll").auto_shrink([false, false]).show(ui, |ui| {
            let (resp, painter) = ui.allocate_painter(vec2(width.max(ui.available_width()), RUL + 4.0 * LH + 8.0), Sense::click_and_drag());
            let org = resp.rect.min; self.tl_org = org; self.tl_rect = resp.rect;
            let pps = self.pps;
            // lanes
            for i in 0..4 {
                let r = Rect::from_min_size(pos2(org.x, org.y + RUL + i as f32 * LH), vec2(resp.rect.width(), LH));
                painter.rect_filled(r, 0.0, if i % 2 == 0 { Color32::from_rgb(0x0c, 0x24, 0x18) } else { Color32::from_rgb(0x0a, 0x1f, 0x15) });
                painter.text(pos2(org.x + 8.0, r.center().y), Align2::LEFT_CENTER, LANES[i], FontId::proportional(11.0), Color32::from_rgb(0x9B, 0xAD, 0x9C));
            }
            // ruler
            painter.rect_filled(Rect::from_min_size(org, vec2(resp.rect.width(), RUL)), 0.0, Color32::from_rgb(0x07, 0x17, 0x0f));
            let mut step = 1.0; for s in [0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 120.0] { step = s; if s * pps >= 64.0 { break; } }
            let mut t = 0.0; while t < dur + 8.0 {
                let x = org.x + GUT + t * pps;
                painter.line_segment([pos2(x, org.y + RUL - 8.0), pos2(x, org.y + RUL)], Stroke::new(1.0, Color32::from_gray(120)));
                painter.text(pos2(x + 3.0, org.y + 3.0), Align2::LEFT_TOP, format!("{}:{:02}", (t / 60.0) as u32, (t % 60.0) as u32), FontId::proportional(10.0), Color32::from_gray(150));
                t += step;
            }
            // clips
            for c in &self.prj.clips {
                let x0 = org.x + GUT + c.start * pps;
                let r = Rect::from_min_size(pos2(x0, org.y + RUL + c.track.min(3) as f32 * LH + 3.0), vec2((c.dur * pps).max(4.0), LH - 6.0));
                let (fill, label) = match c.kind { Kind::Anim => (c32(MOSS), c32(PAPER)), Kind::Text => (Color32::from_rgb(0x8a, 0xa8, 0x2a), c32(INK)), Kind::Image => (Color32::from_rgb(0x2f, 0x8a, 0x63), c32(PAPER)), Kind::Audio => (Color32::from_rgb(0x1f, 0x4f, 0x6b), c32(PAPER)) };
                let sel = self.sel == Some(c.id);
                painter.rect_filled(r, 6.0, fill);
                if c.kind == Kind::Audio { if let Some(a) = &c.asset { if let AssetData::Audio { sr, ch } = &a.data {
                    let mut x = r.left() + 2.0; let mid = r.center().y;
                    while x < r.right() - 2.0 {
                        let ta = c.trim + (x - r.left()) / pps; let i0 = (ta * *sr as f32) as usize; let i1 = (i0 + (3.0 / pps * *sr as f32) as usize).min(ch[0].len());
                        let mut m = 0.0f32; let stp = ((i1.saturating_sub(i0)) / 24).max(1);
                        let mut i = i0; while i < i1 { m = m.max(ch[0][i].abs()); i += stp; }
                        let hh = (m.min(1.0) * (LH - 18.0) / 2.0).max(1.0);
                        painter.line_segment([pos2(x, mid - hh), pos2(x, mid + hh)], Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 130)));
                        x += 3.0;
                    }
                } } }
                if sel { painter.rect_stroke(r, 6.0, Stroke::new(2.0, c32(LIME))); }
                if self.drag_bad && sel { painter.rect_stroke(r, 6.0, Stroke::new(2.5, Color32::RED)); }
                let galley_pos = pos2(r.left() + 7.0, r.top() + 5.0);
                painter.with_clip_rect(r.shrink(2.0)).text(galley_pos, Align2::LEFT_TOP, &c.name, FontId::proportional(11.5), label);
            }
            // playhead
            let px = org.x + GUT + self.ph * pps;
            painter.line_segment([pos2(px, org.y), pos2(px, org.y + RUL + 4.0 * LH)], Stroke::new(2.0, c32(LIME)));
            painter.add(Shape::convex_polygon(vec![pos2(px - 6.0, org.y), pos2(px + 6.0, org.y), pos2(px, org.y + 10.0)], c32(LIME), Stroke::NONE));

            // interaction
            if resp.drag_started() || resp.clicked() {
                if let Some(p) = ui.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos()) {
                    let lane = ((p.y - org.y - RUL) / LH).floor();
                    if p.y < org.y + RUL { self.drag = Some(Drag::Seek); }
                    else {
                        let hit = self.prj.clips.iter().rev().find(|c| {
                            let x0 = org.x + GUT + c.start * pps; lane >= 0.0 && c.track == lane as usize && p.x >= x0 && p.x <= x0 + c.dur * pps
                        }).cloned();
                        match hit {
                            Some(c) => {
                                self.sel = Some(c.id);
                                let x0 = org.x + GUT + c.start * pps; let x1 = x0 + c.dur * pps; let edge = 8.0f32.min(c.dur * pps / 3.0);
                                self.drag = Some(if p.x - x0 < edge { Drag::ResizeL { id: c.id, orig_start: c.start, orig_dur: c.dur, orig_trim: c.trim } }
                                    else if x1 - p.x < edge { Drag::ResizeR { id: c.id } }
                                    else { Drag::Move { id: c.id, grab: (p.x - org.x - GUT) / pps - c.start, orig_start: c.start, orig_track: c.track } });
                                if resp.drag_started() { self.snapshot(); }
                            }
                            None => { self.sel = None; self.drag = None; }
                        }
                    }
                }
            }
            if resp.dragged() {
                if let (Some(d), Some(p)) = (self.drag, resp.interact_pointer_pos()) {
                    let tt = (p.x - org.x - GUT) / pps;
                    match d {
                        Drag::Seek => self.seek(tt),
                        Drag::Move { id, grab, .. } => {
                            if let Some(cl) = self.prj.get(id).cloned() {
                                let ns = self.snap((tt - grab).max(0.0), id, cl.dur);
                                let lane = (((p.y - org.y - RUL) / LH).floor().max(0.0) as usize).min(3);
                                let lane = if cl.kind == Kind::Audio { 3 } else { lane.min(2) };
                                if let Some(m) = self.prj.get_mut(id) { m.start = ns; m.track = lane; }
                                let upd = self.prj.get(id).cloned().unwrap(); self.drag_bad = self.prj.overlaps(&upd); self.changed();
                            }
                        }
                        Drag::ResizeR { id } => {
                            if let Some(cl) = self.prj.get(id).cloned() {
                                let end = self.snap(tt, id, 0.0);
                                let mut nd = (end - cl.start).max(0.2);
                                if cl.kind == Kind::Audio { if let Some(a) = &cl.asset { nd = nd.min(a.duration() - cl.trim); } }
                                if let Some(m) = self.prj.get_mut(id) { m.dur = nd; }
                                let upd = self.prj.get(id).cloned().unwrap(); self.drag_bad = self.prj.overlaps(&upd); self.changed();
                            }
                        }
                        Drag::ResizeL { id, orig_start, orig_dur, orig_trim } => {
                            let mut ns = self.snap(tt, id, 0.0).max(0.0);
                            ns = ns.min(orig_start + orig_dur - 0.2);
                            if matches!(self.prj.get(id).map(|c| c.kind), Some(Kind::Anim | Kind::Audio)) { ns = ns.max(orig_start - orig_trim); }
                            let delta = ns - orig_start;
                            if let Some(m) = self.prj.get_mut(id) { m.start = ns; m.dur = orig_dur - delta; if matches!(m.kind, Kind::Anim | Kind::Audio) { m.trim = orig_trim + delta; } }
                            let upd = self.prj.get(id).cloned().unwrap(); self.drag_bad = self.prj.overlaps(&upd); self.changed();
                        }
                    }
                }
            } else if resp.clicked() { if let Some(Drag::Seek) = self.drag { if let Some(p) = resp.interact_pointer_pos() { self.seek((p.x - org.x - GUT) / pps); } } }
            if resp.drag_stopped() {
                if let Some(Drag::Move { id, orig_start, orig_track, .. }) = self.drag {
                    if self.drag_bad { if let Some(m) = self.prj.get_mut(id) { m.start = orig_start; m.track = orig_track; } self.changed(); self.msg = "Clips cannot overlap on the same layer.".into(); }
                }
                if let Some(Drag::ResizeR { id } | Drag::ResizeL { id, .. }) = self.drag {
                    if self.drag_bad { if let Some(prev) = self.hist.last().and_then(|p| p.get(id)).cloned() { if let Some(m) = self.prj.get_mut(id) { *m = prev; } self.changed(); } }
                }
                self.drag = None; self.drag_bad = false;
            }
        });
    }

    fn drop_pos(&self, ctx: &Context) -> Option<(f32, usize)> {
        let p = ctx.input(|i| i.pointer.latest_pos())?;
        if !self.tl_rect.contains(p) { return None; }
        let lane = ((p.y - self.tl_org.y - RUL) / LH).floor();
        if lane < 0.0 { return None; }
        Some((((p.x - self.tl_org.x - GUT) / self.pps).max(0.0), (lane as usize).min(3)))
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _f: &mut eframe::Frame) {
        // files dropped from the desktop
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() {
            let at = self.drop_pos(ctx);
            let code_target = self.code_open && ctx.input(|i| i.pointer.latest_pos()).map_or(false, |p| p.x > ctx.screen_rect().right() - 480.0 && p.y < self.tl_rect.top());
            if code_target && dropped.len() == 1 && matches!(dropped[0].extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref(), Some("json") | Some("txt") | Some("js") | Some("md")) {
                if let Ok(s) = std::fs::read_to_string(&dropped[0]) { self.code = s; self.code_dirty = true; self.generate(); }
            } else { self.load_paths(dropped, at); }
        }
        self.receive();
        // time
        let dt = self.last_tick.elapsed().as_secs_f32(); self.last_tick = Instant::now();
        if self.playing {
            self.ph += dt;
            if self.ph >= self.prj.duration() { self.ph = self.prj.duration(); self.set_play(false); }
            ctx.request_repaint();
        }
        // keys
        if !ctx.wants_keyboard_input() {
            let (space, del, left, right, s, d, z, y, shift, cmd) = ctx.input(|i| (i.key_pressed(Key::Space), i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace), i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight), i.key_pressed(Key::S), i.key_pressed(Key::D), i.key_pressed(Key::Z), i.key_pressed(Key::Y), i.modifiers.shift, i.modifiers.command));
            if space { let p = !self.playing; self.set_play(p); }
            if del { self.delete(); }
            let step = if shift { 1.0 } else { 1.0 / self.prj.fps as f32 };
            if left { let t = self.ph - step; self.seek(t); }
            if right { let t = self.ph + step; self.seek(t); }
            if s && !cmd { self.split(); }
            if d && cmd { self.duplicate(); }
            if z && cmd { if shift { self.redo(); } else { self.undo(); } }
            if y && cmd { self.redo(); }
        }
        // keep the code in step with the timeline
        if self.code_open && self.code_rev != self.rev && !self.code_dirty { self.sync_code(false); self.code_rev = self.rev; }

        // top bar
        TopBottomPanel::top("bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Tech Unwind").strong().size(18.0).color(c32(LIME)));
                ui.menu_button("File", |ui| {
                    if ui.button("Open code file…").clicked() { ui.close_menu(); if let Some(p) = rfd::FileDialog::new().add_filter("Code", &["json", "txt", "js"]).pick_file() { if let Ok(s) = std::fs::read_to_string(p) { self.code = s; self.code_dirty = true; self.generate(); self.code_open = true; } } }
                    if ui.button("Save code file…").clicked() { ui.close_menu(); if let Some(p) = rfd::FileDialog::new().set_file_name("tech-unwind.json").save_file() { let _ = std::fs::write(p, spec::export(&self.prj)); } }
                    if ui.button("Undo").clicked() { ui.close_menu(); self.undo(); }
                    if ui.button("Redo").clicked() { ui.close_menu(); self.redo(); }
                });
                ui.separator();
                if ui.button(if self.playing { "⏸ Pause" } else { "▶ Play" }).clicked() { let p = !self.playing; self.set_play(p); }
                if ui.button("⏮").clicked() { self.seek(0.0); }
                ui.monospace(format!("{} / {}", mmss(self.ph), mmss(self.prj.duration())));
                ui.separator();
                if ui.button("Split (S)").clicked() { self.split(); }
                if ui.button("Duplicate").clicked() { self.duplicate(); }
                if ui.button("Delete").clicked() { self.delete(); }
                ui.separator();
                ui.label("Zoom"); ui.add(Slider::new(&mut self.pps, 10.0..=240.0).show_value(false));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(Button::new(RichText::new("Export video").color(c32(INK)).strong()).fill(c32(LIME))).clicked() { self.export_open = true; }
                    if ui.selectable_label(self.code_open, "Code").clicked() { self.code_open = !self.code_open; if self.code_open { self.sync_code(false); } }
                });
            });
        });
        SidePanel::left("lib").exact_width(250.0).resizable(false).show(ctx, |ui| { ScrollArea::vertical().show(ui, |ui| self.left(ui)); });
        SidePanel::right("right").default_width(if self.code_open { 460.0 } else { 330.0 }).width_range(300.0..=700.0).show(ctx, |ui| {
            if self.code_open { self.code_panel(ui); } else { ScrollArea::vertical().show(ui, |ui| self.props(ui)); }
        });
        TopBottomPanel::bottom("timeline").resizable(false).exact_height(RUL + 4.0 * LH + 30.0).show(ctx, |ui| self.timeline(ui));
        self.render_preview(ctx);
        CentralPanel::default().show(ctx, |ui| {
            if let Some(t) = &self.tex {
                let avail = ui.available_size(); let a = self.prj.w as f32 / self.prj.h as f32;
                let (mut w, mut h) = (avail.x, avail.x / a); if h > avail.y { h = avail.y; w = h * a; }
                let (rect, _) = ui.allocate_exact_size(avail, Sense::hover());
                let r = Rect::from_center_size(rect.center(), vec2(w, h));
                ui.painter().rect_filled(r, 0.0, Color32::BLACK);
                ui.painter().image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
        });
        // export window
        if self.export_open {
            let mut open = true;
            Window::new("Export").open(&mut open).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0)).show(ctx, |ui| {
                ui.label(format!("{}×{} at {} fps, {:.1} seconds", self.prj.w, self.prj.h, self.prj.fps, self.prj.duration()));
                if let Some(j) = &self.export {
                    let done = j.done_frames.load(std::sync::atomic::Ordering::Relaxed);
                    ui.add(ProgressBar::new(done as f32 / j.total as f32).text(format!("{} / {} frames", done, j.total)));
                    let res = j.result.lock().unwrap().clone();
                    match res {
                        None => { if ui.button("Cancel").clicked() { j.cancel.store(true, std::sync::atomic::Ordering::Relaxed); } ctx.request_repaint_after(Duration::from_millis(100)); }
                        Some(r) => { self.export_msg = Some(match r { Ok(m) => m, Err(e) => e }); }
                    }
                } else {
                    ui.radio_value(&mut self.export_png, false, "MP4 video (needs ffmpeg)");
                    ui.radio_value(&mut self.export_png, true, "PNG frames in a folder");
                    if ui.add(Button::new(RichText::new("Choose where to save…").color(c32(INK)).strong()).fill(c32(LIME))).clicked() {
                        let out = if self.export_png { rfd::FileDialog::new().pick_folder() } else { rfd::FileDialog::new().set_file_name("tech-unwind.mp4").add_filter("MP4", &["mp4"]).save_file() };
                        if let Some(p) = out { self.export_msg = None; self.export = Some(export::start(self.prj.clone(), p, self.export_png)); }
                    }
                }
                if let Some(m) = self.export_msg.clone() { ui.separator(); ui.label(m); if ui.button("Done").clicked() { self.export = None; self.export_msg = None; self.export_open = false; } }
            });
            if !open { if self.export.as_ref().map_or(true, |j| j.result.lock().unwrap().is_some()) { self.export = None; self.export_msg = None; self.export_open = false; } else { self.export_open = true; } }
        }
        // drop overlay
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let p = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("drop")));
            let r = ctx.screen_rect();
            p.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(7, 23, 15, 200));
            p.rect_stroke(r.shrink(6.0), 12.0, Stroke::new(3.0, c32(LIME)));
            p.text(r.center(), Align2::CENTER_CENTER, "Drop files to add them\nDrop on a timeline lane to place them there", FontId::proportional(24.0), c32(PAPER));
        }
        let _ = (DEEP, FOREST, Al::Left);
    }
}

impl App {
    pub fn open_code_file(&mut self, path: &str) {
        match std::fs::read_to_string(path) { Ok(s) => { self.code = s; self.code_dirty = true; self.code_open = true; self.generate(); } Err(e) => self.msg = format!("{}: {}", path, e) }
    }
}
