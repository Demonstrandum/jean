//! Text to speech: Piper (the NixOS box), macOS `say` (development), espeak-ng (fallback), and a recorder for tests.

use std::io::Write;
use std::process::{Command, Stdio};

pub trait Speaker {
    /// Say it aloud; returns when it has been said.
    fn speak(&self, text: &str) -> Result<(), String>;
}

/// macOS's own voice: `say`.
pub struct MacSay {
    pub voice: Option<String>,
}

impl Speaker for MacSay {
    fn speak(&self, text: &str) -> Result<(), String> {
        let mut c = Command::new("say");
        if let Some(v) = &self.voice {
            c.args(["-v", v]);
        }
        let st = c.arg(text).status().map_err(|e| format!("cannot run say: {e}"))?;
        if st.success() { Ok(()) } else { Err("say failed".into()) }
    }
}

/// Piper (https://github.com/rhasspy/piper): the text goes to its stdin, it writes a WAV, a player plays it.
pub struct Piper {
    pub bin: String,
    pub model: String,
    /// `["afplay"]`, `["aplay", "-q"]`, `["play", "-q"]`...: the WAV path is appended.
    pub player: Vec<String>,
    pub tmp: std::path::PathBuf,
}

impl Speaker for Piper {
    fn speak(&self, text: &str) -> Result<(), String> {
        std::fs::create_dir_all(&self.tmp).map_err(|e| e.to_string())?;
        let wav = self.tmp.join("speech.wav");
        let mut child = Command::new(&self.bin)
            .args(["--model", &self.model, "--output_file"])
            .arg(&wav)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("cannot run {}: {e}", self.bin))?;
        child.stdin.take().unwrap().write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        if !child.wait().map_err(|e| e.to_string())?.success() {
            return Err("piper failed".into());
        }
        let (prog, args) = self.player.split_first().ok_or("no player configured")?;
        let st = Command::new(prog).args(args).arg(&wav).status().map_err(|e| format!("cannot run {prog}: {e}"))?;
        if st.success() { Ok(()) } else { Err(format!("{prog} failed")) }
    }
}

/// espeak-ng: plain and everywhere.
pub struct Espeak;

impl Speaker for Espeak {
    fn speak(&self, text: &str) -> Result<(), String> {
        let st = Command::new("espeak-ng").arg(text).status().map_err(|e| format!("cannot run espeak-ng: {e}"))?;
        if st.success() { Ok(()) } else { Err("espeak-ng failed".into()) }
    }
}

/// Prints what would be said (development without sound).
pub struct Print;

impl Speaker for Print {
    fn speak(&self, text: &str) -> Result<(), String> {
        println!("🔊 {text}");
        Ok(())
    }
}

/// Remembers what was said, for tests.
#[derive(Default)]
pub struct Recorder {
    pub said: std::cell::RefCell<Vec<String>>,
}

impl Speaker for Recorder {
    fn speak(&self, text: &str) -> Result<(), String> {
        self.said.borrow_mut().push(text.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn script(dir: &std::path::Path, name: &str, body: &str) -> String {
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p.to_string_lossy().into()
    }

    #[test]
    fn piper_gets_the_text_on_stdin_and_the_player_gets_the_wav() {
        let dir = std::env::temp_dir().join(format!("tts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("log");
        // fake piper: args are --model M --output_file F; it writes stdin into F
        let piper = script(&dir, "piper", "cat > \"$4\"");
        let player = script(&dir, "player", &format!("cp \"$1\" {}", log.display()));
        let s = Piper { bin: piper, model: "voice.onnx".into(), player: vec![player], tmp: dir.join("tmp") };
        s.speak("Your egg is ready.").unwrap();
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "Your egg is ready.");
        assert!(Piper { bin: "/nonexistent/piper".into(), model: "m".into(), player: vec!["true".into()], tmp: dir.join("t2") }.speak("x").is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_recorder_remembers() {
        let r = Recorder::default();
        r.speak("a").unwrap();
        r.speak("b").unwrap();
        assert_eq!(*r.said.borrow(), vec!["a", "b"]);
    }
}
