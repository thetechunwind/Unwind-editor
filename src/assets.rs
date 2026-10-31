//! Loading images and audio. Audio goes through symphonia; anything it cannot read falls back to ffmpeg if installed.
use crate::model::*;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SErr;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

const IMG: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];

pub fn load(path: &Path) -> Result<Arc<Asset>, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".into());
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !path.exists() { return Err(format!("{} was not found.", path.display())); }
    if IMG.contains(&ext.as_str()) {
        let im = image::open(path).map_err(|e| format!("Could not open {}: {}", name, e))?.to_rgba8();
        let (w, h) = im.dimensions();
        let mut pm = tiny_skia::Pixmap::new(w, h).ok_or("Image is empty")?;
        for (d, s) in pm.data_mut().chunks_exact_mut(4).zip(im.as_raw().chunks_exact(4)) {
            let a = s[3] as u32;
            d[0] = ((s[0] as u32 * a + 127) / 255) as u8; d[1] = ((s[1] as u32 * a + 127) / 255) as u8; d[2] = ((s[2] as u32 * a + 127) / 255) as u8; d[3] = s[3];
        }
        return Ok(Arc::new(Asset { name, data: AssetData::Image(pm) }));
    }
    let (sr, ch) = decode_audio(path).map_err(|e| format!("Could not read {} as audio. {}", name, e))?;
    Ok(Arc::new(Asset { name, data: AssetData::Audio { sr, ch } }))
}

pub fn decode_audio(path: &Path) -> Result<(u32, Vec<Vec<f32>>), String> {
    match decode_symphonia(path) {
        Ok(v) => Ok(v),
        Err(e1) => decode_ffmpeg(path).map_err(|e2| format!("({}; {})", e1, e2)),
    }
}

fn decode_symphonia(path: &Path) -> Result<(u32, Vec<Vec<f32>>), String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(f), Default::default());
    let mut hint = Hint::new();
    if let Some(e) = path.extension().and_then(|e| e.to_str()) { hint.with_extension(e); }
    let probed = symphonia::default::get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default()).map_err(|e| format!("not a format this app reads directly ({})", e))?;
    let mut fmt = probed.format;
    let track = fmt.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL).ok_or("no audio track")?;
    let id = track.id;
    let mut dec = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).map_err(|e| format!("codec not supported ({})", e))?;
    let mut sr = track.codec_params.sample_rate.unwrap_or(44100);
    let mut chs: Vec<Vec<f32>> = vec![];
    loop {
        let pkt = match fmt.next_packet() {
            Ok(p) => p,
            Err(SErr::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SErr::ResetRequired) => break,
            Err(e) => { if chs.is_empty() { return Err(e.to_string()); } else { break; } }
        };
        if pkt.track_id() != id { continue; }
        match dec.decode(&pkt) {
            Ok(buf) => {
                let spec = *buf.spec(); sr = spec.rate; let n = spec.channels.count(); let frames = buf.frames();
                if chs.is_empty() { chs = vec![vec![]; n]; }
                let mut sb = SampleBuffer::<f32>::new(buf.capacity() as u64, spec);
                sb.copy_planar_ref(buf);
                let s = sb.samples();
                for c in 0..n.min(chs.len()) { chs[c].extend_from_slice(&s[c * frames..(c + 1) * frames]); }
            }
            Err(SErr::DecodeError(_)) => continue,
            Err(e) => { if chs.is_empty() { return Err(e.to_string()); } else { break; } }
        }
    }
    if chs.is_empty() || chs[0].is_empty() { return Err("no audio found".into()); }
    Ok((sr, chs))
}

fn decode_ffmpeg(path: &Path) -> Result<(u32, Vec<Vec<f32>>), String> {
    let out = Command::new("ffmpeg").args(["-v", "error", "-i"]).arg(path).args(["-vn", "-f", "f32le", "-ac", "2", "-ar", "48000", "-"]).output()
        .map_err(|_| "ffmpeg is not installed, so other formats cannot be converted. Install ffmpeg or convert the file to WAV or MP3".to_string())?;
    if !out.status.success() || out.stdout.len() < 16 { return Err("ffmpeg could not read it either".into()); }
    let f: Vec<f32> = out.stdout.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
    let (mut l, mut r) = (Vec::with_capacity(f.len() / 2), Vec::with_capacity(f.len() / 2));
    for p in f.chunks_exact(2) { l.push(p[0]); r.push(p[1]); }
    Ok((48000, vec![l, r]))
}
