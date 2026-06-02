//! Built-in v1 tools. Inputs are validated against `ToolContext` (filesystem
//! roots, shell deny-list) before any side effects occur.

use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::error::{Error, Result};

use super::{Risk, Tool, ToolContext};

const MAX_READ_BYTES: usize = 256 * 1024;
const MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// Rewrite standalone `sudo` tokens to `sudo -n` (non-interactive). The tool has
/// no terminal to type a password into, so an interactive `sudo` would block
/// forever; `-n` makes it fail fast with "a password is required" instead.
fn force_noninteractive_sudo(command: &str) -> String {
    let mut result = String::with_capacity(command.len() + 4);
    let mut last = 0;
    for (idx, _) in command.match_indices("sudo") {
        let before_ok = idx == 0
            || command[..idx]
                .chars()
                .next_back()
                .map(|c| c.is_whitespace() || matches!(c, '|' | '&' | ';' | '('))
                .unwrap_or(false);
        let after = &command[idx + 4..];
        let after_ok = after
            .chars()
            .next()
            .map(|c| c.is_whitespace())
            .unwrap_or(false);
        if before_ok && after_ok {
            let rest = after.trim_start();
            result.push_str(&command[last..idx + 4]);
            if !(rest == "-n" || rest.starts_with("-n ") || rest.starts_with("--non-interactive"))
            {
                result.push_str(" -n");
            }
            last = idx + 4;
        }
    }
    result.push_str(&command[last..]);
    result
}

fn str_arg(input: &Value, key: &str) -> Result<String> {
    input
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::ToolInput(format!("missing string field '{key}'")))
}

fn truncate(mut s: String, max: usize) -> String {
    if s.len() > max {
        s.truncate(max);
        s.push_str("\n... [truncated]");
    }
    s
}

// ---------------------------------------------------------------------------
// system_info
// ---------------------------------------------------------------------------

pub struct SystemInfo;

#[async_trait]
impl Tool for SystemInfo {
    fn name(&self) -> &str {
        "system_info"
    }
    fn description(&self) -> &str {
        "Get information about this Linux machine: OS, kernel, hostname, CPU count, memory and disk usage."
    }
    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "additionalProperties": false })
    }
    fn risk(&self) -> Risk {
        Risk::ReadOnly
    }
    async fn run(&self, _input: Value, _ctx: &ToolContext) -> Result<String> {
        use sysinfo::{Disks, System};
        let mut sys = System::new_all();
        sys.refresh_memory();
        sys.refresh_cpu_usage();

        let total_mem = sys.total_memory() / 1_048_576;
        let used_mem = sys.used_memory() / 1_048_576;
        let disks = Disks::new_with_refreshed_list();
        let mut disk_lines = Vec::new();
        for disk in &disks {
            disk_lines.push(format!(
                "  {}: {} MB free of {} MB",
                disk.mount_point().display(),
                disk.available_space() / 1_048_576,
                disk.total_space() / 1_048_576,
            ));
        }

        let info = json!({
            "os": System::long_os_version(),
            "kernel": System::kernel_version(),
            "hostname": System::host_name(),
            "cpu_count": sys.cpus().len(),
            "memory_mb": { "used": used_mem, "total": total_mem },
            "disks": disk_lines,
        });
        Ok(serde_json::to_string_pretty(&info)?)
    }
}

// ---------------------------------------------------------------------------
// read_file
// ---------------------------------------------------------------------------

pub struct ReadFile;

#[async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Read the contents of a UTF-8 text file at the given absolute or ~-relative path."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "File path to read" } },
            "required": ["path"],
            "additionalProperties": false
        })
    }
    fn risk(&self) -> Risk {
        Risk::ReadOnly
    }
    fn summarize(&self, input: &Value) -> String {
        format!("read file {}", input.get("path").unwrap_or(&Value::Null))
    }
    async fn run(&self, input: Value, ctx: &ToolContext) -> Result<String> {
        let path = str_arg(&input, "path")?;
        let resolved = ctx.resolve_in_roots(&path, true)?;
        let bytes = tokio::fs::read(&resolved).await?;
        let slice = &bytes[..bytes.len().min(MAX_READ_BYTES)];
        let text = String::from_utf8_lossy(slice).into_owned();
        if bytes.len() > MAX_READ_BYTES {
            Ok(format!("{text}\n... [truncated, file is {} bytes]", bytes.len()))
        } else {
            Ok(text)
        }
    }
}

// ---------------------------------------------------------------------------
// write_file
// ---------------------------------------------------------------------------

pub struct WriteFile;

#[async_trait]
impl Tool for WriteFile {
    fn name(&self) -> &str {
        "write_file"
    }
    fn description(&self) -> &str {
        "Write (creating or overwriting) a UTF-8 text file at the given path."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to write" },
                "content": { "type": "string", "description": "Full file contents" }
            },
            "required": ["path", "content"],
            "additionalProperties": false
        })
    }
    fn risk(&self) -> Risk {
        Risk::Mutating
    }
    fn summarize(&self, input: &Value) -> String {
        let len = input
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.len())
            .unwrap_or(0);
        format!(
            "write {} bytes to {}",
            len,
            input.get("path").unwrap_or(&Value::Null)
        )
    }
    async fn run(&self, input: Value, ctx: &ToolContext) -> Result<String> {
        let path = str_arg(&input, "path")?;
        let content = str_arg(&input, "content")?;
        let resolved = ctx.resolve_in_roots(&path, false)?;
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&resolved, content.as_bytes()).await?;
        Ok(format!(
            "wrote {} bytes to {}",
            content.len(),
            resolved.display()
        ))
    }
}

// ---------------------------------------------------------------------------
// list_dir
// ---------------------------------------------------------------------------

pub struct ListDir;

#[async_trait]
impl Tool for ListDir {
    fn name(&self) -> &str {
        "list_dir"
    }
    fn description(&self) -> &str {
        "List the entries (files and directories) in a directory."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "Directory path" } },
            "required": ["path"],
            "additionalProperties": false
        })
    }
    fn risk(&self) -> Risk {
        Risk::ReadOnly
    }
    fn summarize(&self, input: &Value) -> String {
        format!("list directory {}", input.get("path").unwrap_or(&Value::Null))
    }
    async fn run(&self, input: Value, ctx: &ToolContext) -> Result<String> {
        let path = str_arg(&input, "path")?;
        let resolved = ctx.resolve_in_roots(&path, true)?;
        let mut entries = tokio::fs::read_dir(&resolved).await?;
        let mut lines = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            let marker = if file_type.is_dir() { "/" } else { "" };
            lines.push(format!("{}{}", entry.file_name().to_string_lossy(), marker));
        }
        lines.sort();
        Ok(if lines.is_empty() {
            "(empty)".to_string()
        } else {
            lines.join("\n")
        })
    }
}

// ---------------------------------------------------------------------------
// search_files
// ---------------------------------------------------------------------------

pub struct SearchFiles;

#[async_trait]
impl Tool for SearchFiles {
    fn name(&self) -> &str {
        "search_files"
    }
    fn description(&self) -> &str {
        "Recursively search a directory for a substring in file names and text-file contents. Returns matching paths (and matching lines)."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Substring to search for" },
                "path": { "type": "string", "description": "Root directory to search (defaults to first allowed root)" }
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }
    fn risk(&self) -> Risk {
        Risk::ReadOnly
    }
    fn summarize(&self, input: &Value) -> String {
        format!("search for {}", input.get("query").unwrap_or(&Value::Null))
    }
    async fn run(&self, input: Value, ctx: &ToolContext) -> Result<String> {
        let query = str_arg(&input, "query")?;
        let root = match input.get("path").and_then(|v| v.as_str()) {
            Some(p) => ctx.resolve_in_roots(p, true)?,
            None => ctx
                .file_roots
                .first()
                .cloned()
                .ok_or_else(|| Error::AccessDenied("no allowed roots".into()))?,
        };

        let query_l = query.to_lowercase();
        let mut results = Vec::new();
        let mut count = 0usize;
        for entry in walkdir::WalkDir::new(&root)
            .max_depth(8)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if count >= 200 {
                results.push("... [more results omitted]".to_string());
                break;
            }
            let path = entry.path();
            if entry.file_type().is_file() {
                let name_match = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase().contains(&query_l))
                    .unwrap_or(false);
                if name_match {
                    results.push(format!("{}", path.display()));
                    count += 1;
                    continue;
                }
                if let Ok(meta) = entry.metadata() {
                    if meta.len() > MAX_READ_BYTES as u64 {
                        continue;
                    }
                }
                if let Ok(text) = tokio::fs::read_to_string(path).await {
                    for (i, line) in text.lines().enumerate() {
                        if line.to_lowercase().contains(&query_l) {
                            results.push(format!(
                                "{}:{}: {}",
                                path.display(),
                                i + 1,
                                line.trim()
                            ));
                            count += 1;
                            break;
                        }
                    }
                }
            }
        }
        Ok(if results.is_empty() {
            "no matches".to_string()
        } else {
            results.join("\n")
        })
    }
}

// ---------------------------------------------------------------------------
// run_shell
// ---------------------------------------------------------------------------

pub struct RunShell;

#[async_trait]
impl Tool for RunShell {
    fn name(&self) -> &str {
        "run_shell"
    }
    fn description(&self) -> &str {
        "Run a shell command via `sh -c` and return combined stdout/stderr and the exit code. Use for system tasks; prefer dedicated tools for file reads/writes. Runs non-interactively with no terminal: commands cannot prompt for input. `sudo` is forced to `-n` (non-interactive) and will fail if a password is required — in that case tell the user to run the command themselves in a terminal. Long-running commands are killed after a timeout."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" },
                "cwd": { "type": "string", "description": "Optional working directory" }
            },
            "required": ["command"],
            "additionalProperties": false
        })
    }
    fn risk(&self) -> Risk {
        Risk::Mutating
    }
    fn summarize(&self, input: &Value) -> String {
        format!(
            "run shell: {}",
            input.get("command").and_then(|v| v.as_str()).unwrap_or("")
        )
    }
    async fn run(&self, input: Value, ctx: &ToolContext) -> Result<String> {
        let command = str_arg(&input, "command")?;
        for deny in &ctx.shell_deny {
            if command.contains(deny.as_str()) {
                return Err(Error::AccessDenied(format!(
                    "command blocked by deny-list pattern: {deny}"
                )));
            }
        }

        let to_run = force_noninteractive_sudo(&command);
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c")
            .arg(&to_run)
            // No interactive input: close stdin so anything that reads it gets
            // EOF instead of blocking, and kill the child if we drop on timeout.
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(cwd) = input.get("cwd").and_then(|v| v.as_str()) {
            let dir = ctx.resolve_in_roots(cwd, true)?;
            cmd.current_dir(dir);
        }

        let timeout = Duration::from_secs(ctx.shell_timeout_secs.max(1));
        let output = match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(result) => result?,
            Err(_) => {
                return Ok(format!(
                    "exit code: -1\n[error] command exceeded the {}s timeout and was terminated. \
It may have been waiting for interactive input (this tool has no terminal). If it \
needs a password or a prompt, ask the user to run it themselves in a terminal.",
                    ctx.shell_timeout_secs
                ));
            }
        };

        let mut combined = String::new();
        combined.push_str(&String::from_utf8_lossy(&output.stdout));
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            combined.push_str("\n[stderr]\n");
            combined.push_str(&stderr);
        }

        // Surface a clear hint when sudo refused for lack of a password, so the
        // model stops retrying and tells the user instead.
        let code = output.status.code().unwrap_or(-1);
        if code != 0
            && (stderr.contains("a password is required")
                || stderr.contains("a terminal is required")
                || stderr.contains("no tty present"))
        {
            combined.push_str(
                "\n[hint] This command needs elevated (sudo) privileges and cannot be \
run non-interactively here. Ask the user to run it in a terminal.",
            );
        }

        Ok(truncate(
            format!("exit code: {code}\n{combined}"),
            MAX_OUTPUT_BYTES,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::force_noninteractive_sudo;

    #[test]
    fn rewrites_sudo_to_noninteractive() {
        assert_eq!(force_noninteractive_sudo("sudo apt update"), "sudo -n apt update");
        assert_eq!(
            force_noninteractive_sudo("echo hi | sudo tee /etc/x"),
            "echo hi | sudo -n tee /etc/x"
        );
    }

    #[test]
    fn leaves_other_commands_alone() {
        assert_eq!(force_noninteractive_sudo("ls -la"), "ls -la");
        assert_eq!(force_noninteractive_sudo("pseudo cmd"), "pseudo cmd");
        assert_eq!(force_noninteractive_sudo("sudoers"), "sudoers");
        // Already non-interactive: don't double up.
        assert_eq!(force_noninteractive_sudo("sudo -n apt update"), "sudo -n apt update");
    }
}

// ---------------------------------------------------------------------------
// open
// ---------------------------------------------------------------------------

pub struct Open;

#[async_trait]
impl Tool for Open {
    fn name(&self) -> &str {
        "open"
    }
    fn description(&self) -> &str {
        "Open a file, directory, URL, or application target using the desktop's default handler (xdg-open)."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "target": { "type": "string", "description": "Path or URL to open" } },
            "required": ["target"],
            "additionalProperties": false
        })
    }
    fn risk(&self) -> Risk {
        Risk::Mutating
    }
    fn summarize(&self, input: &Value) -> String {
        format!("open {}", input.get("target").unwrap_or(&Value::Null))
    }
    async fn run(&self, input: Value, _ctx: &ToolContext) -> Result<String> {
        let target = str_arg(&input, "target")?;
        let status = tokio::process::Command::new("xdg-open")
            .arg(&target)
            .status()
            .await?;
        if status.success() {
            Ok(format!("opened {target}"))
        } else {
            Err(Error::other(format!(
                "xdg-open exited with status {}",
                status.code().unwrap_or(-1)
            )))
        }
    }
}
