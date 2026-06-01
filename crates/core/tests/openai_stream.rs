//! Integration test: a minimal mock server emits an OpenAI-style SSE stream and
//! we assert the provider normalizes it into the expected `ChatEvent`s,
//! including a tool call assembled across multiple deltas.

use futures::StreamExt;
use la_core::providers::{ChatEvent, ChatRequest, OpenAiProvider, Provider};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const SSE_BODY: &str = "\
data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n\
data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\n\
data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"function\":{\"name\":\"system_info\",\"arguments\":\"\"}}]}}]}\n\n\
data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{}\"}}]}}]}\n\n\
data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n\
data: {\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5},\"choices\":[]}\n\n\
data: [DONE]\n\n";

async fn spawn_mock_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            // Drain the request headers.
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{SSE_BODY}"
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.flush().await;
        }
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn openai_stream_parses_text_and_tool_call() {
    let base_url = spawn_mock_server().await;
    let provider = OpenAiProvider::new(base_url, Some("test-key".to_string()));

    let req = ChatRequest {
        model: "test-model".to_string(),
        system: Some("system".to_string()),
        messages: vec![la_core::Message::user("hi")],
        tools: vec![],
        max_tokens: 256,
        temperature: None,
    };

    let mut stream = provider.chat_stream(req).await.expect("stream");
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    let mut usage = None;
    while let Some(event) = stream.next().await {
        match event.expect("event") {
            ChatEvent::TextDelta(t) => text.push_str(&t),
            ChatEvent::ToolCall(c) => tool_calls.push(c),
            ChatEvent::Usage(u) => usage = Some(u),
        }
    }

    assert_eq!(text, "Hello world");
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].name, "system_info");
    assert_eq!(tool_calls[0].id, "call_1");
    assert_eq!(tool_calls[0].input, serde_json::json!({}));
    let usage = usage.expect("usage");
    assert_eq!(usage.input_tokens, 10);
    assert_eq!(usage.output_tokens, 5);
}
