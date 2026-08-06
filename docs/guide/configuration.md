# Configuration & secrets

- **Config:** `~/.config/linux-ai/config.toml` (created on first run).
- **History:** `~/.local/share/linux-ai/linux-ai.db` (SQLite).

## API keys

Keys are resolved in this order:

1. **OS keyring** (service `linux-ai`, account = provider name)
2. **Environment variable** (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`,
   `OPENROUTER_API_KEY`, `XAI_API_KEY`, …)
3. **Plaintext `api_key`** in the config file (lowest precedence)

Store keys with `lai auth <provider>` or via the desktop **Settings** panel. On
headless systems without a secret service, the keyring step is skipped and the
env/config fallback is used.

## Providers

Out of the box there are providers for **Anthropic**, **OpenAI**, **xAI** (Grok),
**OpenRouter** (access 400+ models behind one key, e.g. `openrouter/auto`), and a
**local** OpenAI-compatible endpoint (Ollama). Add any other OpenAI-compatible
endpoint by editing the `[providers.*]` tables in the config and pointing
`base_url` at it.

```toml
default_provider = "xai"
default_model = "grok-4.5"
max_tokens = 4096

[providers.xai]
kind = "openai"
base_url = "https://api.x.ai/v1"
api_key_env = "XAI_API_KEY"
models = [
  "grok-4.5",
  "grok-4.5-latest",
  "grok-4.3",
  "grok-4.20-0309-reasoning",
  "grok-4.20-0309-non-reasoning",
  "grok-4.20-multi-agent-0309",
  "grok-build-0.1",
]

[providers.openrouter]
kind = "openai"
base_url = "https://openrouter.ai/api/v1"
api_key_env = "OPENROUTER_API_KEY"
models = ["openrouter/auto", "openai/gpt-5.5"]

[stt]
provider = "openai"
model = "gpt-4o-transcribe"

[tools]
auto_approve = false
file_roots = []          # empty = home directory only
shell_timeout_secs = 120
```

### xAI / Grok

Uses the OpenAI-compatible Chat Completions API at `https://api.x.ai/v1` with an
`XAI_API_KEY` from the [xAI console](https://console.x.ai/). `grok-4.5` is the
flagship (500k context, reasoning, function calling, image input); `grok-4.5-latest`
always tracks the newest pin. See [xAI models](https://docs.x.ai/docs/models) for
pricing and the full catalog.

## Tool policy

| Key | Description |
| --- | --- |
| `auto_approve` | Run mutating tools without prompting |
| `file_roots` | Filesystem roots the agent may read/write (empty = home only) |
| `shell_deny` | Substrings that cause an automatic shell-command denial |
| `shell_timeout_secs` | Kill a `run_shell` command after this many seconds (default 120) |

### Shell commands are non-interactive

`run_shell` runs with no terminal and stdin closed, so commands can't prompt for
input. `sudo` is automatically run as `sudo -n` (non-interactive); if it needs a
password it fails fast instead of hanging, and the agent is told to ask you to
run the command yourself in a terminal. Any command that still blocks is killed
after `shell_timeout_secs`. To allow privileged commands without a prompt, give
your user passwordless sudo for them (via `sudoers`) at your own discretion —
the app will never modify your sudo configuration.
