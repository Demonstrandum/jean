//! Listening: a voice-activity detector over a stream of audio energy, a WAV writer, and the microphone (via `rec` from sox).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const SAMPLE_RATE: u32 = 16_000;

/// What the detector saw in the latest chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vad {
    /// Nothing yet.
    Quiet,
    /// Speech has begun.
    Started,
    /// Speech is going on.
    Speaking,
    /// Speech was followed by enough silence: the utterance is complete.
    Ended,
    /// Speech was too short to be an utterance (a click, a cough): ignore it.
    Discard,
}

/// Energy-threshold voice-activity detection with hangover: speech starts when a chunk is louder than `threshold`, and ends after
/// `hangover` consecutive quiet chunks; very short bursts are discarded.
pub struct Detector {
    pub threshold: f32,
    pub hangover: usize,
    pub min_speech: usize,
    speaking: bool,
    loud: usize,
    quiet_run: usize,
}

impl Detector {
    pub fn new(threshold: f32, hangover: usize, min_speech: usize) -> Detector {
        Detector { threshold, hangover, min_speech, speaking: false, loud: 0, quiet_run: 0 }
    }

    pub fn push(&mut self, rms: f32) -> Vad {
        let loud = rms >= self.threshold;
        if !self.speaking {
            if loud {
                self.speaking = true;
                self.loud = 1;
                self.quiet_run = 0;
                return Vad::Started;
            }
            return Vad::Quiet;
        }
        if loud {
            self.loud += 1;
            self.quiet_run = 0;
            return Vad::Speaking;
        }
        self.quiet_run += 1;
        if self.quiet_run >= self.hangover {
            let long_enough = self.loud >= self.min_speech;
            self.speaking = false;
            self.loud = 0;
            self.quiet_run = 0;
            return if long_enough { Vad::Ended } else { Vad::Discard };
        }
        Vad::Speaking
    }
}

/// Root-mean-square energy of 16-bit little-endian samples, 0..1.
pub fn rms(pcm: &[u8]) -> f32 {
    let n = pcm.len() / 2;
    if n == 0 {
        return 0.0;
    }
    let sum: f64 = pcm.chunks_exact(2).map(|b| { let s = i16::from_le_bytes([b[0], b[1]]) as f64 / 32768.0; s * s }).sum();
    (sum / n as f64).sqrt() as f32
}

/// A mono 16-bit WAV file's bytes around raw PCM.
pub fn wav_bytes(pcm: &[u8], rate: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(44 + pcm.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

/// Something that can wait for a person to say something and hand back the recording.
pub trait Ears {
    /// Wait up to `wait` for speech; the recording of one utterance as a WAV file, or None if nobody spoke.
    fn listen(&mut self, wait: Duration) -> Result<Option<PathBuf>, String>;
}

/// The microphone through sox's `rec`, streamed as raw 16 kHz mono PCM in 100 ms chunks and cut into utterances by the detector.
pub struct SoxEars {
    pub dir: PathBuf,
    pub detector: Detector,
    child: Option<Child>,
    counter: u64,
}

impl SoxEars {
    pub fn new(dir: impl Into<PathBuf>) -> SoxEars {
        SoxEars { dir: dir.into(), detector: Detector::new(0.02, 8, 3), child: None, counter: 0 }
    }

    fn mic(&mut self) -> Result<&mut Child, String> {
        if self.child.is_none() {
            let c = Command::new("rec")
                .args(["-q", "-r", &SAMPLE_RATE.to_string(), "-c", "1", "-b", "16", "-e", "signed-integer", "-t", "raw", "-"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("cannot start the microphone (sox `rec`): {e}"))?;
            self.child = Some(c);
        }
        Ok(self.child.as_mut().unwrap())
    }
}

impl Ears for SoxEars {
    fn listen(&mut self, wait: Duration) -> Result<Option<PathBuf>, String> {
        let started = Instant::now();
        let chunk_bytes = (SAMPLE_RATE as usize / 10) * 2; // 100 ms
        let mut pcm: Vec<u8> = Vec::new();
        let mut speaking = false;
        let mut preroll: std::collections::VecDeque<Vec<u8>> = Default::default();
        loop {
            let mut buf = vec![0u8; chunk_bytes];
            self.mic()?.stdout.as_mut().unwrap().read_exact(&mut buf).map_err(|e| format!("microphone stream ended: {e}"))?;
            match self.detector.push(rms(&buf)) {
                Vad::Quiet => {
                    preroll.push_back(buf);
                    if preroll.len() > 3 {
                        preroll.pop_front(); // keep a little lead-in so the first syllable is not clipped
                    }
                    if started.elapsed() >= wait && !speaking {
                        return Ok(None);
                    }
                }
                Vad::Started => {
                    speaking = true;
                    for p in preroll.drain(..) {
                        pcm.extend(p);
                    }
                    pcm.extend(buf);
                }
                Vad::Speaking => pcm.extend(buf),
                Vad::Discard => {
                    speaking = false;
                    pcm.clear();
                }
                Vad::Ended => {
                    pcm.extend(buf);
                    self.counter += 1;
                    std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
                    let path = self.dir.join(format!("utterance-{}.wav", self.counter));
                    std::fs::write(&path, wav_bytes(&pcm, SAMPLE_RATE)).map_err(|e| e.to_string())?;
                    return Ok(Some(path));
                }
            }
        }
    }
}

impl Drop for SoxEars {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// Reads utterances from standard input, one line each: for developing without a microphone. The "recording" is a text file.
pub struct TextEars {
    pub dir: PathBuf,
    counter: u64,
}

impl TextEars {
    pub fn new(dir: impl Into<PathBuf>) -> TextEars {
        TextEars { dir: dir.into(), counter: 0 }
    }
}

impl Ears for TextEars {
    fn listen(&mut self, _wait: Duration) -> Result<Option<PathBuf>, String> {
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Err("end of input".into());
        }
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        self.counter += 1;
        let path = self.dir.join(format!("typed-{}.txt", self.counter));
        std::fs::write(&path, line.trim()).map_err(|e| e.to_string())?;
        Ok(Some(path))
    }
}

pub fn is_text_recording(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()) == Some("txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(d: &mut Detector, seq: &[f32]) -> Vec<Vad> {
        seq.iter().map(|&e| d.push(e)).collect()
    }

    #[test]
    fn speech_followed_by_silence_is_one_utterance() {
        let mut d = Detector::new(0.02, 3, 2);
        let out = run(&mut d, &[0.0, 0.0, 0.1, 0.2, 0.15, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(out, vec![Vad::Quiet, Vad::Quiet, Vad::Started, Vad::Speaking, Vad::Speaking, Vad::Speaking, Vad::Speaking, Vad::Ended, Vad::Quiet]);
    }

    #[test]
    fn a_short_burst_is_discarded_and_a_pause_inside_speech_is_not_the_end() {
        let mut d = Detector::new(0.02, 3, 3);
        assert_eq!(run(&mut d, &[0.5, 0.0, 0.0, 0.0]).last(), Some(&Vad::Discard));
        let out = run(&mut d, &[0.3, 0.3, 0.0, 0.3, 0.3, 0.0, 0.0, 0.0]);
        assert_eq!(out.last(), Some(&Vad::Ended));
        assert!(!out[..out.len() - 1].contains(&Vad::Ended), "a pause shorter than the hangover does not end it");
    }

    #[test]
    fn energy_and_wav_header() {
        let loud: Vec<u8> = (0..160).flat_map(|i| (if i % 2 == 0 { 16384i16 } else { -16384 }).to_le_bytes()).collect();
        assert!((rms(&loud) - 0.5).abs() < 1e-3);
        assert_eq!(rms(&[0u8; 320]), 0.0);
        let w = wav_bytes(&loud, 16_000);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes([w[24], w[25], w[26], w[27]]), 16_000);
        assert_eq!(w.len(), 44 + loud.len());
    }
}
