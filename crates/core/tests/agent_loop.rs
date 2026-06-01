//! Integration test for the agent loop using a scripted mock provider:
//! turn 1 emits a tool call, turn 2 (after the tool result is fed back) emits
//! final text. We assert the tool ran and the final text streamed through.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use futures::StreamExt;
use la_core::providers::{ChatEvent, ChatRequest, Provider, ToolCallRequest};
use la_core::tools::{auto_approver, ToolContext, ToolRegistry};
use la_core::{Agent, AgentEvent, Result, Store};
use tokio::sync::mpsc;

struct ScriptedProvider {
    calls: AtomicUsize,
}

#[async_trait]
impl Provider for ScriptedProvider {
    async fn chat_stream(&self, _req: ChatRequest) -> Result<BoxStream<'static, Result<ChatEvent>>> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        let events: Vec<Result<ChatEvent>> = if n == 0 {
            vec![
                Ok(ChatEvent::TextDelta("Let me check.".to_string())),
                Ok(ChatEvent::ToolCall(ToolCallRequest {
                    id: "call_1".to_string(),
                    name: "system_info".to_string(),
                    input: serde_json::json!({}),
                })),
            ]
        } else {
            vec![Ok(ChatEvent::TextDelta("All good.".to_string()))]
        };
        Ok(stream::iter(events).boxed())
    }
}

#[tokio::test]
async fn agent_runs_tool_then_finishes() {
    let store = Store::open_in_memory().unwrap();
    let convo = store.create_conversation("test").unwrap();

    let provider = Box::new(ScriptedProvider {
        calls: AtomicUsize::new(0),
    });
    let ctx = ToolContext {
        file_roots: vec![std::env::temp_dir()],
        shell_deny: vec![],
    };
    let agent = Arc::new(Agent::with_provider(
        provider,
        store.clone(),
        ToolRegistry::builtin(),
        ctx,
        "mock",
    ));

    let (tx, mut rx) = mpsc::unbounded_channel::<AgentEvent>();
    let agent2 = agent.clone();
    let convo_id = convo.id.clone();
    tokio::spawn(async move {
        agent2
            .run_turn(&convo_id, "how much memory do I have?", auto_approver(), tx)
            .await
            .unwrap();
    });

    let mut text = String::new();
    let mut tool_completed = false;
    let mut done = false;
    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::TextDelta(t) => text.push_str(&t),
            AgentEvent::ToolCompleted { name, is_error, .. } => {
                assert_eq!(name, "system_info");
                assert!(!is_error);
                tool_completed = true;
            }
            AgentEvent::Done => {
                done = true;
                break;
            }
            AgentEvent::Error(e) => panic!("agent error: {e}"),
            _ => {}
        }
    }

    assert!(tool_completed, "tool should have run");
    assert!(done, "turn should complete");
    assert!(text.contains("Let me check."));
    assert!(text.contains("All good."));

    // The conversation should have: user, assistant(+tool_use), tool result, assistant.
    let messages = store.load_messages(&convo.id).unwrap();
    assert_eq!(messages.len(), 4);
}
