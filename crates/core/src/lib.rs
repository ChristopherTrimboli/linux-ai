//! `la-core`: the shared engine behind the Linux AI Companion desktop app and
//! CLI. It provides a multi-provider LLM abstraction, a built-in tool system
//! with risk-based approval, an agent loop, and local SQLite persistence.

pub mod agent;
pub mod config;
pub mod error;
pub mod message;
pub mod providers;
pub mod secrets;
pub mod store;
pub mod tools;
pub mod transcribe;

pub use agent::{Agent, AgentEvent, DEFAULT_SYSTEM};
pub use config::{Config, ProviderConfig, ProviderKind, SttConfig, ToolPolicy};
pub use transcribe::transcribe;
pub use error::{Error, Result};
pub use message::{ContentBlock, Message, Role};
pub use store::{Conversation, Store, StoredMessage};
pub use tools::{
    ApprovalDecision, ApprovalRequest, Approver, Risk, Tool, ToolContext, ToolRegistry,
};
