# Using the CLI

The CLI installs as `lai`.

```bash
lai "what is using port 8080?"       # one-shot prompt
cat error.log | lai "explain this"    # pipe stdin into the prompt
lai chat                              # interactive REPL (Ctrl-D / 'exit' to quit)

lai providers                         # list providers and whether a key is set
lai models openai                     # list known models for a provider
lai auth anthropic                    # store an API key in the OS keyring
lai config                            # show config + data file locations

lai -p openai -m gpt-4o "hello"       # override provider/model for one run
lai -y "tidy up ~/Downloads"          # auto-approve tool actions for this run
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
| `lai chat` | Interactive REPL with streaming responses |
| `lai providers` | List configured providers and key status |
| `lai models <provider>` | List known models for a provider |
| `lai auth <provider>` | Store an API key in the OS keyring |
| `lai config` | Print config and data file locations |

Anything that isn't a subcommand is treated as a one-shot prompt, so
`lai "summarize this repo"` just works. Piped stdin is appended to the prompt,
which makes `lai` composable with the rest of your shell.
