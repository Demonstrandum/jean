//! The harness: the agent that lives with the display. It hears (STT), thinks (Claude), acts on the display and the house through tools,
//! speaks (TTS), and does its own upkeep: a builder agent changes the display and its own code in a sandbox, behind a promotion pipeline.

pub mod agent;
pub mod claude;
pub mod display;
pub mod tasks;
pub mod timers;
pub mod voice_tools;
pub mod audio;
pub mod stt;
pub mod tts;
pub mod wake;
