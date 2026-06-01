# Arch Linux (AUR)

Arch users can install the desktop app from the AUR. The package is a **binary**
package (`-bin`): it repacks the official `.deb` from the
[GitHub release](./releases), so there is no long compile.

## Install

```bash
# with an AUR helper
yay -S linux-ai-bin      # or: paru -S linux-ai-bin
```

Or manually:

```bash
git clone https://aur.archlinux.org/linux-ai-bin.git
cd linux-ai-bin
makepkg -si
```

## CLI only

The `lai` command line is published to crates.io, so you don't need the AUR for
it:

```bash
cargo install linux-ai-cli   # installs `lai`
```

## Maintaining the package

The recipe lives in the repo at
[`packaging/aur/PKGBUILD`](https://github.com/ChristopherTrimboli/linux-ai/blob/main/packaging/aur/PKGBUILD).
To publish a new version:

1. Bump `pkgver` to match the released tag (without the leading `v`).
2. Regenerate the metadata: `makepkg --printsrcinfo > .SRCINFO`.
3. Commit `PKGBUILD` + `.SRCINFO` to the AUR git remote.

`sha256sums` are `SKIP`ped because the binaries are versioned by tag; pin them if
you prefer stricter integrity checks.
