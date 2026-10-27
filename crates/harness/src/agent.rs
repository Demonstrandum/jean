//! The agent loop: the model answers, asks for tools, gets their results, and answers again, until it stops asking.

use serde_json::Value;

use crate::claude::{Block, Llm, Message, Request, Role, ToolDef};

/// The tools an agent has: what they are, and how to run one. A tool's failure is returned to the model as an error result, not raised.
pub trait Tools {
    fn defs(&self) -> Vec<ToolDef>;
    fn call(&mut self, name: &str, input: &Value) -> Result<String, String>;
}

/// How an agent is set up: which model, what it is told, and how far it may go in one turn.
#[derive(Clone, Debug)]
pub struct Profile {
    pub model: String,
    pub system: String,
    pub max_tokens: u32,
    /// Most model calls in one turn (a runaway loop is cut off).
    pub max_steps: usize,
}

/// What a turn produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The final words (empty if the model only used tools).
    pub text: String,
    /// The tools it called, in order.
    pub calls: Vec<String>,
    /// True if it was cut off at `max_steps`.
    pub truncated: bool,
}

pub struct Agent<'a> {
    pub llm: &'a dyn Llm,
    pub profile: Profile,
}

impl<'a> Agent<'a> {
    /// One user turn. `history` is extended in place, so the conversation carries on from it.
    pub fn run_turn(&self, tools: &mut dyn Tools, history: &mut Vec<Message>, user: &str) -> Result<Outcome, String> {
        history.push(Message::user(user));
        let defs = tools.defs();
        let mut calls = Vec::new();
        for _ in 0..self.profile.max_steps {
            let req = Request { model: self.profile.model.clone(), max_tokens: self.profile.max_tokens, system: self.profile.system.clone(), messages: history.clone(), tools: defs.clone() };
            let resp = self.llm.complete(&req)?;
            history.push(Message { role: Role::Assistant, content: resp.content.clone() });
            let uses = resp.tool_uses();
            if uses.is_empty() {
                return Ok(Outcome { text: resp.text(), calls, truncated: false });
            }
            let mut results = Vec::new();
            for (id, name, input) in uses {
                calls.push(name.to_string());
                let (content, is_error) = match tools.call(name, input) {
                    Ok(s) => (s, false),
                    Err(e) => (e, true),
                };
                results.push(Block::ToolResult { tool_use_id: id.to_string(), content, is_error });
            }
            history.push(Message { role: Role::User, content: results });
        }
        Ok(Outcome { text: String::new(), calls, truncated: true })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::testing::{call, say, Script};
    use serde_json::json;

    struct Fake {
        log: Vec<String>,
    }
    impl Tools for Fake {
        fn defs(&self) -> Vec<ToolDef> {
            vec![ToolDef { name: "echo".into(), description: "echoes".into(), input_schema: json!({ "type": "object" }) }]
        }
        fn call(&mut self, name: &str, input: &Value) -> Result<String, String> {
            self.log.push(format!("{name} {input}"));
            if input["fail"] == true { Err("boom".into()) } else { Ok("done".into()) }
        }
    }

    fn profile(steps: usize) -> Profile {
        Profile { model: "m".into(), system: "sys".into(), max_tokens: 100, max_steps: steps }
    }

    #[test]
    fn a_tool_call_then_an_answer() {
        let llm = Script::new(vec![call("t1", "echo", json!({ "x": 1 })), say("All set.")]);
        let agent = Agent { llm: &llm, profile: profile(5) };
        let mut tools = Fake { log: vec![] };
        let mut history = vec![];
        let out = agent.run_turn(&mut tools, &mut history, "do it").unwrap();
        assert_eq!(out, Outcome { text: "All set.".into(), calls: vec!["echo".into()], truncated: false });
        assert_eq!(tools.log, vec![r#"echo {"x":1}"#]);
        // user, assistant(tool_use), user(tool_result), assistant(text)
        assert_eq!(history.len(), 4);
        assert!(matches!(&history[2].content[0], Block::ToolResult { content, is_error: false, .. } if content == "done"));
        // the second request carried the tool result
        assert_eq!(llm.seen.borrow()[1].messages.len(), 3);
    }

    #[test]
    fn a_failing_tool_is_reported_to_the_model_not_raised() {
        let llm = Script::new(vec![call("t1", "echo", json!({ "fail": true })), say("It failed.")]);
        let agent = Agent { llm: &llm, profile: profile(5) };
        let mut history = vec![];
        let out = agent.run_turn(&mut Fake { log: vec![] }, &mut history, "go").unwrap();
        assert_eq!(out.text, "It failed.");
        assert!(matches!(&history[2].content[0], Block::ToolResult { content, is_error: true, .. } if content == "boom"));
    }

    #[test]
    fn a_runaway_loop_is_cut_off() {
        let llm = Script::new((0..10).map(|i| call(&format!("t{i}"), "echo", json!({}))).collect());
        let agent = Agent { llm: &llm, profile: profile(3) };
        let out = agent.run_turn(&mut Fake { log: vec![] }, &mut vec![], "loop").unwrap();
        assert!(out.truncated);
        assert_eq!(out.calls.len(), 3);
    }
}
