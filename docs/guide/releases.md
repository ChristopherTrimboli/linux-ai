# Releases & versioning

Pushing a version tag builds and publishes a GitHub Release with production
artifacts for both **x86_64** and **aarch64** Linux (CLI tarball + `.deb` +
AppImage per arch):

```bash
git tag v0.1.0
git push origin v0.1.0
```

You can also trigger the workflow manually from the Actions tab (provide the tag
as input).

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
