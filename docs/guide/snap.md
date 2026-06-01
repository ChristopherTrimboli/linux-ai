# Snap package

The repo ships a [`snap/snapcraft.yaml`](https://github.com/ChristopherTrimboli/linux-ai/blob/main/snap/snapcraft.yaml)
so the desktop app can be built and distributed as a snap.

## Install

Once published to the Snap Store:

```bash
sudo snap install linux-ai
```

::: warning Confinement & computer access
The snap uses **strict confinement**, which sandboxes the app. The built-in
"computer access" tools (`run_shell`, `read_file`, `write_file`, `search_files`)
are therefore limited to your **home directory** and the interfaces the snap is
allowed to use. For unrestricted access to your machine, install the **`.deb` or
AppImage** from the [GitHub releases](./releases) instead.
:::

After install, connect the optional interfaces you want:

```bash
snap connect linux-ai:audio-record      # microphone for voice input
snap connect linux-ai:removable-media    # access /media, /mnt
```

`network`, `home`, `audio-playback`, `opengl` and the GNOME desktop interfaces
are auto-connected.

## Building locally

```bash
sudo snap install snapcraft --classic
sudo snap install lxd && sudo lxd init --auto    # build backend
snapcraft                                        # produces linux-ai_*.snap
sudo snap install ./linux-ai_*.snap --dangerous
```

## Publishing

CI builds the snap on every push and can publish automatically. To enable it:

1. Register the name once: `snapcraft register linux-ai`.
2. Export store credentials and save them as the repo secret
   `SNAPCRAFT_STORE_CREDENTIALS`:

   ```bash
   snapcraft export-login --snaps linux-ai \
     --acls package_access,package_push,package_update,package_release -
   ```

With that secret set, the [`Snap` workflow](https://github.com/ChristopherTrimboli/linux-ai/blob/main/.github/workflows/snap.yml)
publishes pushes to `main` to the **edge** channel and version tags to **stable**.

## Full access alternative

If you want the app to act on your whole system without the snap sandbox, either:

- install the **`.deb` / AppImage** (recommended for power use), or
- change `confinement: strict` to `confinement: classic` in `snap/snapcraft.yaml`
  — classic snaps are unconfined but require manual review/approval from the Snap
  Store team, and must be installed with `--classic`.
