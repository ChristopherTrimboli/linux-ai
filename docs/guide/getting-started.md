# What it is

A Linux-first AI chat app that can act on your computer through a small set
of built-in tools, available as both a desktop app and a CLI. Bring your own API
key (Anthropic, OpenAI, or any OpenAI-compatible endpoint, including a local one).

- **Desktop app** — Tauri 2 (Rust) + Svelte 5. Tiny, fast, native WebKitGTK.
- **CLI (`lai`)** — one-shot prompts, stdin piping, and an interactive chat REPL.
- **Shared Rust core (`la-core`)** — both front ends use the exact same engine:
  multi-provider streaming, tool execution, approval gating, and local history.

## Core philosophy

1. We are not competing in the agentic coding realm.
2. This will always be free, open software with no paid subscription tiers or
   locked features.
3. This is not a local-AI monolith for running every niche local LLM.

**It's an app for integrating AI intelligence into your Linux computer.**

## Built-in tools

| Tool | Risk | What it does |
| --- | --- | --- |
| `system_info` | read-only | OS, kernel, CPU, memory, disk usage |
| `read_file` | read-only | Read a text file (within allowed roots) |
| `list_dir` | read-only | List a directory |
| `search_files` | read-only | Recursive name/content search |
| `write_file` | mutating | Create/overwrite a file (needs approval) |
| `run_shell` | mutating | Run a shell command (needs approval) |
| `open` | mutating | Open a file/URL with `xdg-open` (needs approval) |

Read-only tools run automatically. Mutating tools require explicit approval (an
inline card in the desktop app, a `[y/N]` prompt in the CLI) unless you enable
auto-approve. File access is restricted to configured roots (your home directory
by default) and shell commands are screened against a deny-list.

## Repository layout

```
linux-ai/
  crates/
    core/        # la-core: providers, tools, agent loop, SQLite store, config
    cli/         # `lai` binary
  desktop/
    src/         # Svelte 5 + Vite frontend
    src-tauri/   # Tauri 2 Rust app (depends on la-core)
  docs/          # this VitePress site
```

Next: head to [Installation](./installation).
