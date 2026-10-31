//! Video export: frames are rendered here and piped to ffmpeg.
use crate::audio;
use crate::model::*;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub struct Job { pub done_frames: Arc<AtomicU32>, pub total: u32, pub cancel: Arc<AtomicBool>, pub result: Arc<Mutex<Option<Result<String, String>>>> }

pub fn start(prj: Project, out: PathBuf, png_seq: bool) -> Job {
    let total = (prj.duration() * prj.fps as f32).ceil().max(1.0) as u32;
    let job = Job { done_frames: Arc::new(AtomicU32::new(0)), total, cancel: Arc::new(AtomicBool::new(false)), result: Arc::new(Mutex::new(None)) };
    let (df, cancel, res) = (job.done_frames.clone(), job.cancel.clone(), job.result.clone());
    std::thread::spawn(move || {
        let r = run(&prj, &out, png_seq, total, &df, &cancel);
        *res.lock().unwrap() = Some(r);
    });
    job
}

pub fn run(prj: &Project, out: &PathBuf, png_seq: bool, total: u32, done: &AtomicU32, cancel: &AtomicBool) -> Result<String, String> {
    let mut r = Renderer::new(prj.w, prj.h);
    let fps = prj.fps as f32;
    if png_seq {
        std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
        for i in 0..total {
            if cancel.load(Ordering::Relaxed) { return Err("Cancelled.".into()); }
            r.render(prj, i as f32 / fps).save_png(out.join(format!("frame_{:05}.png", i))).map_err(|e| e.to_string())?;
            done.store(i + 1, Ordering::Relaxed);
        }
        return Ok(format!("Saved {} PNG frames to {}", total, out.display()));
    }
    let wav = std::env::temp_dir().join(format!("tech_unwind_{}.wav", std::process::id()));
    let with_audio = audio::has_audio(prj);
    if with_audio { audio::write_wav(&wav, &audio::mixdown(prj)).map_err(|e| e.to_string())?; }
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-v", "error", "-f", "rawvideo", "-pix_fmt", "rgba", "-s", &format!("{}x{}", prj.w, prj.h), "-r", &prj.fps.to_string(), "-i", "-"]);
    if with_audio { cmd.arg("-i").arg(&wav); }
    cmd.args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-preset", "medium", "-crf", "17", "-movflags", "+faststart"]);
    if with_audio { cmd.args(["-c:a", "aac", "-b:a", "192k", "-shortest"]); }
    cmd.arg(out).stdin(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|_| "ffmpeg was not found. Install ffmpeg (ffmpeg.org) and make sure it is on your PATH, or export a PNG sequence instead.".to_string())?;
    let mut stdin = child.stdin.take().ok_or("Could not talk to ffmpeg")?;
    for i in 0..total {
        if cancel.load(Ordering::Relaxed) { drop(stdin); let _ = child.kill(); let _ = std::fs::remove_file(&wav); return Err("Cancelled.".into()); }
        let pm = r.render(prj, i as f32 / fps);
        if stdin.write_all(pm.data()).is_err() { break; }
        done.store(i + 1, Ordering::Relaxed);
    }
    drop(stdin);
    let o = child.wait_with_output().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&wav);
    if !o.status.success() { return Err(format!("ffmpeg stopped: {}", String::from_utf8_lossy(&o.stderr).trim())); }
    Ok(format!("Saved {}", out.display()))
}
