# Contributing

Contributions are welcome. The project is a Cargo workspace plus a Svelte
frontend.

## Local development

```bash
# core + CLI (no system libs required)
cargo build
cargo test

# desktop app (requires the Tauri system libraries — see Installation)
cd desktop && npm install && npm run tauri dev
```

## Layout

- `crates/core` — the shared engine (`la-core`): providers, tools, agent loop,
  SQLite store, config.
- `crates/cli` — the `ai` command line.
- `desktop` — the Tauri 2 + Svelte 5 desktop app.
- `docs` — this VitePress site.

## Guidelines

- Keep the core provider-agnostic; new providers should slot into the existing
  `Provider` trait.
- New tools should declare an appropriate `Risk` and respect the approval flow.
- Run `cargo build` and `npm run check` (in `desktop`) before opening a PR.

## License

MIT — see [LICENSE](https://github.com/ChristopherTrimboli/linux-ai/blob/main/LICENSE).
