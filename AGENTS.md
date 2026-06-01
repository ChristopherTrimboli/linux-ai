# Linux AI — agent guide

Linux-first AI chat app with tool-based computer access, shipped as a Tauri
desktop app **and** a `lai` CLI that share one Rust core. Free and open source.

## Layout

```
crates/core/        la-core: providers, tools, agent loop, SQLite store, config, transcribe
crates/cli/         the `lai` binary (clap + rustyline REPL)
desktop/src/        Svelte 5 + Vite frontend
desktop/src-tauri/  Tauri 2 Rust app (depends on la-core)
docs/               VitePress site (auto-deploys to GitHub Pages)
snap/ packaging/    Snap, Flatpak, and AUR packaging
```

Both front ends call into `la-core`. Put shared logic (providers, tools, agent
loop, persistence) in the core crate, never in the CLI or desktop layer.

## Build & verify

- Core + CLI: `cargo build -p linux-ai-cli` (no system libs needed).
- Desktop: `cd desktop && npm install && npm run tauri dev`. Needs the Tauri
  Linux libs plus `libasound2-dev` (mic capture for voice input).
- Frontend type-check: `cd desktop && npm run check` (must be clean).
- Docs: `cd docs && npx vitepress build`.

Always run `cargo build -p linux-ai-cli` and `npm run check` after changes in
their respective areas.

## Conventions

- **Bring-your-own-key**: never hardcode API keys; resolve via keyring → env →
  config (`crates/core/src/secrets.rs`).
- **Security model is load-bearing**: mutating tools (`run_shell`, `write_file`,
  `open`) require approval; filesystem tools are confined to configured roots;
  shell is deny-listed. Don't weaken these defaults.
- **Naming**: the product is "Linux AI", the CLI command is `lai`. Never
  reintroduce "companion" anywhere.
- **Versioning**: the git tag (`vX.Y.Z`) is the single source of truth. Don't
  hand-edit versions; the release workflow stamps `Cargo.toml`,
  `desktop/package.json`, and the `la-core` dep version.

## Don't

- Don't add a local-LLM monolith or agentic-coding features — out of scope.
- Don't put provider/tool logic in the CLI or desktop crates.
- Don't commit `target/`, `node_modules/`, or `docs/.vitepress/dist/`.
