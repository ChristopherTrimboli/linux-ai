# Configuration & secrets

- **Config:** `~/.config/linux-ai/config.toml` (created on first run).
- **History:** `~/.local/share/linux-ai/linux-ai.db` (SQLite).

## API keys

Keys are resolved in this order:

1. **OS keyring** (service `linux-ai`, account = provider name)
2. **Environment variable** (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`,
   `OPENROUTER_API_KEY`, …)
3. **Plaintext `api_key`** in the config file (lowest precedence)

Store keys with `ai auth <provider>` or via the desktop **Settings** panel. On
headless systems without a secret service, the keyring step is skipped and the
env/config fallback is used.

## Providers

Out of the box there are providers for **Anthropic**, **OpenAI**, **OpenRouter**
(access 400+ models behind one key, e.g. `openrouter/auto`), and a **local**
OpenAI-compatible endpoint (Ollama). Add any other OpenAI-compatible endpoint by
editing the `[providers.*]` tables in the config and pointing `base_url` at it.

```toml
default_provider = "openrouter"
default_model = "anthropic/claude-sonnet-4.6"
max_tokens = 4096

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
```

## Tool policy

| Key | Description |
| --- | --- |
| `auto_approve` | Run mutating tools without prompting |
| `file_roots` | Filesystem roots the agent may read/write (empty = home only) |
| `shell_deny` | Substrings that cause an automatic shell-command denial |
