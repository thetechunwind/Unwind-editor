//! The "code" view of a project: the same JSON-like format as the web editor.
use crate::anim;
use crate::assets;
use crate::gfx::*;
use crate::model::*;
use serde_json::{Map, Value};
use std::sync::Arc;

pub enum SpecErr { Parse { msg: String, line: usize, col: usize }, List(Vec<String>) }
impl SpecErr {
    pub fn text(&self) -> String {
        match self {
            SpecErr::Parse { msg, line, col } => format!("Line {}, column {}: {}. Nothing was changed.", line, col, msg),
            SpecErr::List(l) => format!("{}{}", if l.len() > 1 { format!("{} problems, nothing was changed:\n", l.len()) } else { "Nothing was changed. ".into() },
                l.iter().take(4).cloned().collect::<Vec<_>>().join("\n") + &if l.len() > 4 { format!("\n+ {} more", l.len() - 4) } else { String::new() }),
        }
    }
}

pub struct Compiled { pub clips: Vec<Clip>, pub nid: u32, pub canvas: Option<(u32, u32)>, pub fps: Option<u32>, pub bg: Option<String>, pub assets: Vec<Arc<Asset>>, pub warn: Vec<String> }

/// Strips a markdown fence and turns `backtick strings` into normal strings.
fn preprocess(src: &str) -> String {
    let mut s = src.trim_start_matches('\u{feff}').to_string();
    let t = s.trim_start();
    if t.starts_with("```") { let cut = s.find("```").unwrap(); let nl = s[cut..].find('\n').map(|i| cut + i).unwrap_or(s.len()); s = format!("{}{}", " ".repeat(0), &s[nl..]); s.insert(0, '\n'); }
    let tr = s.trim_end(); if tr.ends_with("```") { s = tr[..tr.len() - 3].to_string(); }
    let ch: Vec<char> = s.chars().collect();
    let mut out = String::new(); let mut i = 0; let mut q: Option<char> = None;
    while i < ch.len() {
        let c = ch[i];
        if let Some(qc) = q {
            out.push(c);
            if c == '\\' && i + 1 < ch.len() { out.push(ch[i + 1]); i += 2; continue; }
            if c == qc { q = None; }
            i += 1; continue;
        }
        if c == '/' && ch.get(i + 1) == Some(&'/') { while i < ch.len() && ch[i] != '\n' { out.push(ch[i]); i += 1; } continue; }
        if c == '/' && ch.get(i + 1) == Some(&'*') { while i < ch.len() && !(ch[i] == '*' && ch.get(i + 1) == Some(&'/')) { out.push(ch[i]); i += 1; } if i < ch.len() { out.push_str("*/"); i += 2; } continue; }
        if c == '"' || c == '\'' { q = Some(c); out.push(c); i += 1; continue; }
        if c == '`' {
            i += 1; out.push('"');
            while i < ch.len() && ch[i] != '`' {
                match ch[i] { '\n' => out.push_str("\\n"), '\r' => {}, '"' => out.push_str("\\\""), '\\' if i + 1 < ch.len() => { out.push('\\'); out.push(ch[i + 1]); i += 1; } c2 => out.push(c2) }
                i += 1;
            }
            out.push('"'); i += 1; continue;
        }
        out.push(c); i += 1;
    }
    out
}

pub fn parse(src: &str) -> Result<Value, SpecErr> {
    if src.trim().is_empty() { return Err(SpecErr::List(vec!["Paste your code first.".into()])); }
    json5::from_str::<Value>(&preprocess(src)).map_err(|e| match e {
        json5::Error::Message { msg, location } => { let (line, col) = location.map(|l| (l.line, l.column)).unwrap_or((1, 1)); SpecErr::Parse { msg: msg.trim_end_matches('.').to_string(), line, col } }
    })
}

struct Cx { errs: Vec<String>, warn: Vec<String>, cur: String }
impl Cx {
    fn num(&mut self, v: Option<&Value>, name: &str, def: Option<f32>, min: Option<f32>, max: Option<f32>) -> Option<f32> {
        let Some(v) = v else { return def };
        let n = match v { Value::Number(n) => n.as_f64().map(|x| x as f32), Value::String(s) if !s.trim().is_empty() => s.trim().parse::<f32>().ok(), _ => None };
        let Some(n) = n.filter(|x| x.is_finite()) else { self.errs.push(format!("{}: \"{}\" must be a number.", self.cur, name)); return def };
        if let Some(m) = min { if n < m { self.errs.push(format!("{}: \"{}\" must be at least {}.", self.cur, name, m)); return def; } }
        if let Some(m) = max { if n > m { self.errs.push(format!("{}: \"{}\" must be at most {}.", self.cur, name, m)); return def; } }
        Some(n)
    }
    fn text(&mut self, v: &Value, name: &str) -> String {
        match v {
            Value::String(s) => s.clone(), Value::Number(n) => n.to_string(),
            Value::Array(a) if a.iter().all(|e| e.is_string() || e.is_number()) => a.iter().map(|e| match e { Value::String(s) => s.clone(), o => o.to_string() }).collect::<Vec<_>>().join("\n"),
            _ => { self.errs.push(format!("{}: \"{}\" must be text.", self.cur, name)); String::new() }
        }
    }
    fn pick(&mut self, v: Option<&Value>, name: &str, list: &[&str], def: &str) -> String {
        let Some(v) = v else { return def.into() };
        let mut s = v.as_str().map(|x| x.to_lowercase()).unwrap_or_default();
        if s == "centre" { s = "center".into(); }
        if !list.contains(&s.as_str()) { self.errs.push(format!("{}: \"{}\" must be one of {}.", self.cur, name, list.join(", "))); return def.into(); }
        s
    }
    fn hex(&mut self, v: Option<&Value>, name: &str, def: Col) -> Col {
        let Some(v) = v else { return def };
        match v.as_str().and_then(Col::parse) { Some(c) => c, None => { self.errs.push(format!("{}: \"{}\" must be a hex colour like #C8F23A.", self.cur, name)); def } }
    }
    fn boolean(&mut self, v: Option<&Value>, name: &str, def: bool) -> bool {
        match v { None => def, Some(Value::Bool(b)) => *b, _ => { self.errs.push(format!("{}: \"{}\" must be true or false.", self.cur, name)); def } }
    }
}

const COMMON: &[&str] = &["type", "name", "start", "dur", "duration", "layer", "x", "y", "scale", "rotate", "opacity", "keys", "ease"];
fn allowed(kind: &str) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = COMMON.to_vec();
    match kind {
        "anim" => v.extend(["tag", "num", "head", "sub", "data", "backdrop", "colors", "colours", "trim"]),
        "text" => v.extend(["text", "style", "size", "color", "colour", "highlight", "align", "in", "out"]),
        "image" => v.extend(["file", "fit", "width", "shape", "border", "borderColor", "shadow"]),
        _ => v = vec!["type", "name", "start", "dur", "duration", "file", "trim", "volume", "mute", "fadeIn", "fadeOut"],
    }
    v
}

pub fn compile(prj: &Project, root: &Value) -> Result<Compiled, SpecErr> {
    let root_obj: Map<String, Value> = match root { Value::Array(a) => { let mut m = Map::new(); m.insert("clips".into(), Value::Array(a.clone())); m } Value::Object(o) => o.clone(), _ => Map::new() };
    let Some(Value::Array(list)) = root_obj.get("clips") else {
        return Err(SpecErr::List(vec!["The code needs a \"clips\" list, for example { \"clips\": [ { \"type\": \"head\", \"start\": 0, \"dur\": 5 } ] }.".into()]));
    };
    let mut cx = Cx { errs: vec![], warn: vec![], cur: String::new() };
    let mut out = Compiled { clips: vec![], nid: 1, canvas: None, fps: None, bg: None, assets: vec![], warn: vec![] };
    for k in root_obj.keys() { if !["canvas", "fps", "background", "clips"].contains(&k.as_str()) { cx.warn.push(format!("Project field \"{}\" was ignored.", k)); } }
    let (mut cw, mut ch) = (prj.w, prj.h);
    if let Some(v) = root_obj.get("canvas") {
        let s = v.as_str().unwrap_or("").to_lowercase().replace('×', "x").replace(' ', "");
        if ["1920x1080", "1080x1920", "1080x1350", "1080x1080"].contains(&s.as_str()) { let p: Vec<u32> = s.split('x').map(|x| x.parse().unwrap()).collect(); cw = p[0]; ch = p[1]; out.canvas = Some((cw, ch)); }
        else { cx.errs.push("\"canvas\" must be one of 1920x1080, 1080x1920, 1080x1350, 1080x1080.".into()); }
    }
    if let Some(v) = root_obj.get("fps") { match v.as_f64().map(|x| x as u32) { Some(f) if [24, 30, 60].contains(&f) => out.fps = Some(f), _ => cx.errs.push("\"fps\" must be 24, 30 or 60.".into()) } }
    if let Some(v) = root_obj.get("background") { let s = v.as_str().unwrap_or("").to_lowercase(); if ["deep", "ink", "black", "clear"].contains(&s.as_str()) { out.bg = Some(s) } else { cx.errs.push("\"background\" must be one of deep, ink, black, clear.".into()); } }
    let mut prev_end = 0.0f32;
    let mut known: Vec<Arc<Asset>> = prj.assets.clone();
    for (i, spv) in list.iter().enumerate() {
        cx.cur = format!("Clip {}", i + 1);
        let e0 = cx.errs.len();
        let Some(sp) = spv.as_object() else { cx.errs.push(format!("{} must be an object like {{ \"type\": \"head\", \"start\": 0, \"dur\": 5 }}.", cx.cur)); continue };
        let ty = sp.get("type").and_then(|v| v.as_str()).unwrap_or("").trim().to_lowercase();
        let kind = if anim::tpl(&ty).is_some() { "anim" } else if ["text", "image", "audio"].contains(&ty.as_str()) { ty.as_str() } else { "" };
        if kind.is_empty() {
            if ty == "video" { cx.errs.push(format!("{}: video clips are not in the desktop app yet.", cx.cur)); }
            else { cx.errs.push(format!("{}: type \"{}\" is not known. Open \"Names and fields\" below for the list.", cx.cur, ty)); }
            continue;
        }
        cx.cur = format!("{} ({})", cx.cur, ty);
        let ok = allowed(kind);
        for k in sp.keys() { if !ok.contains(&k.as_str()) { cx.warn.push(format!("{}: \"{}\" is not a field here and was ignored.", cx.cur, k)); } }
        let start = cx.num(sp.get("start"), "start", Some(prev_end), Some(0.0), None).unwrap_or(0.0);
        let dur_v = sp.get("dur").or_else(|| sp.get("duration"));
        let dur = cx.num(dur_v, "dur", None, Some(0.2), None);
        let mut layer: Option<usize> = None;
        if kind != "audio" { if let Some(l) = cx.num(sp.get("layer"), "layer", None, Some(1.0), Some(3.0)) { if l.fract() != 0.0 { cx.errs.push(format!("{}: \"layer\" must be 1, 2 or 3.", cx.cur)); } else { layer = Some(3 - l as usize); } } }
        let mut c: Clip;
        let cands: Vec<usize>;
        match kind {
            "anim" => {
                let tp = anim::tpl(&ty).unwrap();
                c = Clip::new_anim(&ty);
                let fl: Vec<&str> = tp.fields.split(' ').collect();
                for k in ["tag", "num", "head", "sub", "data"] {
                    if let Some(v) = sp.get(k) {
                        if !fl.contains(&k) { cx.warn.push(format!("{}: \"{}\" is not used by this animation.", cx.cur, k)); }
                        let t = cx.text(v, k);
                        match k { "tag" => c.p.tag = t, "num" => c.p.num = t, "head" => c.p.head = t, "sub" => c.p.sub = t, _ => c.p.data = t }
                    }
                }
                if let Some(v) = sp.get("backdrop") { let b = v.as_str().unwrap_or("").to_lowercase(); if ["brand", "clear", "chroma"].contains(&b.as_str()) { c.p.bgm = b } else { cx.errs.push(format!("{}: \"backdrop\" must be brand, clear or chroma.", cx.cur)); } }
                if let Some(co) = sp.get("colors").or_else(|| sp.get("colours")) {
                    match co.as_object() {
                        None => cx.errs.push(format!("{}: \"colors\" must look like {{ \"hl\": \"#C8F23A\" }}.", cx.cur)),
                        Some(o) => for (k, v) in o { if ["bg", "head", "hl", "sm"].contains(&k.as_str()) { let col = cx.hex(Some(v), &format!("colors.{}", k), LIME); c.p.col.insert(k.clone(), col); } else { cx.warn.push(format!("{}: colour \"{}\" was ignored. Use bg, head, hl or sm.", cx.cur, k)); } }
                    }
                }
                let hd: String = [&c.p.head, &c.p.num, &c.p.tag].iter().find(|s| !s.is_empty()).map(|s| s.replace('*', "").chars().take(26).collect()).unwrap_or_default();
                c.name = match sp.get("name").and_then(|v| v.as_str()) { Some(n) => n.into(), None => format!("{}{}", tp.name, if hd.is_empty() { String::new() } else { format!(" · {}", hd) }) };
                c.start = start; c.dur = dur.unwrap_or(tp.dur); c.trim = cx.num(sp.get("trim"), "trim", Some(0.0), Some(0.0), None).unwrap_or(0.0);
                cands = if tp.clear { vec![1, 2, 0] } else { vec![2, 1, 0] };
            }
            "text" => {
                c = Clip::new_text();
                let o = &mut c.txt;
                if let Some(v) = sp.get("text") { o.text = cx.text(v, "text"); }
                o.style = cx.pick(sp.get("style"), "style", &["headline", "serif", "label"], &o.style.clone());
                o.size = cx.num(sp.get("size"), "size", Some(o.size), Some(0.5), Some(100.0)).unwrap();
                o.color = cx.hex(sp.get("color").or_else(|| sp.get("colour")), "color", o.color);
                o.hl = cx.hex(sp.get("highlight"), "highlight", o.hl);
                o.align = cx.pick(sp.get("align"), "align", &["center", "left"], &o.align.clone());
                o.in_a = cx.pick(sp.get("in"), "in", &["rise", "fade", "words", "none"], &o.in_a.clone());
                o.out_a = cx.pick(sp.get("out"), "out", &["fade", "none"], &o.out_a.clone());
                c.name = match sp.get("name").and_then(|v| v.as_str()) { Some(n) => n.into(), None => format!("Text · {}", c.txt.text.replace('*', "").chars().take(24).collect::<String>()) };
                c.start = start; c.dur = dur.unwrap_or(3.5);
                cands = vec![0, 1, 2];
            }
            _ => {
                let fname = sp.get("file").and_then(|v| v.as_str());
                let Some(fname) = fname else { cx.errs.push(format!("{}: \"file\" is needed. Use the name of a file you added, or a path.", cx.cur)); continue };
                let asset = known.iter().rev().find(|a| a.name.to_lowercase() == fname.trim().to_lowercase()).cloned().or_else(|| {
                    match assets::load(std::path::Path::new(fname.trim())) { Ok(a) => { known.push(a.clone()); out.assets.push(a.clone()); Some(a) } Err(_) => None }
                });
                let Some(asset) = asset else { cx.warn.push(format!("{}: \"{}\" is not loaded, so this clip was skipped. Add the file, then press Generate video.", cx.cur, fname)); continue };
                if kind == "image" {
                    if !matches!(asset.data, AssetData::Image(_)) { cx.errs.push(format!("{}: \"{}\" is not an image.", cx.cur, fname)); continue; }
                    c = Clip::blank(Kind::Image);
                    c.fit = cx.pick(sp.get("fit"), "fit", &["free", "cover", "contain"], "free");
                    c.shape = cx.pick(sp.get("shape"), "shape", &["none", "round", "circle"], "none");
                    c.bw = cx.num(sp.get("border"), "border", Some(0.0), Some(0.0), None).unwrap();
                    c.bc = cx.hex(sp.get("borderColor"), "borderColor", LIME);
                    c.shadow = cx.boolean(sp.get("shadow"), "shadow", false);
                    let dw = if let AssetData::Image(p) = &asset.data { let (nw, nh) = (p.width() as f32, p.height() as f32); (0.6 * ch as f32 * (nw / nh) / cw as f32 * 100.0).clamp(12.0, 70.0).round() } else { 40.0 };
                    c.w = cx.num(sp.get("width"), "width", Some(dw), Some(1.0), Some(300.0)).unwrap();
                    c.start = start; c.dur = dur.unwrap_or(4.0);
                    cands = vec![1, 0, 2];
                } else {
                    if !matches!(asset.data, AssetData::Audio { .. }) { cx.errs.push(format!("{}: \"{}\" is not an audio file.", cx.cur, fname)); continue; }
                    c = Clip::blank(Kind::Audio);
                    c.track = 3;
                    c.trim = cx.num(sp.get("trim"), "trim", Some(0.0), Some(0.0), None).unwrap();
                    let sd = asset.duration();
                    if c.trim >= sd - 0.1 { cx.errs.push(format!("{}: \"trim\" is longer than the file.", cx.cur)); c.trim = 0.0; }
                    let mx = sd - c.trim;
                    c.dur = match dur { None => mx, Some(d) => { if d > mx + 0.01 { cx.warn.push(format!("{}: \"dur\" was shortened to the length of the file.", cx.cur)); } d.min(mx) } };
                    c.vol = cx.num(sp.get("volume"), "volume", Some(1.0), Some(0.0), Some(4.0)).unwrap();
                    c.mute = cx.boolean(sp.get("mute"), "mute", false);
                    c.fade_in = cx.num(sp.get("fadeIn"), "fadeIn", Some(0.0), Some(0.0), None).unwrap();
                    c.fade_out = cx.num(sp.get("fadeOut"), "fadeOut", Some(0.0), Some(0.0), None).unwrap();
                    c.start = start;
                    cands = vec![3];
                }
                c.name = sp.get("name").and_then(|v| v.as_str()).map(String::from).unwrap_or_else(|| asset.name.clone());
                c.file = asset.name.clone();
                c.asset = Some(asset);
            }
        }
        if kind != "audio" {
            c.tf = Tf { x: cx.num(sp.get("x"), "x", Some(0.0), None, None).unwrap(), y: cx.num(sp.get("y"), "y", Some(0.0), None, None).unwrap(), s: cx.num(sp.get("scale"), "scale", Some(1.0), Some(0.01), None).unwrap(),
                        r: cx.num(sp.get("rotate"), "rotate", Some(0.0), None, None).unwrap(), o: cx.num(sp.get("opacity"), "opacity", Some(1.0), Some(0.0), Some(1.0)).unwrap() };
            if let Some(e) = sp.get("ease") { match e.as_str().map(|s| s.to_lowercase()).as_deref() { Some("linear") | Some("lin") => c.lin = true, Some("smooth") => {}, _ => cx.errs.push(format!("{}: \"ease\" must be smooth or linear.", cx.cur)) } }
            if let Some(k) = sp.get("keys") {
                match k.as_object() {
                    None => cx.errs.push(format!("{}: \"keys\" must look like {{ \"opacity\": [[0, 0], [0.6, 1]] }}.", cx.cur)),
                    Some(ko) => for (name, arr) in ko {
                        let Some(pi) = ["x", "y", "scale", "rotate", "opacity"].iter().position(|n| n == name) else { cx.warn.push(format!("{}: keys \"{}\" was ignored. Use x, y, scale, rotate or opacity.", cx.cur, name)); continue };
                        let Some(arr) = arr.as_array() else { cx.errs.push(format!("{}: keys.{} must be a list like [[0, 0], [0.6, 1]].", cx.cur, name)); continue };
                        for q in arr {
                            let (t, v) = match q { Value::Array(a) if a.len() >= 2 => (a[0].as_f64(), a[1].as_f64()), Value::Object(o) => (o.get("t").or(o.get("time")).and_then(|x| x.as_f64()), o.get("v").or(o.get("value")).and_then(|x| x.as_f64())), _ => (None, None) };
                            match (t, v) { (Some(t), Some(v)) => c.add_key(pi, (t as f32).clamp(0.0, c.dur), v as f32), _ => cx.errs.push(format!("{}: each keyframe in keys.{} must be [time, value].", cx.cur, name)) }
                        }
                    }
                }
            }
        }
        if cx.errs.len() > e0 { continue; }
        c.id = out.nid; out.nid += 1;
        if kind == "audio" { c.track = 3; if out.clips.iter().any(|o| o.track == 3 && o.start < c.end() - 0.001 && o.end() > c.start + 0.001) { cx.warn.push(format!("{} overlaps another audio clip.", cx.cur)); } }
        else {
            let cd: Vec<usize> = match layer { Some(l) => vec![l], None => cands };
            let mut placed = false;
            for &t in &cd { c.track = t; if !out.clips.iter().any(|o| o.track == t && o.start < c.end() - 0.001 && o.end() > c.start + 0.001) { placed = true; break; } }
            if !placed { c.track = cd[0]; cx.warn.push(format!("{} overlaps another clip on Layer {}.", cx.cur, 3 - c.track)); }
        }
        prev_end = c.end();
        out.clips.push(c);
    }
    if !cx.errs.is_empty() { return Err(SpecErr::List(cx.errs)); }
    out.warn = cx.warn;
    Ok(out)
}

// ---------- timeline to code ----------
fn r3(v: f32) -> Value { let r = ((v as f64) * 1000.0).round() / 1000.0; serde_json::Number::from_f64(r).map(Value::Number).unwrap_or(Value::Null) }
pub fn export(prj: &Project) -> String {
    let mut clips = prj.clips.clone();
    clips.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap().then(b.track.cmp(&a.track)));
    let mut list = vec![];
    for c in &clips {
        let mut e = Map::new();
        let lay = 3 - c.track.min(3);
        match c.kind {
            Kind::Anim => {
                let tp = anim::tpl(&c.p.tpl).unwrap_or(&anim::TPLS[0]);
                e.insert("type".into(), c.p.tpl.clone().into());
                if !c.name.starts_with(tp.name) { e.insert("name".into(), c.name.clone().into()); }
                e.insert("start".into(), r3(c.start)); e.insert("dur".into(), r3(c.dur));
                if c.trim > 0.0005 { e.insert("trim".into(), r3(c.trim)); }
                if lay != if tp.clear { 2 } else { 1 } { e.insert("layer".into(), lay.into()); }
                let fl: Vec<&str> = tp.fields.split(' ').collect();
                for (k, v) in [("tag", &c.p.tag), ("num", &c.p.num), ("head", &c.p.head), ("sub", &c.p.sub), ("data", &c.p.data)] {
                    if fl.contains(&k) { e.insert(k.into(), if k == "data" && v.contains('\n') { Value::Array(v.split('\n').map(|s| s.into()).collect()) } else { v.clone().into() }); }
                }
                if c.p.bgm != if tp.clear { "clear" } else { "brand" } { e.insert("backdrop".into(), c.p.bgm.clone().into()); }
                if !c.p.col.is_empty() { let mut o = Map::new(); for k in ["bg", "head", "hl", "sm"] { if let Some(v) = c.p.col.get(k) { o.insert(k.into(), v.hex().into()); } } e.insert("colors".into(), Value::Object(o)); }
            }
            Kind::Text => {
                let o = &c.txt;
                e.insert("type".into(), "text".into());
                if !c.name.starts_with("Text") { e.insert("name".into(), c.name.clone().into()); }
                e.insert("start".into(), r3(c.start)); e.insert("dur".into(), r3(c.dur));
                if lay != 3 { e.insert("layer".into(), lay.into()); }
                e.insert("text".into(), o.text.clone().into()); e.insert("style".into(), o.style.clone().into()); e.insert("size".into(), r3(o.size));
                e.insert("color".into(), o.color.hex().into()); e.insert("highlight".into(), o.hl.hex().into()); e.insert("align".into(), o.align.clone().into());
                e.insert("in".into(), o.in_a.clone().into()); e.insert("out".into(), o.out_a.clone().into());
            }
            Kind::Image => {
                e.insert("type".into(), "image".into()); e.insert("file".into(), c.file.clone().into());
                if c.name != c.file { e.insert("name".into(), c.name.clone().into()); }
                e.insert("start".into(), r3(c.start)); e.insert("dur".into(), r3(c.dur));
                if lay != 2 { e.insert("layer".into(), lay.into()); }
                e.insert("fit".into(), c.fit.clone().into());
                if c.fit == "free" {
                    e.insert("width".into(), r3(c.w));
                    if c.shape != "none" { e.insert("shape".into(), c.shape.clone().into()); }
                    if c.bw > 0.0 { e.insert("border".into(), r3(c.bw)); e.insert("borderColor".into(), c.bc.hex().into()); }
                    if c.shadow { e.insert("shadow".into(), true.into()); }
                }
            }
            Kind::Audio => {
                e.insert("type".into(), "audio".into()); e.insert("file".into(), c.file.clone().into());
                if c.name != c.file { e.insert("name".into(), c.name.clone().into()); }
                e.insert("start".into(), r3(c.start)); e.insert("dur".into(), r3(c.dur));
                if c.trim > 0.0005 { e.insert("trim".into(), r3(c.trim)); }
                if (c.vol - 1.0).abs() > 1e-4 { e.insert("volume".into(), r3(c.vol)); }
                if c.mute { e.insert("mute".into(), true.into()); }
                if c.fade_in > 0.0 { e.insert("fadeIn".into(), r3(c.fade_in)); }
                if c.fade_out > 0.0 { e.insert("fadeOut".into(), r3(c.fade_out)); }
            }
        }
        if c.kind != Kind::Audio {
            let t = &c.tf;
            if t.x != 0.0 { e.insert("x".into(), r3(t.x)); } if t.y != 0.0 { e.insert("y".into(), r3(t.y)); }
            if t.s != 1.0 { e.insert("scale".into(), r3(t.s)); } if t.r != 0.0 { e.insert("rotate".into(), r3(t.r)); } if t.o != 1.0 { e.insert("opacity".into(), r3(t.o)); }
            let mut ko = Map::new();
            for (i, n) in ["x", "y", "scale", "rotate", "opacity"].iter().enumerate() { if !c.kf[i].is_empty() { ko.insert((*n).into(), Value::Array(c.kf[i].iter().map(|k| Value::Array(vec![r3(k.0), r3(k.1)])).collect())); } }
            if !ko.is_empty() { e.insert("keys".into(), Value::Object(ko)); }
            if c.lin { e.insert("ease".into(), "linear".into()); }
        }
        list.push(Value::Object(e));
    }
    let body = if list.is_empty() { String::new() } else { format!("\n{}\n  ", list.iter().map(|e| format!("    {}", ser(e, 2, ""))).collect::<Vec<_>>().join(",\n\n")) };
    format!("{{\n  \"canvas\": \"{}x{}\",\n  \"fps\": {},\n  \"background\": \"{}\",\n  \"clips\": [{}]\n}}", prj.w, prj.h, prj.fps, prj.bg, body)
}
fn ser(v: &Value, l: usize, key: &str) -> String {
    match v {
        Value::Array(a) => {
            if a.is_empty() { return "[]".into(); }
            let nums = a.iter().all(|e| e.is_number() || e.as_array().map_or(false, |x| x.iter().all(|n| n.is_number())));
            if nums { return format!("[{}]", a.iter().map(|e| match e { Value::Array(x) => format!("[{}]", x.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")), o => o.to_string() }).collect::<Vec<_>>().join(", ")); }
            let ip = "  ".repeat(l + 1);
            format!("[\n{}\n{}]", a.iter().map(|e| format!("{}{}", ip, ser(e, l + 1, ""))).collect::<Vec<_>>().join(",\n"), "  ".repeat(l))
        }
        Value::Object(o) => {
            if o.is_empty() { return "{}".into(); }
            if key == "colors" { return format!("{{ {} }}", o.iter().map(|(k, v)| format!("{}: {}", serde_json::to_string(k).unwrap(), v)).collect::<Vec<_>>().join(", ")); }
            let ip = "  ".repeat(l + 1);
            format!("{{\n{}\n{}}}", o.iter().map(|(k, v)| format!("{}{}: {}", ip, serde_json::to_string(k).unwrap(), ser(v, l + 1, k))).collect::<Vec<_>>().join(",\n"), "  ".repeat(l))
        }
        o => o.to_string(),
    }
}
