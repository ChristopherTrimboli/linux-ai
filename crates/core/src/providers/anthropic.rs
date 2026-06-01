//! Anthropic Messages API provider with streaming.

use std::collections::HashMap;

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

const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        AnthropicProvider {
            base_url,
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

fn to_anthropic_messages(messages: &[Message]) -> Vec<Value> {
    let mut out = Vec::new();
    for m in messages {
        match m.role {
            Role::System => {
                // Folded into top-level `system`; emit as a user note if present.
                if !m.text().is_empty() {
                    out.push(json!({
                        "role": "user",
                        "content": [{ "type": "text", "text": m.text() }],
                    }));
                }
            }
            Role::User => {
                out.push(json!({
                    "role": "user",
                    "content": [{ "type": "text", "text": m.text() }],
                }));
            }
            Role::Assistant => {
                let blocks: Vec<Value> = m
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text { text } if !text.is_empty() => {
                            Some(json!({ "type": "text", "text": text }))
                        }
                        ContentBlock::ToolUse { id, name, input } => Some(json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input,
                        })),
                        _ => None,
                    })
                    .collect();
                if !blocks.is_empty() {
                    out.push(json!({ "role": "assistant", "content": blocks }));
                }
            }
            Role::Tool => {
                let blocks: Vec<Value> = m
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } => Some(json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": content,
                            "is_error": is_error,
                        })),
                        _ => None,
                    })
                    .collect();
                if !blocks.is_empty() {
                    out.push(json!({ "role": "user", "content": blocks }));
                }
            }
        }
    }
    out
}

fn to_anthropic_tools(req: &ChatRequest) -> Option<Value> {
    if req.tools.is_empty() {
        return None;
    }
    let tools: Vec<Value> = req
        .tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            })
        })
        .collect();
    Some(Value::Array(tools))
}

#[derive(Default)]
struct PartialBlock {
    is_tool: bool,
    id: String,
    name: String,
    json_acc: String,
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn chat_stream(
        &self,
        req: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatEvent>>> {
        let mut body = json!({
            "model": req.model,
            "max_tokens": req.max_tokens,
            "stream": true,
            "messages": to_anthropic_messages(&req.messages),
        });
        if let Some(sys) = &req.system {
            body["system"] = json!(sys);
        }
        if let Some(temp) = req.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(tools) = to_anthropic_tools(&req) {
            body["tools"] = tools;
        }

        let url = format!("{}/v1/messages", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

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
            let mut blocks: HashMap<usize, PartialBlock> = HashMap::new();

            while let Some(event) = stream.next().await {
                let event = match event {
                    Ok(e) => e,
                    Err(e) => {
                        let _ = tx.send(Err(Error::other(format!("stream error: {e}"))));
                        return;
                    }
                };
                let payload: Value = match serde_json::from_str(&event.data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                match event.event.as_str() {
                    "content_block_start" => {
                        let index = payload
                            .get("index")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;
                        let cb = payload.get("content_block");
                        let kind = cb
                            .and_then(|c| c.get("type"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("text");
                        let mut block = PartialBlock::default();
                        if kind == "tool_use" {
                            block.is_tool = true;
                            block.id = cb
                                .and_then(|c| c.get("id"))
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            block.name = cb
                                .and_then(|c| c.get("name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                        }
                        blocks.insert(index, block);
                    }
                    "content_block_delta" => {
                        let index = payload
                            .get("index")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;
                        let delta = payload.get("delta");
                        let dtype = delta
                            .and_then(|d| d.get("type"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("");
                        match dtype {
                            "text_delta" => {
                                if let Some(text) =
                                    delta.and_then(|d| d.get("text")).and_then(|v| v.as_str())
                                {
                                    let _ = tx.send(Ok(ChatEvent::TextDelta(text.to_string())));
                                }
                            }
                            "input_json_delta" => {
                                if let Some(partial) = delta
                                    .and_then(|d| d.get("partial_json"))
                                    .and_then(|v| v.as_str())
                                {
                                    if let Some(block) = blocks.get_mut(&index) {
                                        block.json_acc.push_str(partial);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    "content_block_stop" => {
                        let index = payload
                            .get("index")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize;
                        if let Some(block) = blocks.remove(&index) {
                            if block.is_tool {
                                let input: Value = serde_json::from_str(&block.json_acc)
                                    .unwrap_or_else(|_| json!({}));
                                let _ = tx.send(Ok(ChatEvent::ToolCall(ToolCallRequest {
                                    id: block.id,
                                    name: block.name,
                                    input,
                                })));
                            }
                        }
                    }
                    "message_start" => {
                        if let Some(input_tokens) = payload
                            .get("message")
                            .and_then(|m| m.get("usage"))
                            .and_then(|u| u.get("input_tokens"))
                            .and_then(|v| v.as_u64())
                        {
                            let _ = tx.send(Ok(ChatEvent::Usage(Usage {
                                input_tokens: input_tokens as u32,
                                output_tokens: 0,
                            })));
                        }
                    }
                    "message_delta" => {
                        if let Some(output_tokens) = payload
                            .get("usage")
                            .and_then(|u| u.get("output_tokens"))
                            .and_then(|v| v.as_u64())
                        {
                            let _ = tx.send(Ok(ChatEvent::Usage(Usage {
                                input_tokens: 0,
                                output_tokens: output_tokens as u32,
                            })));
                        }
                    }
                    "error" => {
                        let msg = payload
                            .get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown error");
                        let _ = tx.send(Err(Error::other(format!("anthropic: {msg}"))));
                        return;
                    }
                    _ => {}
                }
            }
        });

        Ok(UnboundedReceiverStream::new(rx).boxed())
    }
}
