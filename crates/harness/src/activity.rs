//! What the assistant is doing, as the display shows it live: the steps of a request (transcribing, each tool call as it starts and ends), published as the
//! typed key `claude.steps` next to `claude.state`, so the Claude tile can turn black and narrate while the work happens.

use std::cell::RefCell;

use serde::Serialize;
use serde_json::Value;

use crate::agent::Watch;
use crate::display::Display;

/// One step, in words for the display: `ok` is None while it runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Step {
    pub tool: String,
    pub detail: String,
    pub ok: Option<bool>,
}

/// Steps kept on the display: the oldest drop off.
const KEEP: usize = 8;

#[derive(Clone, Debug, Default)]
pub struct Activity {
    steps: Vec<Step>,
}

impl Activity {
    pub fn clear(&mut self) {
        self.steps.clear();
    }

    pub fn begin(&mut self, tool: &str, detail: &str) {
        self.steps.push(Step { tool: tool.into(), detail: detail.into(), ok: None });
        if self.steps.len() > KEEP {
            self.steps.remove(0);
        }
    }

    /// The most recent step that is still running ends, well or not.
    pub fn end(&mut self, ok: bool) {
        if let Some(s) = self.steps.iter_mut().rev().find(|s| s.ok.is_none()) {
            s.ok = Some(ok);
        }
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

/// A tool's input in a few words: its short scalar values, in order. For the display only; nothing reads it back.
pub fn summarize(input: &Value) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(o) = input.as_object() {
        for v in o.values() {
            match v {
                Value::String(s) if !s.is_empty() => parts.push(s.chars().take(28).collect()),
                Value::Number(n) => parts.push(n.to_string()),
                Value::Bool(b) => parts.push(b.to_string()),
                _ => {}
            }
        }
    }
    parts.truncate(3);
    parts.join(" · ")
}

/// Publishes an activity to the display as the agent loop reports each tool.
pub struct Feed<'a> {
    pub display: &'a Display,
    pub activity: &'a RefCell<Activity>,
}

impl Feed<'_> {
    pub fn publish(&self) {
        let _ = self.display.show_steps(&self.activity.borrow());
    }
}

impl Watch for Feed<'_> {
    fn begin(&self, tool: &str, input: &Value) {
        self.activity.borrow_mut().begin(tool, &summarize(input));
        self.publish();
    }

    fn end(&self, ok: bool) {
        self.activity.borrow_mut().end(ok);
        self.publish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_tool_call_is_a_step_that_starts_open_and_ends_closed() {
        let mut a = Activity::default();
        a.begin("set_timer", &summarize(&json!({ "seconds": 390, "label": "egg" })));
        assert_eq!(a.steps(), &[Step { tool: "set_timer".into(), detail: "egg · 390".into(), ok: None }]);
        a.end(true);
        assert_eq!(a.steps()[0].ok, Some(true));
        a.begin("show_screen", "weather");
        a.end(false);
        assert_eq!(a.steps()[1].ok, Some(false));
    }

    #[test]
    fn only_the_last_steps_are_kept_and_long_text_is_cut() {
        let mut a = Activity::default();
        for i in 0..12 {
            a.begin("t", &i.to_string());
        }
        assert_eq!(a.steps().len(), KEEP);
        assert_eq!(a.steps()[0].detail, "4");
        assert!(summarize(&json!({ "text": "x".repeat(100) })).chars().count() <= 28);
        assert_eq!(summarize(&json!({ "a": 1, "b": 2, "c": 3, "d": 4 })), "1 · 2 · 3");
    }
}
