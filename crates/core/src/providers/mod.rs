//! Provider abstraction. Each provider streams a normalized sequence of
//! [`ChatEvent`]s so the agent loop is provider-agnostic. Providers accumulate
//! partial tool-call arguments internally and only emit a [`ChatEvent::ToolCall`]
//! once the arguments are complete.

mod anthropic;
mod openai;

use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use crate::config::{ProviderConfig, ProviderKind};
use crate::error::Result;
use crate::message::Message;

pub use anthropic::AnthropicProvider;
pub use openai::OpenAiProvider;

/// Description of a tool exposed to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// A fully-assembled tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Normalized streaming event emitted by any provider.
#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// Incremental assistant text.
    TextDelta(String),
    /// A complete tool call.
    ToolCall(ToolCallRequest),
    /// Token usage (may arrive near the end of the stream).
    Usage(Usage),
}

pub struct ChatRequest {
    pub model: String,
    pub system: Option<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSpec>,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat_stream(
        &self,
        req: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatEvent>>>;
}

/// Build a provider trait object from configuration + resolved API key.
pub fn build_provider(cfg: &ProviderConfig, api_key: Option<String>) -> Box<dyn Provider> {
    let base_url = cfg.effective_base_url();
    match cfg.kind {
        ProviderKind::Openai => Box::new(OpenAiProvider::new(base_url, api_key)),
        ProviderKind::Anthropic => {
            Box::new(AnthropicProvider::new(base_url, api_key.unwrap_or_default()))
        }
    }
}
