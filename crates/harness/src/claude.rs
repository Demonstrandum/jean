//! A small client for the Anthropic Messages API, behind a trait so the agent loop is testable without a network.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// One block of a message's content.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
    ToolResult { tool_use_id: String, content: String, #[serde(default, skip_serializing_if = "std::ops::Not::not")] is_error: bool },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<Block>,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Message {
        Message { role: Role::User, content: vec![Block::Text { text: text.into() }] }
    }
}

/// A tool the model may call: its name, what it is for, and a JSON schema for its input.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct Request {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDef>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Response {
    pub content: Vec<Block>,
    #[serde(default)]
    pub stop_reason: String,
}

impl Response {
    /// The text blocks, joined.
    pub fn text(&self) -> String {
        self.content.iter().filter_map(|b| if let Block::Text { text } = b { Some(text.as_str()) } else { None }).collect::<Vec<_>>().join("")
    }
    pub fn tool_uses(&self) -> Vec<(&str, &str, &Value)> {
        self.content.iter().filter_map(|b| if let Block::ToolUse { id, name, input } = b { Some((id.as_str(), name.as_str(), input)) } else { None }).collect()
    }
}

/// Anything that can answer a request: the real API, or a script in a test.
pub trait Llm {
    fn complete(&self, req: &Request) -> Result<Response, String>;
}

/// The real thing. `base_url` is overridable so tests can point it at a local server.
pub struct Anthropic {
    pub api_key: String,
    pub base_url: String,
}

impl Anthropic {
    pub fn new(api_key: impl Into<String>) -> Anthropic {
        Anthropic { api_key: api_key.into(), base_url: "https://api.anthropic.com".into() }
    }

    /// From `ANTHROPIC_API_KEY` (and `ANTHROPIC_BASE_URL` if set).
    pub fn from_env() -> Result<Anthropic, String> {
        let key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| "ANTHROPIC_API_KEY is not set".to_string())?;
        let mut a = Anthropic::new(key);
        if let Ok(u) = std::env::var("ANTHROPIC_BASE_URL") {
            a.base_url = u;
        }
        Ok(a)
    }
}

impl Llm for Anthropic {
    fn complete(&self, req: &Request) -> Result<Response, String> {
        let resp = ureq::post(&format!("{}/v1/messages", self.base_url.trim_end_matches('/')))
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", "2023-06-01")
            .set("content-type", "application/json")
            .timeout(std::time::Duration::from_secs(120))
            .send_json(json!(req));
        match resp {
            Ok(r) => r.into_json::<Response>().map_err(|e| format!("bad response: {e}")),
            Err(ureq::Error::Status(code, r)) => Err(format!("API {code}: {}", r.into_string().unwrap_or_default())),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// A model reached through the `claude` command-line program (`claude -p`), for machines with a login but no API key. It has no native tool calls,
/// so the tools and the conversation are written into the prompt and the model answers with one JSON object: `{"text": "...", "tool_uses": [{"name": "...", "input": {...}}]}`.
pub struct ClaudeCli {
    pub bin: String,
    pub model: Option<String>,
}

impl ClaudeCli {
    fn prompt(req: &Request) -> String {
        let mut p = format!("{}\n\nYou may call these tools (JSON schemas):\n", req.system);
        for t in &req.tools {
            p.push_str(&format!("- {}: {} schema={}\n", t.name, t.description, t.input_schema));
        }
        p.push_str("\nReply with ONE JSON object and nothing else: {\"text\": \"what to say, or empty\", \"tool_uses\": [{\"name\": \"...\", \"input\": {...}}]}. Use tool_uses to act; once the tool results are in, give the final words, which must not be empty, in text with no tool_uses.\n\nConversation so far:\n");
        for m in &req.messages {
            let who = if m.role == Role::User { "USER" } else { "YOU" };
            for b in &m.content {
                match b {
                    Block::Text { text } => p.push_str(&format!("{who}: {text}\n")),
                    Block::ToolUse { name, input, .. } => p.push_str(&format!("YOU CALLED {name} {input}\n")),
                    Block::ToolResult { content, is_error, .. } => p.push_str(&format!("TOOL RESULT{}: {content}\n", if *is_error { " (error)" } else { "" })),
                }
            }
        }
        p
    }

    /// The first complete JSON object in the model's reply.
    fn object(reply: &str) -> Option<Value> {
        let start = reply.find('{')?;
        serde_json::Deserializer::from_str(&reply[start..]).into_iter::<Value>().next()?.ok()
    }
}

impl Llm for ClaudeCli {
    fn complete(&self, req: &Request) -> Result<Response, String> {
        let mut c = std::process::Command::new(&self.bin);
        c.args(["-p", "--output-format", "json", "--tools", "", "--no-session-persistence"]);
        if let Some(m) = &self.model {
            c.args(["--model", m]);
        }
        let mut child = c.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().map_err(|e| format!("cannot run {}: {e}", self.bin))?;
        std::io::Write::write_all(&mut child.stdin.take().unwrap(), Self::prompt(req).as_bytes()).map_err(|e| e.to_string())?;
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!("{} failed: {}", self.bin, String::from_utf8_lossy(&out.stderr).trim()));
        }
        let wrapper: Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("bad output from {}: {e}", self.bin))?;
        let reply = wrapper["result"].as_str().ok_or("no result in the reply")?;
        let Some(obj) = Self::object(reply) else {
            return Ok(Response { content: vec![Block::Text { text: reply.trim().to_string() }], stop_reason: "end_turn".into() });
        };
        let mut content = Vec::new();
        if let Some(t) = obj["text"].as_str().filter(|t| !t.is_empty()) {
            content.push(Block::Text { text: t.to_string() });
        }
        let uses = obj["tool_uses"].as_array().cloned().unwrap_or_default();
        for (i, u) in uses.iter().enumerate() {
            content.push(Block::ToolUse { id: format!("cli-{}-{i}", req.messages.len()), name: u["name"].as_str().unwrap_or("").to_string(), input: u["input"].clone() });
        }
        Ok(Response { content, stop_reason: if uses.is_empty() { "end_turn" } else { "tool_use" }.into() })
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use std::cell::RefCell;

    /// An `Llm` that plays back scripted responses and records what it was asked.
    pub struct Script {
        pub replies: RefCell<Vec<Response>>,
        pub seen: RefCell<Vec<Request>>,
    }

    impl Script {
        pub fn new(replies: Vec<Response>) -> Script {
            Script { replies: RefCell::new(replies.into_iter().rev().collect()), seen: RefCell::new(vec![]) }
        }
    }

    impl Llm for Script {
        fn complete(&self, req: &Request) -> Result<Response, String> {
            self.seen.borrow_mut().push(req.clone());
            self.replies.borrow_mut().pop().ok_or_else(|| "script exhausted".to_string())
        }
    }

    pub fn say(text: &str) -> Response {
        Response { content: vec![Block::Text { text: text.into() }], stop_reason: "end_turn".into() }
    }

    pub fn call(id: &str, name: &str, input: Value) -> Response {
        Response { content: vec![Block::ToolUse { id: id.into(), name: name.into(), input }], stop_reason: "tool_use".into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_round_trip_in_the_apis_shape() {
        let r: Response = serde_json::from_value(json!({
            "content": [{ "type": "text", "text": "Hi" }, { "type": "tool_use", "id": "t1", "name": "docs", "input": { "q": "tile" } }],
            "stop_reason": "tool_use"
        }))
        .unwrap();
        assert_eq!(r.text(), "Hi");
        assert_eq!(r.tool_uses().len(), 1);
        let m = Message { role: Role::User, content: vec![Block::ToolResult { tool_use_id: "t1".into(), content: "ok".into(), is_error: false }] };
        assert_eq!(serde_json::to_value(&m).unwrap(), json!({ "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "t1", "content": "ok" }] }));
    }

    #[test]
    fn the_client_talks_to_a_server_with_the_right_headers() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        let t = std::thread::spawn(move || {
            let mut req = server.recv().unwrap();
            let key = req.headers().iter().find(|h| h.field.equiv("x-api-key")).map(|h| h.value.to_string());
            let ver = req.headers().iter().find(|h| h.field.equiv("anthropic-version")).map(|h| h.value.to_string());
            let mut body = String::new();
            req.as_reader().read_to_string(&mut body).unwrap();
            req.respond(tiny_http::Response::from_string(r#"{"content":[{"type":"text","text":"pong"}],"stop_reason":"end_turn"}"#)).unwrap();
            (key, ver, body)
        });
        let mut a = Anthropic::new("sk-test");
        a.base_url = format!("http://127.0.0.1:{port}");
        let r = a.complete(&Request { model: "m".into(), max_tokens: 10, system: "s".into(), messages: vec![Message::user("ping")], tools: vec![] }).unwrap();
        assert_eq!(r.text(), "pong");
        let (key, ver, body) = t.join().unwrap();
        assert_eq!(key.as_deref(), Some("sk-test"));
        assert_eq!(ver.as_deref(), Some("2023-06-01"));
        assert!(body.contains("\"ping\"") && !body.contains("\"tools\""), "{body}");
    }
}
