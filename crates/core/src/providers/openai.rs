//! OpenAI Chat Completions compatible provider. Works with OpenAI as well as
//! any compatible endpoint (Ollama, LM Studio, vLLM, ...) via a custom base URL.

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::stream::BoxStream;
use futures::StreamExt;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::error::{Error, Result};
use crate::message::{ContentBlock, Message, Role};

use super::{ChatEvent, ChatRequest, Provider, ToolCallRequest, Usage};

pub struct OpenAiProvider {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        OpenAiProvider {
            base_url,
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

/// Convert our message model into OpenAI's `messages` array.
fn to_openai_messages(system: &Option<String>, messages: &[Message]) -> Vec<Value> {
    let mut out = Vec::new();
    if let Some(sys) = system {
        out.push(json!({ "role": "system", "content": sys }));
    }
    for m in messages {
        match m.role {
            Role::System => {
                out.push(json!({ "role": "system", "content": m.text() }));
            }
            Role::User => {
                out.push(json!({ "role": "user", "content": m.text() }));
            }
            Role::Assistant => {
                let mut tool_calls = Vec::new();
                for b in &m.content {
                    if let ContentBlock::ToolUse { id, name, input } = b {
                        tool_calls.push(json!({
                            "id": id,
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": serde_json::to_string(input).unwrap_or_default(),
                            }
                        }));
                    }
                }
                let mut msg = json!({ "role": "assistant", "content": m.text() });
                if !tool_calls.is_empty() {
                    msg["tool_calls"] = Value::Array(tool_calls);
                }
                out.push(msg);
            }
            Role::Tool => {
                for b in &m.content {
                    if let ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } = b
                    {
                        out.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_use_id,
                            "content": content,
                        }));
                    }
                }
            }
        }
    }
    out
}

fn to_openai_tools(req: &ChatRequest) -> Option<Value> {
    if req.tools.is_empty() {
        return None;
    }
    let tools: Vec<Value> = req
        .tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                }
            })
        })
        .collect();
    Some(Value::Array(tools))
}

/// Accumulates a streaming tool call across deltas.
#[derive(Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

#[async_trait]
impl Provider for OpenAiProvider {
    async fn chat_stream(
        &self,
        req: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatEvent>>> {
        let mut body = json!({
            "model": req.model,
            "messages": to_openai_messages(&req.system, &req.messages),
            "stream": true,
            "max_tokens": req.max_tokens,
            "stream_options": { "include_usage": true },
        });
        if let Some(temp) = req.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(tools) = to_openai_tools(&req) {
            body["tools"] = tools;
        }

        let url = format!("{}/chat/completions", self.base_url);
        let mut request = self.client.post(&url).json(&body);
        if let Some(key) = &self.api_key {
            request = request.bearer_auth(key);
        }

        let resp = request.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Provider {
                status: status.as_u16(),
                body: text,
            });
        }

        let (tx, rx) = mpsc::unbounded_channel::<Result<ChatEvent>>();
        tokio::spawn(async move {
            let mut stream = resp.bytes_stream().eventsource();
            // tool calls accumulated by stream index
            let mut tool_calls: Vec<PartialToolCall> = Vec::new();

            while let Some(event) = stream.next().await {
                let event = match event {
                    Ok(e) => e,
                    Err(e) => {
                        let _ = tx.send(Err(Error::other(format!("stream error: {e}"))));
                        return;
                    }
                };
                let data = event.data;
                if data == "[DONE]" {
                    break;
                }
                let chunk: Value = match serde_json::from_str(&data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if let Some(usage) = chunk.get("usage").filter(|u| !u.is_null()) {
                    let _ = tx.send(Ok(ChatEvent::Usage(Usage {
                        input_tokens: usage
                            .get("prompt_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32,
                        output_tokens: usage
                            .get("completion_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32,
                    })));
                }

                let Some(choice) = chunk.get("choices").and_then(|c| c.get(0)) else {
                    continue;
                };
                let delta = choice.get("delta");

                if let Some(content) = delta
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                {
                    if !content.is_empty() {
                        let _ = tx.send(Ok(ChatEvent::TextDelta(content.to_string())));
                    }
                }

                if let Some(calls) = delta
                    .and_then(|d| d.get("tool_calls"))
                    .and_then(|c| c.as_array())
                {
                    for call in calls {
                        let index =
                            call.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                        while tool_calls.len() <= index {
                            tool_calls.push(PartialToolCall::default());
                        }
                        let slot = &mut tool_calls[index];
                        if let Some(id) = call.get("id").and_then(|v| v.as_str()) {
                            slot.id = id.to_string();
                        }
                        if let Some(func) = call.get("function") {
                            if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                slot.name.push_str(name);
                            }
                            if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                slot.arguments.push_str(args);
                            }
                        }
                    }
                }
            }

            for call in tool_calls {
                if call.name.is_empty() {
                    continue;
                }
                let input: Value = serde_json::from_str(&call.arguments).unwrap_or(json!({}));
                let id = if call.id.is_empty() {
                    uuid::Uuid::new_v4().to_string()
                } else {
                    call.id
                };
                let _ = tx.send(Ok(ChatEvent::ToolCall(ToolCallRequest {
                    id,
                    name: call.name,
                    input,
                })));
            }
        });

        Ok(UnboundedReceiverStream::new(rx).boxed())
    }
}
