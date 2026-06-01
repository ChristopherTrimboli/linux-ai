//! Speech-to-text via an OpenAI-compatible `/audio/transcriptions` endpoint.
//!
//! Two request shapes are supported, selected automatically from the provider's
//! base URL:
//! - **OpenAI-style** (OpenAI, whisper.cpp, faster-whisper, Groq, …): a
//!   `multipart/form-data` upload with a `file` field.
//! - **OpenRouter-style**: a JSON body with base64-encoded `input_audio.data`.
//!
//! Both return `{ "text": ... }`. The provider and model come from `[stt]`.

use base64::Engine;
use reqwest::multipart::{Form, Part};
use serde_json::json;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::secrets;

/// Transcribe encoded audio bytes (e.g. a WAV file) to text.
///
/// `filename` is the multipart file name; its extension hints the server at the
/// container format (use `audio.wav` for 16-bit PCM WAV).
pub async fn transcribe(cfg: &Config, audio: Vec<u8>, filename: &str) -> Result<String> {
    if audio.is_empty() {
        return Err(Error::other("no audio captured"));
    }

    let provider_name = cfg.stt.provider.clone();
    let pc = cfg.provider(&provider_name)?;
    let base = pc.effective_base_url();
    let url = format!("{base}/audio/transcriptions");
    let api_key = secrets::resolve_api_key(&provider_name, pc);

    let client = reqwest::Client::new();
    let builder = if base.contains("openrouter.ai") {
        // OpenRouter wants a JSON body with base64-encoded audio (raw bytes).
        let data = base64::engine::general_purpose::STANDARD.encode(&audio);
        let body = json!({
            "model": cfg.stt.model,
            "input_audio": { "data": data, "format": "wav" },
        });
        client.post(&url).json(&body)
    } else {
        // OpenAI-compatible multipart upload.
        let part = Part::bytes(audio)
            .file_name(filename.to_string())
            .mime_str("audio/wav")?;
        let form = Form::new()
            .text("model", cfg.stt.model.clone())
            .text("response_format", "json")
            .part("file", part);
        client.post(&url).multipart(form)
    };

    let builder = match api_key {
        Some(key) => builder.bearer_auth(key),
        None => builder,
    };

    let resp = builder.send().await?;
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        return Err(Error::Provider {
            status: status.as_u16(),
            body,
        });
    }

    let value: serde_json::Value = serde_json::from_str(&body)?;
    let text = value
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    Ok(text)
}
