//! A conversation with the household: what was heard goes to the agent, the answer is shown and spoken, and the timers are kept.

use std::cell::{Cell, RefCell};

use chrono::{TimeZone, Utc};
use chrono_tz::Tz;

use crate::activity::{Activity, Feed};
use crate::agent::{Agent, Profile};
use crate::claude::{Block, Llm, Message, Role};
use crate::display::Display;
use crate::prompts;
use crate::tasks::Tasks;
use crate::timers::{Timer, Timers};
use crate::voice_tools::VoiceTools;

/// What the assistant is doing: the integers the display's `Presence` enum uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Presence {
    Idle = 0,
    Listening = 1,
    Thinking = 2,
    Speaking = 3,
    /// On a background job (the builder).
    Working = 4,
}

/// How many user turns of conversation are kept in the history.
pub const KEEP_TURNS: usize = 12;

pub struct Session<'a> {
    pub llm: &'a dyn Llm,
    pub display: &'a Display,
    pub tasks: &'a Tasks,
    pub timers: Timers,
    pub history: Vec<Message>,
    pub model: String,
    pub tz: Tz,
    pub place: String,
    /// The steps of the request in hand, as the display shows them live.
    activity: RefCell<Activity>,
    /// A transcription step is already on the display for the request that is coming.
    fresh: Cell<bool>,
}

impl<'a> Session<'a> {
    pub fn new(llm: &'a dyn Llm, display: &'a Display, tasks: &'a Tasks, model: &str, tz: Tz, place: &str) -> Session<'a> {
        Session { llm, display, tasks, timers: Timers::default(), history: vec![], model: model.into(), tz, place: place.into(), activity: RefCell::new(Activity::default()), fresh: Cell::new(false) }
    }

    /// The facts of the moment the agent is told fresh each turn.
    pub fn context(&self, now_ms: u64) -> String {
        let t = Utc.timestamp_millis_opt(now_ms as i64).unwrap().with_timezone(&self.tz);
        format!("It is {} on {} in {}. The timezone is {}.", t.format("%H:%M"), t.format("%A %-d %B %Y"), self.place, self.tz)
    }

    fn set_presence(&self, p: Presence, now_ms: u64) {
        let _ = self.display.show_exchange(None, None, p as u8, now_ms);
    }

    pub fn listening(&self, now_ms: u64) {
        self.set_presence(Presence::Listening, now_ms);
    }

    /// A recording is being turned into words: the display shows it as the first step of the request.
    pub fn transcribing(&self, now_ms: u64) {
        self.fresh.set(true);
        {
            let mut a = self.activity.borrow_mut();
            a.clear();
            a.begin("hear", "transcribing");
        }
        let _ = self.display.show_exchange(Some(""), None, Presence::Listening as u8, now_ms);
        Feed { display: self.display, activity: &self.activity }.publish();
    }

    pub fn transcribed(&self, ok: bool) {
        self.activity.borrow_mut().end(ok);
        Feed { display: self.display, activity: &self.activity }.publish();
    }

    pub fn idle(&self, now_ms: u64) {
        self.set_presence(Presence::Idle, now_ms);
    }

    /// One request, start to finish: show what was heard, let the agent work, show what it says. The reply is returned to be spoken.
    pub fn handle(&mut self, heard: &str, now_ms: u64) -> Result<String, String> {
        let _ = self.display.show_exchange(Some(heard), None, Presence::Thinking as u8, now_ms);
        if !self.fresh.replace(false) {
            self.activity.borrow_mut().clear(); // a typed request: no transcription step came before it
        }
        let feed = Feed { display: self.display, activity: &self.activity };
        feed.publish();
        let profile = Profile { model: self.model.clone(), system: prompts::voice_system(&self.context(now_ms)), max_tokens: 600, max_steps: 8 };
        let agent = Agent { llm: self.llm, profile, watch: Some(&feed) };
        let clock = move || now_ms;
        let outcome = {
            let mut tools = VoiceTools { display: self.display, timers: &mut self.timers, tasks: self.tasks, now_ms: &clock };
            agent.run_turn(&mut tools, &mut self.history, heard)
        };
        trim_history(&mut self.history, KEEP_TURNS);
        let reply = match outcome {
            Ok(o) if !o.text.trim().is_empty() => o.text.trim().to_string(),
            Ok(o) if o.truncated => "Sorry, that got away from me.".to_string(),
            Ok(_) => "Done.".to_string(),
            Err(e) => {
                let _ = self.display.show_exchange(None, Some("Sorry, I couldn't do that."), Presence::Speaking as u8, now_ms);
                return Err(e);
            }
        };
        let _ = self.display.show_exchange(None, Some(&reply), Presence::Speaking as u8, now_ms);
        Ok(reply)
    }

    /// Called about once a second: the timers that have rung (to be announced), and the display's timer shown up to date.
    pub fn tick(&mut self, now_ms: u64) -> Vec<Timer> {
        let rang = self.timers.due(now_ms);
        if !rang.is_empty() {
            let _ = self.display.push(&self.timers.to_push());
        }
        rang
    }
}

/// Keep the last `turns` user turns, cutting only at a turn boundary (a user text message), so a tool call never loses its result.
pub fn trim_history(history: &mut Vec<Message>, turns: usize) {
    let starts: Vec<usize> = history
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == Role::User && m.content.iter().all(|b| matches!(b, Block::Text { .. })))
        .map(|(i, _)| i)
        .collect();
    if starts.len() > turns {
        let cut = starts[starts.len() - turns];
        history.drain(..cut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::testing::{call, say, Script};
    use crate::display::testing::mock;
    use serde_json::json;

    fn tasks() -> Tasks {
        Tasks::open(std::env::temp_dir().join(format!("sess-{}-{:?}", std::process::id(), std::thread::current().id()))).unwrap()
    }

    #[test]
    fn a_request_shows_thinking_acts_and_shows_the_answer() {
        let m = mock();
        let display = Display::new(&m.base);
        let tasks = tasks();
        let llm = Script::new(vec![call("t1", "set_timer", json!({ "seconds": 390, "label": "egg" })), say("Timer set for six and a half minutes.")]);
        let mut s = Session::new(&llm, &display, &tasks, "m", chrono_tz::Europe::London, "London");
        let reply = s.handle("set an egg timer for six and a half minutes", 1_793_000_000_000).unwrap();
        assert_eq!(reply, "Timer set for six and a half minutes.");
        assert_eq!(s.timers.len(), 1);
        let seen = m.seen.lock().unwrap();
        let pushes: Vec<&str> = seen.iter().filter(|r| r.1 == "/push").map(|r| r.2.as_str()).collect();
        assert!(pushes[0].contains("\"claude.heard\":\"set an egg timer") && pushes[0].contains("\"claude.state\":2"), "{pushes:?}");
        assert!(pushes.iter().any(|p| p.contains("timer.end")));
        assert!(pushes.last().unwrap().contains("\"claude.say\":\"Timer set") && pushes.last().unwrap().contains("\"claude.state\":3"));
        // the tool call went to the display as a step that started open and ended closed
        let steps: Vec<&&str> = pushes.iter().filter(|p| p.contains("claude.steps")).collect();
        assert!(steps.iter().any(|p| p.contains(r#""tool":"set_timer""#) && p.contains(r#""ok":null"#)), "{steps:?}");
        assert!(steps.last().unwrap().contains(r#""detail":"egg · 390""#) && steps.last().unwrap().contains(r#""ok":true"#), "{steps:?}");
        // the agent was told the time and the place
        assert!(llm.seen.borrow()[0].system.contains("in London"));
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join(format!("sess-{}-{:?}", std::process::id(), std::thread::current().id())));
    }

    #[test]
    fn a_timer_that_rings_is_announced_once_and_the_display_updated() {
        let m = mock();
        let display = Display::new(&m.base);
        let tasks = tasks();
        let llm = Script::new(vec![]);
        let mut s = Session::new(&llm, &display, &tasks, "m", chrono_tz::Europe::London, "London");
        s.timers.start(0, 5, "tea");
        assert!(s.tick(1_000).is_empty());
        let rang = s.tick(5_000);
        assert_eq!(rang.len(), 1);
        assert_eq!(rang[0].label, "tea");
        assert!(s.tick(6_000).is_empty());
        assert!(m.seen.lock().unwrap().iter().any(|r| r.2.contains("\"timer.end\":null")));
    }

    #[test]
    fn history_is_trimmed_at_a_turn_boundary() {
        let mut h = Vec::new();
        for i in 0..5 {
            h.push(Message::user(format!("q{i}")));
            h.push(Message { role: Role::Assistant, content: vec![Block::ToolUse { id: format!("t{i}"), name: "x".into(), input: json!({}) }] });
            h.push(Message { role: Role::User, content: vec![Block::ToolResult { tool_use_id: format!("t{i}"), content: "ok".into(), is_error: false }] });
            h.push(Message { role: Role::Assistant, content: vec![Block::Text { text: format!("a{i}") }] });
        }
        trim_history(&mut h, 2);
        assert_eq!(h.len(), 8);
        assert!(matches!(&h[0].content[0], Block::Text { text } if text == "q3"));
    }

    #[test]
    fn the_context_names_the_local_time() {
        let m = mock();
        let display = Display::new(&m.base);
        let tasks = tasks();
        let llm = Script::new(vec![]);
        let s = Session::new(&llm, &display, &tasks, "m", chrono_tz::Europe::London, "London");
        // 2026-10-27 00:30 UTC is 00:30 in London (GMT after the clocks went back)
        assert_eq!(s.context(1_793_061_000_000), "It is 00:30 on Tuesday 27 October 2026 in London. The timezone is Europe/London.");
    }
}
