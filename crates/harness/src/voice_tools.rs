//! What the voice agent can do: change screens, keep the lists and the calendar, run timers, look things up, and hand bigger jobs to the builder.
//! Each tool is a small typed action; the model's input is checked here, not trusted.

use serde_json::{json, Value};

use crate::agent::Tools;
use crate::claude::ToolDef;
use crate::display::Display;
use crate::tasks::Tasks;
use crate::timers::Timers;

/// The names of the screens the display ships (a closed set: a request for any other is an error the model can read).
pub const SCREENS: [&str; 7] = ["home", "day", "weather", "system", "energy", "lists", "claude"];

pub struct VoiceTools<'a> {
    pub display: &'a Display,
    pub timers: &'a mut Timers,
    pub tasks: &'a Tasks,
    pub now_ms: &'a dyn Fn() -> u64,
}

fn def(name: &str, description: &str, props: Value, required: &[&str]) -> ToolDef {
    ToolDef { name: name.into(), description: description.into(), input_schema: json!({ "type": "object", "properties": props, "required": required }) }
}

fn str_arg<'v>(input: &'v Value, key: &str) -> Result<&'v str, String> {
    input[key].as_str().filter(|s| !s.trim().is_empty()).ok_or_else(|| format!("missing text argument `{key}`"))
}

fn command_result(r: Value) -> Result<String, String> {
    if r["ok"] == true {
        Ok(r["stdout"].as_str().unwrap_or("done").trim().to_string())
    } else {
        Err(r["error"].as_str().map(String::from).or_else(|| r["stderr"].as_str().map(String::from)).unwrap_or_else(|| "failed".into()))
    }
}

impl Tools for VoiceTools<'_> {
    fn defs(&self) -> Vec<ToolDef> {
        vec![
            def("show_screen", "Show one of the display's screens now (it returns to home by itself after a while).", json!({ "name": { "type": "string", "enum": SCREENS }, "stay_minutes": { "type": "integer", "description": "How long to stay; omit for the default." } }), &["name"]),
            def("set_timer", "Start a kitchen timer.", json!({ "seconds": { "type": "integer", "minimum": 1 }, "label": { "type": "string" } }), &["seconds", "label"]),
            def("cancel_timer", "Cancel a timer by its label, or the next one to ring if no label is given.", json!({ "label": { "type": "string" } }), &[]),
            def("shopping", "Change the shopping list.", json!({ "action": { "type": "string", "enum": ["add", "done", "remove", "clear_done"] }, "item": { "type": "string" }, "qty": { "type": "string" } }), &["action"]),
            def("note", "Keep a note, or finish one.", json!({ "action": { "type": "string", "enum": ["add", "done", "remove"] }, "text": { "type": "string" }, "due": { "type": "string" } }), &["action", "text"]),
            def("calendar", "Add, move or delete an event on the person's Google Calendar, or list the coming week.", json!({ "action": { "type": "string", "enum": ["list", "add", "move", "delete"] }, "title": { "type": "string" }, "date": { "type": "string", "description": "YYYY-MM-DD" }, "time": { "type": "string", "description": "HH:MM 24-hour, or `all` for an all-day event" }, "minutes": { "type": "integer" }, "id": { "type": "string", "description": "event id from a list, for move/delete" } }), &["action"]),
            def("look_up", "Search the display library's reference (components, typed keys, enums). Use before changing the display.", json!({ "query": { "type": "string" } }), &["query"]),
            def("display_state", "The live values the display has right now (weather, power, machine, calendar...).", json!({}), &[]),
            def("start_task", "Hand a bigger job to the builder agent (change a screen, add a feed, fix a scraper). It works in a sandbox and its change is checked before it goes live. Tell the person it is on the way.", json!({ "title": { "type": "string" }, "details": { "type": "string", "description": "What is wanted, precisely." } }), &["title", "details"]),
        ]
    }

    fn call(&mut self, name: &str, input: &Value) -> Result<String, String> {
        let now = (self.now_ms)();
        match name {
            "show_screen" => {
                let screen = str_arg(input, "name")?;
                if !SCREENS.contains(&screen) {
                    return Err(format!("no screen `{screen}`; the screens are {}", SCREENS.join(", ")));
                }
                let hold = input["stay_minutes"].as_u64().map(|m| m * 60_000);
                self.display.screen(screen, hold)?;
                Ok(format!("showing {screen}"))
            }
            "set_timer" => {
                let seconds = input["seconds"].as_u64().filter(|s| *s > 0).ok_or("seconds must be a positive integer")?;
                let label = str_arg(input, "label")?;
                let t = self.timers.start(now, seconds, label);
                self.display.push(&self.timers.to_push())?;
                self.display.screen("claude", Some(20_000))?;
                Ok(format!("timer `{}` set for {} seconds", t.label, t.total_s))
            }
            "cancel_timer" => {
                let cancelled = self.timers.cancel(input["label"].as_str().unwrap_or(""));
                self.display.push(&self.timers.to_push())?;
                Ok(if cancelled { "cancelled".into() } else { "no such timer".into() })
            }
            "shopping" => {
                let mut args = vec![match input["action"].as_str() {
                    Some("add") => "add",
                    Some("done") => "done",
                    Some("remove") => "remove",
                    Some("clear_done") => "clear-done",
                    _ => return Err("action must be add, done, remove or clear_done".into()),
                }
                .to_string()];
                if input["action"] != "clear_done" {
                    args.push(str_arg(input, "item")?.to_string());
                    if let (Some(q), "add") = (input["qty"].as_str(), args[0].as_str()) {
                        args.push(q.to_string());
                    }
                }
                command_result(self.display.run("shop", &args)?)
            }
            "note" => {
                let verb = match input["action"].as_str() {
                    Some(v @ ("add" | "done" | "remove")) => v,
                    _ => return Err("action must be add, done or remove".into()),
                };
                let mut args = vec![verb.to_string(), str_arg(input, "text")?.to_string()];
                if let (Some(d), "add") = (input["due"].as_str(), verb) {
                    args.push(d.to_string());
                }
                command_result(self.display.run("note", &args)?)
            }
            "calendar" => {
                let mut args: Vec<String> = Vec::new();
                match input["action"].as_str() {
                    Some("list") => args.extend(["list".into(), "7".into()]),
                    Some("add") => {
                        args.extend(["add".into(), str_arg(input, "title")?.into(), str_arg(input, "date")?.into(), str_arg(input, "time")?.into()]);
                        if let Some(m) = input["minutes"].as_u64() {
                            args.push(m.to_string());
                        }
                    }
                    Some("move") => args.extend(["move".into(), str_arg(input, "id")?.into(), str_arg(input, "date")?.into(), str_arg(input, "time")?.into()]),
                    Some("delete") => args.extend(["delete".into(), str_arg(input, "id")?.into()]),
                    _ => return Err("action must be list, add, move or delete".into()),
                }
                command_result(self.display.run("cal", &args)?)
            }
            "look_up" => {
                let items = self.display.docs(str_arg(input, "query")?)?;
                let brief: Vec<String> = items.as_array().map(|a| a.iter().take(8).map(|i| format!("{}\n{}\n{}", i["signature"].as_str().unwrap_or(""), i["doc"].as_str().unwrap_or(""), i["members"].as_array().map(|m| m.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default())).collect()).unwrap_or_default();
                Ok(if brief.is_empty() { "nothing found".into() } else { brief.join("\n---\n") })
            }
            "display_state" => {
                let data = self.display.data()?;
                // values only, and only the short ones: the model needs facts, not 48-slot arrays
                let mut out = serde_json::Map::new();
                if let Some(o) = data.as_object() {
                    for (k, v) in o {
                        let val = &v["value"];
                        if val.to_string().len() <= 160 {
                            out.insert(k.clone(), val.clone());
                        }
                    }
                }
                Ok(Value::Object(out).to_string())
            }
            "start_task" => {
                let t = self.tasks.enqueue(now, str_arg(input, "title")?, str_arg(input, "details")?)?;
                Ok(format!("queued task {}: {}", t.id, t.title))
            }
            other => Err(format!("no such tool `{other}`")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::testing::mock;

    fn with_tools<T>(f: impl FnOnce(&mut VoiceTools, &crate::display::testing::Mock) -> T) -> T {
        let m = mock();
        let display = Display::new(&m.base);
        let dir = std::env::temp_dir().join(format!("vt-{}-{}", std::process::id(), m.base.len() + (m.base.as_bytes()[m.base.len() - 1] as usize)));
        let tasks = Tasks::open(&dir).unwrap();
        let mut timers = Timers::default();
        let now = || 10_000u64;
        let mut tools = VoiceTools { display: &display, timers: &mut timers, tasks: &tasks, now_ms: &now };
        let out = f(&mut tools, &m);
        let _ = std::fs::remove_dir_all(dir);
        out
    }

    #[test]
    fn showing_a_screen_validates_the_name() {
        with_tools(|t, m| {
            assert!(t.call("show_screen", &json!({ "name": "energy", "stay_minutes": 10 })).is_ok());
            assert!(t.call("show_screen", &json!({ "name": "garden" })).unwrap_err().contains("the screens are"));
            assert_eq!(m.seen.lock().unwrap().iter().filter(|r| r.1.starts_with("/screen")).count(), 1);
            assert_eq!(m.seen.lock().unwrap()[0].1, "/screen?name=energy&hold=600000");
        });
    }

    #[test]
    fn a_timer_is_started_published_and_shown() {
        with_tools(|t, m| {
            t.call("set_timer", &json!({ "seconds": 390, "label": "egg" })).unwrap();
            let seen = m.seen.lock().unwrap();
            assert_eq!(seen[0].2, r#"{"timer.end":400000,"timer.label":"egg","timer.total":390}"#);
            assert!(seen[1].1.starts_with("/screen?name=claude"));
            drop(seen);
            assert!(t.call("set_timer", &json!({ "seconds": 0, "label": "x" })).is_err());
        });
    }

    #[test]
    fn lists_and_calendar_go_through_named_commands() {
        with_tools(|t, m| {
            let _ = t.call("shopping", &json!({ "action": "add", "item": "oat milk", "qty": "2" }));
            let _ = t.call("calendar", &json!({ "action": "add", "title": "Dentist", "date": "2026-11-02", "time": "15:30", "minutes": 45 }));
            let _ = t.call("note", &json!({ "action": "add", "text": "renew insurance", "due": "Fri" }));
            let seen = m.seen.lock().unwrap();
            assert_eq!(seen[0].1, "/run?cmd=shop&arg=add&arg=oat%20milk&arg=2");
            assert_eq!(seen[1].1, "/run?cmd=cal&arg=add&arg=Dentist&arg=2026%2D11%2D02&arg=15%3A30&arg=45");
            assert_eq!(seen[2].1, "/run?cmd=note&arg=add&arg=renew%20insurance&arg=Fri");
        });
    }

    #[test]
    fn a_bigger_job_becomes_a_queued_task() {
        with_tools(|t, _| {
            let r = t.call("start_task", &json!({ "title": "Add a garden screen", "details": "soil moisture from the sensor" })).unwrap();
            assert!(r.contains("queued task 1"), "{r}");
            assert!(t.call("start_task", &json!({ "title": "", "details": "x" })).is_err());
            assert!(t.call("nonsense", &json!({})).unwrap_err().contains("no such tool"));
        });
    }
}
