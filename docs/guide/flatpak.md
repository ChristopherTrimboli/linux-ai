# Flatpak

A Flatpak manifest lives at
[`packaging/flatpak/dev.linux_ai.app.yml`](https://github.com/ChristopherTrimboli/linux-ai/blob/main/packaging/flatpak/dev.linux_ai.app.yml).

::: tip App ID
Flatpak application IDs cannot contain hyphens, so the id is `dev.linux_ai.app`
— the underscore-normalized form of the app's identifier `dev.linux-ai.app`.
:::

## Build & install locally

The manifest repacks the Tauri `.deb`, so build that first:

```bash
cd desktop
npm run tauri build -- --bundles deb
# copy/symlink the built deb next to the manifest as linux-ai.deb
cp src-tauri/target/release/bundle/deb/*.deb \
  ../packaging/flatpak/linux-ai.deb
```

Then build and install with `flatpak-builder`:

```bash
cd packaging/flatpak
flatpak install -y flathub org.gnome.Platform//47 org.gnome.Sdk//47
flatpak-builder --user --install --force-clean build-dir dev.linux_ai.app.yml
flatpak run dev.linux_ai.app
```

## Permissions & computer access

The manifest grants `--filesystem=host`, microphone access, network, and the
Secret Service (`org.freedesktop.secrets`) for keyring storage. This is
deliberately broad because the app's built-in tools act on your machine. To
sandbox it, remove `--filesystem=host` (the tools will then only see the app's
own data directory) and re-build.

## Flathub

The repacked-`.deb` manifest is meant for self-distribution. A Flathub
submission requires building from source with vendored Cargo/npm sources; that
manifest is not yet provided.
