# Using the CLI

The CLI installs as `ai`.

```bash
ai "what is using port 8080?"        # one-shot prompt
cat error.log | ai "explain this"    # pipe stdin into the prompt
ai chat                              # interactive REPL (Ctrl-D / 'exit' to quit)

ai providers                         # list providers and whether a key is set
ai models openai                     # list known models for a provider
ai auth anthropic                    # store an API key in the OS keyring
ai config                            # show config + data file locations

ai -p openai -m gpt-4o "hello"       # override provider/model for one run
ai -y "tidy up ~/Downloads"          # auto-approve tool actions for this run
```

## Flags

| Flag | Description |
| --- | --- |
| `-p, --provider <name>` | Provider to use (overrides config default) |
| `-m, --model <id>` | Model to use (overrides config default) |
| `-y, --yes` | Auto-approve mutating tool actions for this run |

## Subcommands

| Command | Description |
| --- | --- |
| `ai chat` | Interactive REPL with streaming responses |
| `ai providers` | List configured providers and key status |
| `ai models <provider>` | List known models for a provider |
| `ai auth <provider>` | Store an API key in the OS keyring |
| `ai config` | Print config and data file locations |

Anything that isn't a subcommand is treated as a one-shot prompt, so
`ai "summarize this repo"` just works. Piped stdin is appended to the prompt,
which makes `ai` composable with the rest of your shell.
