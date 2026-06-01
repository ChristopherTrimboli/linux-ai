# Releases & versioning

Pushing a version tag builds and publishes a GitHub Release with production
artifacts for both **x86_64** and **aarch64** Linux (CLI tarball + `.deb` +
`.rpm` + AppImage per arch):

```bash
git tag v0.1.0
git push origin v0.1.0
```

You can also trigger the workflow manually from the Actions tab (provide the tag
as input).

The same tag triggers two more workflows: the [Snap](./snap) build/publish and a
**crates.io** publish of `la-core` and `linux-ai-cli`.

## Publishing to crates.io

The [`Publish crates` workflow](https://github.com/ChristopherTrimboli/linux-ai/blob/main/.github/workflows/publish-crates.yml)
publishes `la-core` then `linux-ai-cli` (the `lai` binary) on every version tag.
It needs the repo secret `CARGO_REGISTRY_TOKEN` (a crates.io API token), and the
crate names must be available/owned on crates.io. Once published, anyone can
install the CLI with `cargo install linux-ai-cli`.

## Versioning

The **git tag is the single source of truth** for a release's version. Tags must
be semver (`vMAJOR.MINOR.PATCH`, optionally `-rc.1`); the workflow rejects
anything else. At build time it strips the leading `v` and stamps that version
into the workspace `Cargo.toml` (which every crate and the CLI inherit) and
`desktop/package.json`. The desktop bundle has no version of its own in
`tauri.conf.json` — it inherits the workspace crate version — so the CLI, the
crates, and the `.deb`/AppImage all carry the exact tag version.

## Documentation

This site is built with [VitePress](https://vitepress.dev) and auto-deploys to
GitHub Pages on every push to `main` that touches `docs/`. Source lives in the
[`docs/`](https://github.com/ChristopherTrimboli/linux-ai/tree/main/docs)
directory.
