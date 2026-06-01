//! The agent loop ties providers, tools, and persistence together. It streams
//! assistant text, runs any requested tools (gated by approval), feeds results
//! back to the model, and repeats until the model stops calling tools.

use tokio::sync::mpsc::UnboundedSender;

use crate::config::Config;
use crate::error::Result;
use crate::message::{ContentBlock, Message, Role};
use crate::providers::{build_provider, ChatEvent, ChatRequest, Provider, Usage};
use crate::secrets;
use crate::store::Store;
use crate::tools::{
    auto_approver, ApprovalDecision, ApprovalRequest, Approver, Risk, ToolContext, ToolRegistry,
};

pub const DEFAULT_SYSTEM: &str = "You are Linux AI Companion, a helpful assistant embedded in the user's Linux desktop. \
You can inspect and act on the user's computer through the provided tools (reading and writing files, \
listing directories, searching, querying system info, running shell commands, and opening files/URLs). \
Prefer the dedicated file tools over shell when possible. Be concise. Explain what you are about to do \
before destructive actions. When a tool returns an error, adapt rather than repeating the same call.";

/// Events streamed to a client (CLI or desktop) during a turn. Serialized with
/// an adjacent `type`/`data` tag so the desktop frontend can discriminate.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AgentEvent {
    TextDelta(String),
    ToolStarted {
        id: String,
        name: String,
        summary: String,
    },
    ToolCompleted {
        id: String,
        name: String,
        output: String,
        is_error: bool,
    },
    /// Emitted by a client's approver (not the loop itself) to request approval.
    ApprovalRequired {
        id: String,
        tool: String,
        summary: String,
    },
    Usage(Usage),
    Done,
    Error(String),
}

pub struct Agent {
    provider: Box<dyn Provider>,
    registry: ToolRegistry,
    ctx: ToolContext,
    store: Store,
    model: String,
    system: String,
    max_tokens: u32,
    auto_approve: bool,
    max_iterations: usize,
}

impl Agent {
    /// Build an agent from configuration, choosing a provider/model (with
    /// optional overrides) and resolving the API key.
    pub fn from_config(
        cfg: &Config,
        store: Store,
        provider_override: Option<&str>,
        model_override: Option<&str>,
    ) -> Result<Agent> {
        let provider_name = provider_override.unwrap_or(&cfg.default_provider);
        let provider_cfg = cfg.provider(provider_name)?;
        let api_key = secrets::resolve_api_key(provider_name, provider_cfg);
        let provider = build_provider(provider_cfg, api_key);

        let model = model_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| cfg.default_model.clone());

        Ok(Agent {
            provider,
            registry: ToolRegistry::builtin(),
            ctx: ToolContext::from_config(cfg),
            store,
            model,
            system: DEFAULT_SYSTEM.to_string(),
            max_tokens: cfg.max_tokens,
            auto_approve: cfg.tools.auto_approve,
            max_iterations: 12,
        })
    }

    /// Construct an agent from an explicit provider. Useful for tests and for
    /// embedders that build their own provider. Auto-approve defaults to true
    /// here; callers that want gating should set it via the config constructor.
    pub fn with_provider(
        provider: Box<dyn Provider>,
        store: Store,
        registry: ToolRegistry,
        ctx: ToolContext,
        model: impl Into<String>,
    ) -> Agent {
        Agent {
            provider,
            registry,
            ctx,
            store,
            model: model.into(),
            system: DEFAULT_SYSTEM.to_string(),
            max_tokens: 1024,
            auto_approve: true,
            max_iterations: 12,
        }
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// Run one user turn to completion, streaming events to `sink`.
    pub async fn run_turn(
        &self,
        conversation_id: &str,
        user_input: &str,
        approver: Approver,
        sink: UnboundedSender<AgentEvent>,
    ) -> Result<()> {
        let approver: Approver = if self.auto_approve {
            auto_approver()
        } else {
            approver
        };

        self.store
            .append_message(conversation_id, &Message::user(user_input))?;

        for _ in 0..self.max_iterations {
            let messages = self.store.load_messages(conversation_id)?;
            let req = ChatRequest {
                model: self.model.clone(),
                system: Some(self.system.clone()),
                messages,
                tools: self.registry.specs(),
                max_tokens: self.max_tokens,
                temperature: None,
            };

            let mut stream = match self.provider.chat_stream(req).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = sink.send(AgentEvent::Error(e.to_string()));
                    return Err(e);
                }
            };

            let mut assistant_text = String::new();
            let mut tool_calls = Vec::new();

            use futures::StreamExt;
            while let Some(event) = stream.next().await {
                match event {
                    Ok(ChatEvent::TextDelta(t)) => {
                        assistant_text.push_str(&t);
                        let _ = sink.send(AgentEvent::TextDelta(t));
                    }
                    Ok(ChatEvent::ToolCall(call)) => tool_calls.push(call),
                    Ok(ChatEvent::Usage(u)) => {
                        let _ = sink.send(AgentEvent::Usage(u));
                    }
                    Err(e) => {
                        let _ = sink.send(AgentEvent::Error(e.to_string()));
                        return Err(e);
                    }
                }
            }

            // Persist the assistant message (text + any tool-use blocks).
            let mut assistant_blocks = Vec::new();
            if !assistant_text.is_empty() {
                assistant_blocks.push(ContentBlock::text(assistant_text));
            }
            for call in &tool_calls {
                assistant_blocks.push(ContentBlock::ToolUse {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    input: call.input.clone(),
                });
            }
            if !assistant_blocks.is_empty() {
                self.store
                    .append_message(conversation_id, &Message::assistant(assistant_blocks))?;
            }

            if tool_calls.is_empty() {
                let _ = sink.send(AgentEvent::Done);
                return Ok(());
            }

            // Execute each tool call, gathering tool_result blocks.
            let mut result_blocks = Vec::new();
            for call in tool_calls {
                let (output, is_error) =
                    self.execute_tool(conversation_id, &call.id, &call.name, &call.input, &approver, &sink)
                        .await;
                result_blocks.push(ContentBlock::ToolResult {
                    tool_use_id: call.id,
                    content: output,
                    is_error,
                });
            }

            self.store.append_message(
                conversation_id,
                &Message {
                    role: Role::Tool,
                    content: result_blocks,
                },
            )?;
        }

        let _ = sink.send(AgentEvent::Done);
        Ok(())
    }

    async fn execute_tool(
        &self,
        conversation_id: &str,
        id: &str,
        name: &str,
        input: &serde_json::Value,
        approver: &Approver,
        sink: &UnboundedSender<AgentEvent>,
    ) -> (String, bool) {
        let Some(tool) = self.registry.get(name) else {
            let msg = format!("tool '{name}' not found");
            let _ = sink.send(AgentEvent::ToolCompleted {
                id: id.to_string(),
                name: name.to_string(),
                output: msg.clone(),
                is_error: true,
            });
            return (msg, true);
        };

        let summary = tool.summarize(input);
        let _ = sink.send(AgentEvent::ToolStarted {
            id: id.to_string(),
            name: name.to_string(),
            summary: summary.clone(),
        });

        // Approval gate for mutating tools.
        if tool.risk() == Risk::Mutating {
            let decision = approver(ApprovalRequest {
                tool: name.to_string(),
                summary: summary.clone(),
                input: input.clone(),
                risk: Risk::Mutating,
            })
            .await;
            if decision == ApprovalDecision::Deny {
                let msg = "Denied by user.".to_string();
                let _ = self
                    .store
                    .record_tool_call(conversation_id, name, input, &msg, true);
                let _ = sink.send(AgentEvent::ToolCompleted {
                    id: id.to_string(),
                    name: name.to_string(),
                    output: msg.clone(),
                    is_error: true,
                });
                return (msg, true);
            }
        }

        match tool.run(input.clone(), &self.ctx).await {
            Ok(output) => {
                let _ = self
                    .store
                    .record_tool_call(conversation_id, name, input, &output, false);
                let _ = sink.send(AgentEvent::ToolCompleted {
                    id: id.to_string(),
                    name: name.to_string(),
                    output: output.clone(),
                    is_error: false,
                });
                (output, false)
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = self
                    .store
                    .record_tool_call(conversation_id, name, input, &msg, true);
                let _ = sink.send(AgentEvent::ToolCompleted {
                    id: id.to_string(),
                    name: name.to_string(),
                    output: msg.clone(),
                    is_error: true,
                });
                (msg, true)
            }
        }
    }
}
