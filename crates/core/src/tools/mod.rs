//! Tool system: a registry of built-in tools the model can call, plus a
//! risk-based approval mechanism. Read-only tools run freely; mutating tools
//! (shell, write, open) require approval unless the policy auto-approves.

mod builtins;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::BoxFuture;
use serde_json::Value;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::providers::ToolSpec;

pub use builtins::*;

/// Whether a tool can change system state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    ReadOnly,
    Mutating,
}

/// Runtime context handed to every tool invocation.
#[derive(Clone)]
pub struct ToolContext {
    pub file_roots: Vec<PathBuf>,
    pub shell_deny: Vec<String>,
    pub shell_timeout_secs: u64,
}

impl ToolContext {
    pub fn from_config(cfg: &Config) -> Self {
        ToolContext {
            file_roots: cfg
                .file_roots()
                .iter()
                .map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()))
                .collect(),
            shell_deny: cfg.tools.shell_deny.clone(),
            shell_timeout_secs: cfg.tools.shell_timeout_secs,
        }
    }

    /// Resolve a user-supplied path and ensure it falls within an allowed root.
    pub fn resolve_in_roots(&self, path: &str, must_exist: bool) -> Result<PathBuf> {
        let raw = PathBuf::from(shellexpand_tilde(path));
        let candidate = if must_exist {
            raw.canonicalize()
                .map_err(|e| Error::AccessDenied(format!("{path}: {e}")))?
        } else {
            // Canonicalize the parent, then re-attach the filename.
            let parent = raw.parent().unwrap_or_else(|| Path::new("."));
            let file = raw
                .file_name()
                .ok_or_else(|| Error::AccessDenied(format!("invalid path: {path}")))?;
            let parent = parent
                .canonicalize()
                .map_err(|e| Error::AccessDenied(format!("{path}: {e}")))?;
            parent.join(file)
        };

        let allowed = self
            .file_roots
            .iter()
            .any(|root| candidate.starts_with(root));
        if !allowed {
            return Err(Error::AccessDenied(format!(
                "{} is outside the allowed roots",
                candidate.display()
            )));
        }
        Ok(candidate)
    }
}

fn shellexpand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(dirs) = directories::UserDirs::new() {
            return dirs.home_dir().join(rest).to_string_lossy().into_owned();
        }
    }
    path.to_string()
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    fn risk(&self) -> Risk;
    /// A short human-readable summary of what this call will do (for approval UI).
    fn summarize(&self, input: &Value) -> String {
        format!("{}({})", self.name(), input)
    }
    async fn run(&self, input: Value, ctx: &ToolContext) -> Result<String>;
}

#[derive(Clone)]
pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn builtin() -> Self {
        ToolRegistry {
            tools: vec![
                Arc::new(SystemInfo),
                Arc::new(ReadFile),
                Arc::new(WriteFile),
                Arc::new(ListDir),
                Arc::new(SearchFiles),
                Arc::new(RunShell),
                Arc::new(Open),
            ],
        }
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }

    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools
            .iter()
            .map(|t| ToolSpec {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.input_schema(),
            })
            .collect()
    }
}

/// A request for the user to approve a mutating tool call.
#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub tool: String,
    pub summary: String,
    pub input: Value,
    pub risk: Risk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approve,
    Deny,
}

/// Async callback invoked before running a mutating tool. Clients (CLI/desktop)
/// supply this to surface an approval prompt or card.
pub type Approver =
    Arc<dyn Fn(ApprovalRequest) -> BoxFuture<'static, ApprovalDecision> + Send + Sync>;

/// An approver that approves everything (used when auto-approve is enabled).
pub fn auto_approver() -> Approver {
    Arc::new(|_req| Box::pin(async { ApprovalDecision::Approve }))
}
