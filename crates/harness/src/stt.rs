//! Speech to text: whisper.cpp as a command or as its HTTP server, and plain text files for developing without a microphone.

use std::path::Path;
use std::process::Command;

pub trait Stt {
    fn transcribe(&self, recording: &Path) -> Result<String, String>;
}

/// `whisper-cli -m <model> -f <wav> -nt` (whisper.cpp): the transcript is what it prints.
pub struct WhisperCli {
    pub bin: String,
    pub model: String,
    pub language: String,
}

impl Stt for WhisperCli {
    fn transcribe(&self, wav: &Path) -> Result<String, String> {
        let out = Command::new(&self.bin)
            .args(["-m", &self.model, "-f"])
            .arg(wav)
            .args(["-nt", "-np", "-l", &self.language])
            .output()
            .map_err(|e| format!("cannot run {}: {e}", self.bin))?;
        if !out.status.success() {
            return Err(format!("{} failed: {}", self.bin, String::from_utf8_lossy(&out.stderr).trim()));
        }
        Ok(clean_transcript(&String::from_utf8_lossy(&out.stdout)))
    }
}

/// whisper.cpp's HTTP server (`whisper-server`): POST /inference with the WAV as a multipart `file`, the answer is `{"text": "..."}`.
pub struct WhisperServer {
    pub url: String,
}

impl Stt for WhisperServer {
    fn transcribe(&self, wav: &Path) -> Result<String, String> {
        let bytes = std::fs::read(wav).map_err(|e| e.to_string())?;
        let boundary = "----jeanboundary7e3f";
        let mut body: Vec<u8> = Vec::new();
        body.extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"response_format\"\r\n\r\njson\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\nContent-Type: audio/wav\r\n\r\n").as_bytes());
        body.extend(&bytes);
        body.extend(format!("\r\n--{boundary}--\r\n").as_bytes());
        let resp = ureq::post(&format!("{}/inference", self.url.trim_end_matches('/')))
            .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
            .timeout(std::time::Duration::from_secs(60))
            .send_bytes(&body)
            .map_err(|e| e.to_string())?;
        let v: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
        Ok(clean_transcript(v["text"].as_str().unwrap_or("")))
    }
}

/// A recording that is already text (what `TextEars` makes).
pub struct TextStt;

impl Stt for TextStt {
    fn transcribe(&self, recording: &Path) -> Result<String, String> {
        std::fs::read_to_string(recording).map(|s| s.trim().to_string()).map_err(|e| e.to_string())
    }
}

/// Text files go to `TextStt`, audio to the real recogniser.
pub struct AutoStt<S: Stt>(pub S);

impl<S: Stt> Stt for AutoStt<S> {
    fn transcribe(&self, recording: &Path) -> Result<String, String> {
        if crate::audio::is_text_recording(recording) { TextStt.transcribe(recording) } else { self.0.transcribe(recording) }
    }
}

/// Whisper marks silence and noise in brackets ("[BLANK_AUDIO]", "(music)"): those are not speech.
pub fn clean_transcript(raw: &str) -> String {
    let joined = raw.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    let mut depth = 0usize;
    for c in joined.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn noise_markers_are_not_speech() {
        assert_eq!(clean_transcript(" [BLANK_AUDIO]\n"), "");
        assert_eq!(clean_transcript("Hey Claude, (music) what time is it? [noise]"), "Hey Claude, what time is it?");
        assert_eq!(clean_transcript("line one\n\n line two "), "line one line two");
    }

    #[test]
    fn the_cli_backend_runs_the_binary_with_the_wav_and_cleans_its_output() {
        let dir = std::env::temp_dir().join(format!("stt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("fake-whisper");
        std::fs::write(&bin, "#!/bin/sh\n# args: -m model -f wav -nt -np -l en\n[ \"$1\" = -m ] && [ \"$3\" = -f ] || exit 2\necho \" Hey Claude, set a timer for ten minutes. [BLANK_AUDIO]\"\n").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let wav = dir.join("a.wav");
        std::fs::write(&wav, b"RIFF").unwrap();
        let stt = WhisperCli { bin: bin.to_string_lossy().into(), model: "m.bin".into(), language: "en".into() };
        assert_eq!(stt.transcribe(&wav).unwrap(), "Hey Claude, set a timer for ten minutes.");
        let missing = WhisperCli { bin: "/nonexistent/whisper".into(), model: "m".into(), language: "en".into() };
        assert!(missing.transcribe(&wav).is_err());
        let txt = dir.join("t.txt");
        std::fs::write(&txt, "typed words\n").unwrap();
        assert_eq!(AutoStt(missing).transcribe(&txt).unwrap(), "typed words");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_server_backend_posts_a_multipart_wav() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        let t = std::thread::spawn(move || {
            let mut req = server.recv().unwrap();
            let ct = req.headers().iter().find(|h| h.field.equiv("content-type")).unwrap().value.to_string();
            let mut body = Vec::new();
            std::io::Read::read_to_end(req.as_reader(), &mut body).unwrap();
            let url = req.url().to_string();
            req.respond(tiny_http::Response::from_string(r#"{"text": " turn the lights off"}"#)).unwrap();
            (url, ct, body)
        });
        let wav = std::env::temp_dir().join(format!("srv-{}.wav", std::process::id()));
        std::fs::write(&wav, b"RIFFdata").unwrap();
        let text = WhisperServer { url: format!("http://127.0.0.1:{port}") }.transcribe(&wav).unwrap();
        assert_eq!(text, "turn the lights off");
        let (url, ct, body) = t.join().unwrap();
        assert_eq!(url, "/inference");
        assert!(ct.starts_with("multipart/form-data; boundary="));
        assert!(body.windows(8).any(|w| w == b"RIFFdata"));
        let _ = std::fs::remove_file(wav);
    }
}
