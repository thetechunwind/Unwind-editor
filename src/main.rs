mod anim; mod app; mod assets; mod audio; mod export; mod gfx; mod model; mod spec;
use model::*;
use std::path::PathBuf;

fn usage() {
    println!("Tech Unwind\n\n  tech_unwind                         open the editor\n  tech_unwind project.json            open the editor with a code file loaded\n  tech_unwind render code.json out.mp4   render without opening a window (out.mp4, or a folder for PNG frames)\n  tech_unwind frame code.json 12.5 out.png   save one frame\n  tech_unwind --sheet folder           save a still of every animation");
}

fn load_code(path: &str) -> Result<Project, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?;
    let mut prj = Project::new();
    let v = spec::parse(&src).map_err(|e| e.text())?;
    let c = spec::compile(&prj, &v).map_err(|e| e.text())?;
    for w in &c.warn { eprintln!("note: {}", w); }
    prj.assets.extend(c.assets); prj.clips = c.clips; prj.nid = c.nid;
    if let Some((w, h)) = c.canvas { prj.w = w; prj.h = h; }
    if let Some(f) = c.fps { prj.fps = f; }
    if let Some(b) = c.bg { prj.bg = b; }
    Ok(prj)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(|s| s.as_str()) {
        Some("--help") | Some("-h") => usage(),
        Some("--sheet") => {
            let out = a.get(2).cloned().unwrap_or("sheet".into());
            std::fs::create_dir_all(&out).unwrap();
            let mut r = Renderer::new(960, 540);
            for t in anim::TPLS {
                let mut p = Project::new(); p.add(Clip::new_anim(t.id), &[2]);
                let d = p.clips[0].dur.min(8.0);
                for (i, f) in [0.35f32, 0.8].iter().enumerate() {
                    let tt = if t.id == "chains" { 4.0 + i as f32 * 2.0 } else { d * f };
                    r.render(&p, tt).save_png(format!("{}/{}_{}.png", out, t.id, i)).unwrap();
                }
            }
        }
        Some("frame") if a.len() >= 5 => match load_code(&a[2]) {
            Ok(p) => { let t: f32 = a[3].parse().unwrap_or(0.0); let mut r = Renderer::new(p.w, p.h); r.render(&p, t).save_png(&a[4]).unwrap(); println!("Saved {}", a[4]); }
            Err(e) => { eprintln!("{}", e); std::process::exit(1); }
        },
        Some("render") if a.len() >= 4 => match load_code(&a[2]) {
            Ok(p) => {
                let total = (p.duration() * p.fps as f32).ceil().max(1.0) as u32;
                let out = PathBuf::from(&a[3]);
                let png = out.extension().is_none();
                let done = std::sync::atomic::AtomicU32::new(0); let cancel = std::sync::atomic::AtomicBool::new(false);
                match export::run(&p, &out, png, total, &done, &cancel) { Ok(m) => println!("{}", m), Err(e) => { eprintln!("{}", e); std::process::exit(1); } }
            }
            Err(e) => { eprintln!("{}", e); std::process::exit(1); }
        },
        other => {
            let opts = eframe::NativeOptions { viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1440.0, 900.0]).with_min_inner_size([1000.0, 640.0]).with_title("Tech Unwind").with_drag_and_drop(true), ..Default::default() };
            let file = other.map(String::from);
            let _ = eframe::run_native("Tech Unwind", opts, Box::new(move |cc| {
                let mut app = app::App::new(cc);
                if let Some(f) = file { app.open_code_file(&f); }
                Ok(Box::new(app))
            }));
        }
    }
}
