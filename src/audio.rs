//! Mixing the audio clips, playing them, and writing WAV for export.
use crate::model::*;
use std::sync::Arc;

pub const SR: u32 = 48000;

pub fn mixdown(prj: &Project) -> Vec<f32> {
    let total = ((prj.duration() + 0.05) * SR as f32).ceil() as usize;
    let mut out = vec![0f32; total * 2];
    for c in prj.clips.iter().filter(|c| c.kind == Kind::Audio && !c.mute) {
        let Some(a) = &c.asset else { continue };
        let AssetData::Audio { sr: asr, ch } = &a.data else { continue };
        if ch.is_empty() { continue; }
        let n0 = (c.start * SR as f32) as usize;
        let n1 = ((c.end() * SR as f32) as usize).min(total);
        for n in n0..n1 {
            let lt = n as f32 / SR as f32 - c.start;
            let pos = (c.trim + lt) * *asr as f32;
            let i = pos as usize; let fr = pos - i as f32;
            let mut g = c.vol;
            if c.fade_in > 0.0 && lt < c.fade_in { g *= lt / c.fade_in; }
            let rem = c.dur - lt;
            if c.fade_out > 0.0 && rem < c.fade_out { g *= rem.max(0.0) / c.fade_out; }
            for k in 0..2 {
                let chn = &ch[k.min(ch.len() - 1)];
                if i + 1 >= chn.len() { continue; }
                out[n * 2 + k] += (chn[i] * (1.0 - fr) + chn[i + 1] * fr) * g;
            }
        }
    }
    for v in out.iter_mut() { *v = v.clamp(-1.0, 1.0); }
    out
}
pub fn has_audio(prj: &Project) -> bool { prj.clips.iter().any(|c| c.kind == Kind::Audio && !c.mute && c.asset.is_some()) }

pub fn write_wav(path: &std::path::Path, mix: &[f32]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    let data = (mix.len() * 2) as u32;
    f.write_all(b"RIFF")?; f.write_all(&(36 + data).to_le_bytes())?; f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?; f.write_all(&1u16.to_le_bytes())?; f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&SR.to_le_bytes())?; f.write_all(&(SR * 4).to_le_bytes())?; f.write_all(&4u16.to_le_bytes())?; f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?; f.write_all(&data.to_le_bytes())?;
    for v in mix { f.write_all(&((v * 32767.0) as i16).to_le_bytes())?; }
    Ok(())
}

pub struct Player { _stream: rodio::OutputStream, handle: rodio::OutputStreamHandle, sink: Option<rodio::Sink> }
impl Player {
    pub fn new() -> Option<Player> {
        let (s, h) = rodio::OutputStream::try_default().ok()?;
        Some(Player { _stream: s, handle: h, sink: None })
    }
    pub fn play(&mut self, mix: &Arc<Vec<f32>>, from: f32) {
        self.stop();
        let start = ((from.max(0.0) * SR as f32) as usize * 2).min(mix.len());
        if start >= mix.len() { return; }
        if let Ok(sink) = rodio::Sink::try_new(&self.handle) {
            sink.append(rodio::buffer::SamplesBuffer::new(2, SR, mix[start..].to_vec()));
            self.sink = Some(sink);
        }
    }
    pub fn stop(&mut self) { if let Some(s) = self.sink.take() { s.stop(); } }
}
