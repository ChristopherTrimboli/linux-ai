# Security model

- Mutating tools are gated behind explicit approval by default.
- Filesystem tools are confined to configured roots (your home directory by
  default).
- Shell commands are screened against a configurable deny-list (`rm -rf /`,
  `mkfs`, fork bombs, raw disk writes, …).
- Everything runs locally; the only network calls are to the LLM (and
  transcription) provider you configure.

## Approval flow

When the model requests a mutating tool, you decide:

- **Desktop:** an inline approval card with **Approve** / **Deny**.
- **CLI:** a `[y/N]` prompt.

Set `tools.auto_approve = true` (or pass `lai -y`) to skip prompts for a trusted
session. Read-only tools (`system_info`, `read_file`, `list_dir`, `search_files`)
always run without prompting.

## Data & keys

- Conversation history lives in a local SQLite database under
  `~/.local/share/linux-ai/`.
- API keys live in your OS keyring when available, never synced anywhere.
