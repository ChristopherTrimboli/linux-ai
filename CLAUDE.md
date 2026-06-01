# CLAUDE.md

This project's agent guidance lives in [AGENTS.md](./AGENTS.md). Read it first.

@AGENTS.md

## Quick reference

- Build core + CLI: `cargo build -p linux-ai-cli`
- Type-check frontend: `cd desktop && npm run check`
- Run desktop dev: `cd desktop && npm run tauri dev`
- Build docs: `cd docs && npx vitepress build`

Product name is "Linux AI"; the CLI command is `lai`. Shared logic belongs in
`crates/core` (`la-core`), not in the CLI or desktop crates.
