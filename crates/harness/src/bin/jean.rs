//! The assistant. Configured by environment:
//!   ANTHROPIC_API_KEY   required
//!   JEAN_DISPLAY        display server (default http://127.0.0.1:8788)
//!   JEAN_MODE           `text` (read lines from stdin, print answers) or `voice` (microphone, whisper, speaker)
//!   JEAN_MODEL          the voice agent's model (default claude-haiku-5-5)
//!   JEAN_TZ, JEAN_PLACE timezone (default Europe/London) and place name
//!   JEAN_WAKE           wake phrases, comma separated (default "hey claude,hey jean"); empty = everything is for it
//!   JEAN_DATA           state directory (default ./harness-state)
//!   JEAN_WHISPER_BIN, JEAN_WHISPER_MODEL, JEAN_WHISPER_URL   whisper.cpp CLI or server
//!   JEAN_TTS            say | piper | espeak | print; JEAN_PIPER_BIN, JEAN_PIPER_MODEL, JEAN_PLAYER

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use harness::audio::{Ears, SoxEars, TextEars};
use harness::claude::Anthropic;
use harness::display::Display;
use harness::run::Runner;
use harness::session::Session;
use harness::stt::{AutoStt, Stt, WhisperCli, WhisperServer};
use harness::tasks::Tasks;
use harness::triggers::Triggers;
use harness::tts::{Espeak, MacSay, Piper, Print, Speaker};

fn env(k: &str, default: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| default.to_string())
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

fn main() {
    let llm = Anthropic::from_env().unwrap_or_else(|e| {
        eprintln!("jean: {e}");
        std::process::exit(2)
    });
    let display = Display::new(env("JEAN_DISPLAY", "http://127.0.0.1:8788"));
    let data = std::path::PathBuf::from(env("JEAN_DATA", "harness-state"));
    let tasks = Tasks::open(data.join("tasks")).expect("cannot open the task queue");
    let tz: chrono_tz::Tz = env("JEAN_TZ", "Europe/London").parse().expect("unknown JEAN_TZ");
    let voice = env("JEAN_MODE", "text") == "voice";

    let stt: Box<dyn Stt> = if !env("JEAN_WHISPER_URL", "").is_empty() {
        Box::new(AutoStt(WhisperServer { url: env("JEAN_WHISPER_URL", "") }))
    } else {
        Box::new(AutoStt(WhisperCli { bin: env("JEAN_WHISPER_BIN", "whisper-cli"), model: env("JEAN_WHISPER_MODEL", "models/ggml-base.en.bin"), language: "en".into() }))
    };
    let speaker: Box<dyn Speaker> = match env("JEAN_TTS", if voice { "say" } else { "print" }).as_str() {
        "say" => Box::new(MacSay { voice: None }),
        "piper" => Box::new(Piper {
            bin: env("JEAN_PIPER_BIN", "piper"),
            model: env("JEAN_PIPER_MODEL", "models/en_GB-alba-medium.onnx"),
            player: env("JEAN_PLAYER", "aplay -q").split_whitespace().map(String::from).collect(),
            tmp: data.join("speech.wav"),
        }),
        "espeak" => Box::new(Espeak),
        _ => Box::new(Print),
    };
    let ears: Box<dyn Ears> = if voice { Box::new(SoxEars::new(data.join("heard"))) } else { Box::new(TextEars::new(data.join("heard"))) };
    let wake: Vec<String> = env("JEAN_WAKE", if voice { "hey claude,hey jean" } else { "" }).split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

    let session = Session::new(&llm, &display, &tasks, &env("JEAN_MODEL", "claude-haiku-5-5"), tz, &env("JEAN_PLACE", "London"));
    let mut runner = Runner { ears, stt: stt.as_ref(), speaker: speaker.as_ref(), session, triggers: Triggers::new(tz), wake };
    if !display.healthy() {
        eprintln!("jean: the display is not answering yet; carrying on");
    }
    loop {
        runner.tick(now_ms());
        match runner.hear_once(Duration::from_secs(1), now_ms()) {
            Ok(_) => {}
            Err(e) if e == "end of input" => break,
            Err(e) => {
                eprintln!("jean: {e}");
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    }
}
