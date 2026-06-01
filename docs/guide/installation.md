# Installation

## Prerequisites

- **Rust** stable 1.88 or newer (developed on 1.96). Install/upgrade with
  `rustup update stable`.
- For the **desktop app** only: Node.js 18+ and the Tauri Linux system libraries.

The CLI and core do **not** require any of the system libraries below — only a
Rust toolchain.

::: code-group

```bash [Debian/Ubuntu]
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libdbus-1-dev libasound2-dev \
  libayatana-appindicator3-dev librsvg2-dev pkg-config
```

```bash [Fedora]
sudo dnf install -y webkit2gtk4.1-devel openssl-devel curl wget file \
  libxdo-devel dbus-devel libappindicator-gtk3-devel librsvg2-devel alsa-lib-devel
```

:::

`libasound2-dev` / `alsa-lib-devel` is required for the microphone capture used by
[voice input](./voice).

## Build & run

### CLI

```bash
# from the repo root
cargo build --release -p linux-ai-cli      # builds ./target/release/ai
cargo install --path crates/cli            # installs `ai` onto your PATH
```

### Desktop app

```bash
cd desktop
npm install
npm run tauri dev        # dev build with hot reload
npm run tauri build      # produces a .deb and an AppImage under src-tauri/target/release/bundle
```

### Dock icon (GNOME / Ubuntu)

Installed `.deb` / AppImage builds register their own icon. When running
`npm run tauri dev` on **Wayland**, install a matching desktop entry so the dock
shows the real icon:

```bash
desktop/install-desktop-entry.sh            # install (re-run anytime)
desktop/install-desktop-entry.sh --uninstall
```
