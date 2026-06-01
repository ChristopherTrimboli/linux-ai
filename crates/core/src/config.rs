use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Which wire protocol a provider speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    /// OpenAI Chat Completions compatible (covers OpenAI, Ollama, LM Studio, etc.).
    Openai,
    /// Anthropic Messages API.
    Anthropic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    /// Base URL without a trailing slash. Defaults are filled in if omitted.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Environment variable to read the API key from, e.g. `OPENAI_API_KEY`.
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Plaintext API key (lowest precedence fallback; prefer keyring or env).
    #[serde(default)]
    pub api_key: Option<String>,
    /// Known model identifiers for this provider (for listing/selection).
    #[serde(default)]
    pub models: Vec<String>,
}

impl ProviderConfig {
    pub fn effective_base_url(&self) -> String {
        let url = self.base_url.clone().unwrap_or_else(|| match self.kind {
            ProviderKind::Openai => "https://api.openai.com/v1".to_string(),
            ProviderKind::Anthropic => "https://api.anthropic.com".to_string(),
        });
        url.trim_end_matches('/').to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPolicy {
    /// When true, mutating tools run without prompting for approval.
    #[serde(default)]
    pub auto_approve: bool,
    /// Filesystem roots the agent may read/write within. Empty means "home only".
    #[serde(default)]
    pub file_roots: Vec<PathBuf>,
    /// Substrings that, if present in a shell command, cause an automatic deny.
    #[serde(default = "default_shell_deny")]
    pub shell_deny: Vec<String>,
}

fn default_shell_deny() -> Vec<String> {
    vec![
        "rm -rf /".to_string(),
        "mkfs".to_string(),
        ":(){".to_string(),
        "dd if=".to_string(),
        "> /dev/sd".to_string(),
    ]
}

impl Default for ToolPolicy {
    fn default() -> Self {
        ToolPolicy {
            auto_approve: false,
            file_roots: Vec::new(),
            shell_deny: default_shell_deny(),
        }
    }
}

/// Speech-to-text settings. Transcription goes through an OpenAI-compatible
/// `/audio/transcriptions` endpoint, reusing one of the configured providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// Provider key (into `providers`) used for transcription.
    pub provider: String,
    /// Transcription model, e.g. `whisper-1` or `gpt-4o-mini-transcribe`.
    pub model: String,
}

impl Default for SttConfig {
    fn default() -> Self {
        SttConfig {
            provider: "openai".to_string(),
            // gpt-4o-transcribe is OpenAI's current best transcription model
            // (lower WER than whisper-1, same price). Override for local servers.
            model: "gpt-4o-transcribe".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub default_provider: String,
    pub default_model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    pub providers: BTreeMap<String, ProviderConfig>,
    #[serde(default)]
    pub tools: ToolPolicy,
    #[serde(default)]
    pub stt: SttConfig,
}

fn default_max_tokens() -> u32 {
    4096
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = BTreeMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                kind: ProviderKind::Anthropic,
                base_url: None,
                api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
                api_key: None,
                models: vec![
                    "claude-opus-4-8".to_string(),
                    "claude-sonnet-4-6".to_string(),
                    "claude-haiku-4-5".to_string(),
                ],
            },
        );
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                kind: ProviderKind::Openai,
                base_url: None,
                api_key_env: Some("OPENAI_API_KEY".to_string()),
                api_key: None,
                models: vec![
                    "gpt-5.5".to_string(),
                    "gpt-5.5-pro".to_string(),
                    "gpt-5.4".to_string(),
                    "gpt-5.4-mini".to_string(),
                    "gpt-5.4-nano".to_string(),
                ],
            },
        );
        providers.insert(
            "openrouter".to_string(),
            ProviderConfig {
                kind: ProviderKind::Openai,
                base_url: Some("https://openrouter.ai/api/v1".to_string()),
                api_key_env: Some("OPENROUTER_API_KEY".to_string()),
                api_key: None,
                models: vec![
                    "openrouter/auto".to_string(),
                    "anthropic/claude-opus-4.8".to_string(),
                    "anthropic/claude-sonnet-4.6".to_string(),
                    "openai/gpt-5.5".to_string(),
                    "google/gemini-3.5-flash".to_string(),
                    "meta-llama/llama-3.3-70b-instruct".to_string(),
                ],
            },
        );
        providers.insert(
            "local".to_string(),
            ProviderConfig {
                kind: ProviderKind::Openai,
                base_url: Some("http://localhost:11434/v1".to_string()),
                api_key_env: None,
                api_key: Some("ollama".to_string()),
                models: vec![
                    "llama3.3".to_string(),
                    "qwen3".to_string(),
                    "gpt-oss:20b".to_string(),
                ],
            },
        );

        Config {
            default_provider: "anthropic".to_string(),
            default_model: "claude-sonnet-4-6".to_string(),
            max_tokens: default_max_tokens(),
            providers,
            tools: ToolPolicy::default(),
            stt: SttConfig::default(),
        }
    }
}

impl Config {
    /// Path to the config file: `~/.config/linux-ai/config.toml`.
    pub fn config_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("dev", "linux-ai", "linux-ai")
            .ok_or_else(|| Error::Config("could not determine config directory".into()))?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    /// Load config from disk, creating a default file if none exists.
    pub fn load() -> Result<Config> {
        let path = Self::config_path()?;
        if !path.exists() {
            let cfg = Config::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(&path)?;
        let cfg: Config =
            toml::from_str(&text).map_err(|e| Error::Config(format!("parse {path:?}: {e}")))?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text =
            toml::to_string_pretty(self).map_err(|e| Error::Config(format!("serialize: {e}")))?;
        std::fs::write(&path, text)?;
        Ok(())
    }

    pub fn provider(&self, name: &str) -> Result<&ProviderConfig> {
        self.providers
            .get(name)
            .ok_or_else(|| Error::UnknownProvider(name.to_string()))
    }

    /// Effective filesystem roots: configured roots, or the home directory.
    pub fn file_roots(&self) -> Vec<PathBuf> {
        if self.tools.file_roots.is_empty() {
            directories::UserDirs::new()
                .map(|d| vec![d.home_dir().to_path_buf()])
                .unwrap_or_default()
        } else {
            self.tools.file_roots.clone()
        }
    }
}
