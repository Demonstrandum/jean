//! The loop that ties the senses to the session: listen, transcribe, answer aloud, ring timers, act on events.

use std::time::Duration;

use crate::audio::Ears;
use crate::session::Session;
use crate::stt::Stt;
use crate::tts::Speaker;
use crate::triggers::{Action, Event, MotionWatch, Outlook, Triggers};
use crate::wake::strip_wake;

pub struct Runner<'a> {
    pub ears: Box<dyn Ears + 'a>,
    pub stt: &'a dyn Stt,
    pub speaker: &'a dyn Speaker,
    pub session: Session<'a>,
    pub triggers: Triggers,
    pub motion: MotionWatch,
    /// Phrases that address the assistant; empty means everything heard is for it.
    pub wake: Vec<String>,
}

impl<'a> Runner<'a> {
    fn say(&self, text: &str) {
        if let Err(e) = self.speaker.speak(text) {
            eprintln!("jean: cannot speak: {e}");
        }
    }

    /// Wait up to `wait` for one utterance and deal with it. True if something was heard that was for us.
    pub fn hear_once(&mut self, wait: Duration, now_ms: u64) -> Result<bool, String> {
        let Some(rec) = self.ears.listen(wait)? else { return Ok(false) };
        self.session.transcribing(now_ms);
        let text = self.stt.transcribe(&rec);
        let _ = std::fs::remove_file(&rec);
        self.session.transcribed(text.is_ok());
        let text = text?;
        let request = if self.wake.is_empty() {
            Some(text.as_str())
        } else {
            let phrases: Vec<&str> = self.wake.iter().map(String::as_str).collect();
            strip_wake(&text, &phrases)
        };
        match request {
            Some(r) if !r.trim().is_empty() => {
                let reply = self.session.handle(r, now_ms).unwrap_or_else(|e| {
                    eprintln!("jean: {e}");
                    "Sorry, I couldn't reach my brain just now.".to_string()
                });
                self.say(&reply);
                self.session.idle(now_ms);
                Ok(true)
            }
            Some(_) => {
                self.say("Yes?");
                self.session.idle(now_ms);
                Ok(true)
            }
            None => {
                self.session.idle(now_ms);
                Ok(false)
            }
        }
    }

    /// Once a second: announce timers that rang.
    pub fn tick(&mut self, now_ms: u64) {
        for t in self.session.tick(now_ms) {
            let what = if t.label.is_empty() { "Your timer".to_string() } else { format!("Your {} timer", t.label) };
            self.say(&format!("{what} is done."));
        }
    }

    /// Look at the room's motion sensor (live data `room.motion`) and act on anything new.
    pub fn watch_sensors(&mut self, now_ms: u64) {
        let Ok(data) = self.session.display.data() else { return };
        if self.motion.observe(data["room.motion"]["value"].as_u64()) {
            let outlook = self.outlook();
            self.event(Event::Motion, outlook, now_ms);
        }
    }

    /// An event from a sensor: whatever the rules do about it is spoken.
    pub fn event(&mut self, ev: Event, outlook: Option<Outlook>, now_ms: u64) {
        for a in self.triggers.on(ev, outlook, now_ms) {
            match a {
                Action::Say(s) => self.say(&s),
            }
        }
    }

    /// The weather the rules care about, from the display's live data.
    pub fn outlook(&self) -> Option<Outlook> {
        let data = self.session.display.data().ok()?;
        let temp = data["wx.temp"]["value"].as_f64()?;
        Some(Outlook { rain_from: data["wx.rain_from"]["value"].as_u64(), temp })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::testing::{call, say, Script};
    use crate::display::testing::mock;
    use crate::display::Display;
    use crate::stt::TextStt;
    use crate::tasks::Tasks;
    use crate::tts::Recorder;
    use serde_json::json;
    use std::path::PathBuf;

    struct Queue(Vec<String>, PathBuf);
    impl Ears for Queue {
        fn listen(&mut self, _w: Duration) -> Result<Option<PathBuf>, String> {
            if self.0.is_empty() {
                return Ok(None);
            }
            let line = self.0.remove(0);
            let p = self.1.join(format!("u{}.txt", self.0.len()));
            std::fs::write(&p, line).map_err(|e| e.to_string())?;
            Ok(Some(p))
        }
    }

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("run-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn only_what_follows_the_wake_phrase_is_acted_on_and_the_answer_is_spoken() {
        let m = mock();
        let display = Display::new(&m.base);
        let d = dir("wake");
        let tasks = Tasks::open(d.join("tasks")).unwrap();
        let llm = Script::new(vec![call("a", "set_timer", json!({ "seconds": 60, "label": "tea" })), say("One minute, started.")]);
        let rec = Recorder::default();
        let mut r = Runner {
            ears: Box::new(Queue(vec!["we should get milk".into(), "Hey Claude, one minute timer for tea".into()], d.clone())),
            stt: &TextStt,
            speaker: &rec,
            session: Session::new(&llm, &display, &tasks, "m", chrono_tz::Europe::London, "London"),
            triggers: Triggers::new(chrono_tz::Europe::London),
            motion: MotionWatch::default(),
            wake: vec!["hey claude".into()],
        };
        assert!(!r.hear_once(Duration::ZERO, 1_000).unwrap());
        assert!(r.hear_once(Duration::ZERO, 2_000).unwrap());
        assert_eq!(rec.said(), vec!["One minute, started."]);
        // sixty seconds on, the timer rings and is announced
        r.tick(62_000);
        assert_eq!(rec.said().last().unwrap(), "Your tea timer is done.");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn a_morning_event_is_spoken() {
        let m = mock();
        let display = Display::new(&m.base);
        let d = dir("event");
        let tasks = Tasks::open(d.join("tasks")).unwrap();
        let llm = Script::new(vec![]);
        let rec = Recorder::default();
        let mut r = Runner {
            ears: Box::new(Queue(vec![], d.clone())),
            stt: &TextStt,
            speaker: &rec,
            session: Session::new(&llm, &display, &tasks, "m", chrono_tz::Europe::London, "London"),
            triggers: Triggers::new(chrono_tz::Europe::London),
            motion: MotionWatch::default(),
            wake: vec![],
        };
        r.event(Event::Motion, Some(Outlook { rain_from: None, temp: 2.0 }), 1_793_059_200_000 + 7 * 3_600_000);
        assert!(rec.said()[0].starts_with("Good morning.") && rec.said()[0].contains("jacket"));
        let _ = std::fs::remove_dir_all(d);
    }
}
