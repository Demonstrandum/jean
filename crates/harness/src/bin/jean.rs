//! The assistant. Configured by environment:
//!   ANTHROPIC_API_KEY   required, unless JEAN_LLM=cli (then the logged-in `claude` program is the model; JEAN_CLI_MODEL optional)
//!   JEAN_DISPLAY        display server (default http://127.0.0.1:8788)
//!   JEAN_MODE           `text` (read lines from stdin, print answers) or `voice` (microphone, whisper, speaker)
//!   JEAN_MODEL          the voice agent's model (default claude-haiku-5-5)
//!   JEAN_TZ, JEAN_PLACE timezone (default Europe/London) and place name
//!   JEAN_WAKE           wake phrases, comma separated (default "hey claude,hey jean"); empty = everything is for it
//!   JEAN_REPO           the project checkout the builder changes (enables the builder); JEAN_BUILDER_MODEL, JEAN_APPROVAL (auto|person), JEAN_RESTART (command run after a promotion)
//!   JEAN_DATA           state directory (default ./harness-state)
//!   JEAN_WHISPER_BIN, JEAN_WHISPER_MODEL, JEAN_WHISPER_URL   whisper.cpp CLI or server
//!   JEAN_TTS            say | piper | espeak | print; JEAN_PIPER_BIN, JEAN_PIPER_MODEL, JEAN_PLAYER

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use harness::audio::{Ears, SoxEars, TextEars};
use harness::audit::Audit;
use harness::claude::{Anthropic, ClaudeCli, Llm};
use harness::promote::{Checks, Pipeline};
use harness::sandbox::Backend;
use harness::work::{Approval, CommandLive, ScriptCanary, Worker};
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
    let llm: Box<dyn Llm + Sync> = if env("JEAN_LLM", "api") == "cli" || std::env::args().nth(1).as_deref() == Some("tasks") {
        Box::new(ClaudeCli { bin: env("JEAN_CLAUDE_BIN", "claude"), model: std::env::var("JEAN_CLI_MODEL").ok() })
    } else {
        Box::new(Anthropic::from_env().unwrap_or_else(|e| {
            eprintln!("jean: {e}");
            std::process::exit(2)
        }))
    };
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

    let repo = std::env::var("JEAN_REPO").ok().map(std::path::PathBuf::from);
    let approval = if env("JEAN_APPROVAL", "person") == "auto" { Approval::Auto } else { Approval::Person };
    // the builder: its own models, worktrees, pipeline and audit log. Built where it runs, because the pipeline's parts are not shared between threads.
    let with_builder = |f: &dyn Fn(&Worker)| {
        let repo = repo.as_ref().expect("JEAN_REPO is not set");
        let audit = Audit::open(data.join("audit.jsonl"));
        let backend = Backend::detect().expect("no sandbox (sandbox-exec or bwrap) on this machine");
        let live = CommandLive { display: &display, restart: env("JEAN_RESTART", "").split_whitespace().map(String::from).collect() };
        let checks = Checks(vec![vec!["cargo".into(), "test".into(), "--offline".into(), "-q".into()]]);
        let pipeline = Pipeline { live_repo: repo, audit: &audit, backend, checks, canary: &ScriptCanary, live: &live, probes: 6, probe_every: Duration::from_secs(10), sleep: &std::thread::sleep };
        let w = Worker { llm: llm.as_ref(), model: env("JEAN_BUILDER_MODEL", "claude-opus-5-5"), tasks: &tasks, audit: &audit, repo, scratch: data.join("trees"), display: &display, backend, pipeline: &pipeline, approval };
        f(&w);
    };
    match std::env::args().nth(1).as_deref() {
        Some("approve") => {
            let id: u64 = std::env::args().nth(2).and_then(|a| a.parse().ok()).expect("usage: jean approve <task id>");
            with_builder(&|w| match w.approve(id, now_ms()) {
                Ok(st) => println!("task {id}: {st:?}"),
                Err(e) => {
                    eprintln!("jean: {e}");
                    std::process::exit(1)
                }
            });
            return;
        }
        Some("tasks") => {
            for t in tasks.all().unwrap_or_default() {
                println!("#{} {:?} {} {}", t.id, t.status, t.title, t.result);
            }
            return;
        }
        Some(other) => {
            eprintln!("jean: unknown command {other} (commands: approve <id>, tasks)");
            std::process::exit(2)
        }
        None => {}
    }
    std::thread::scope(|scope| {
    if repo.is_some() {
        scope.spawn(|| {
            with_builder(&|w| loop {
                match w.run_next(now_ms()) {
                    Ok(Some((id, st))) => eprintln!("jean: builder task {id}: {st:?}"),
                    Ok(None) => std::thread::sleep(Duration::from_secs(5)),
                    Err(e) => {
                        eprintln!("jean: builder: {e}");
                        std::thread::sleep(Duration::from_secs(30));
                    }
                }
            })
        });
    }
    let session = Session::new(llm.as_ref(), &display, &tasks, &env("JEAN_MODEL", "claude-haiku-5-5"), tz, &env("JEAN_PLACE", "London"));
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
    std::process::exit(0); // the builder thread loops forever
    });
}
