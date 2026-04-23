# D2R Show Me When — Developer Guide

## Requirements

### All Platforms

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.77+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | 18+ | https://nodejs.org |
| Tauri CLI | 2.x | `cargo install tauri-cli --version "^2"` |

### macOS

- Xcode Command Line Tools: `xcode-select --install`
- No extra libraries — WKWebView is bundled with macOS

### Windows

- WebView2 is pre-installed on Windows 10 (1803+) and Windows 11
- Visual Studio Build Tools 2022 with "Desktop development with C++" workload:
  ```
  winget install Microsoft.VisualStudio.2022.BuildTools
  ```

### Linux

```bash
# Ubuntu / Debian
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
                 libssl-dev libayatana-appindicator3-dev

# Arch / SteamOS
sudo pacman -S webkit2gtk-4.1 gtk3 librsvg openssl

# Steam Deck (SteamOS — run inside Desktop Mode)
sudo steamos-readonly disable
sudo pacman -S webkit2gtk-4.1 gtk3 librsvg openssl
sudo steamos-readonly enable
```

**Global hotkeys on Linux** use rdev which reads `/dev/input/event*`. The user must be in the `input` group:
```bash
sudo usermod -aG input $USER   # then log out and back in
```

## Running in Dev Mode

```bash
cd app
npm install
npm run tauri:dev
```

The app opens with hot-reload for the frontend. Rust changes require a full restart. DevTools open automatically in debug builds — check the Console for JS errors.

**Config location during dev** is the same as production (see [user-guide.md](user-guide.md)).

### macOS — Input Monitoring Permission

On first dev launch macOS will prompt for Input Monitoring. If the prompt doesn't appear:

1. System Settings → Privacy & Security → Input Monitoring
2. Click **+** → add the `cargo-tauri` process (shown in the running terminal)
3. Restart `npm run tauri:dev`

Without this permission hotkeys silently don't fire and the app shows a red banner.

### Icon Assets During Dev

In dev mode Tauri doesn't bundle resources, so icons fall back to `app/assets/icons/` via an absolute path. `convertFileSrc()` converts it to an `asset://` URL. The production `tauri.conf.json` restricts asset protocol scope to `$RESOURCE/**` and `$APPDATA/**`, so icons won't render in dev unless you temporarily add `"$HOME/**"` to `assetProtocol.scope` in `tauri.conf.json` (don't commit this).

## Production Build

```bash
cd app
npm run tauri:build
```

Output in `app/src-tauri/target/release/bundle/`:

| Platform | Output |
|----------|--------|
| macOS    | `macos/D2RShowMeWhen.app` + `.dmg` |
| Windows  | `msi/D2RShowMeWhen_*.msi` |
| Linux    | `deb/d2rshowmewhen_*.deb` + `appimage/D2RShowMeWhen_*.AppImage` |

## CI / Cross-Compilation

macOS builds are produced locally (no codesigning in CI). Windows and Linux are built on GitHub Actions automatically on every push to `main` that bumps the version.

**macOS → Windows / Linux** cross-compilation is not set up. Use:
- A Windows or Linux machine, or
- GitHub Actions `windows-latest` / `ubuntu-latest` runners

The `.github/workflows/` directory contains the current CI definitions.

To upload a locally built macOS DMG to an existing release:
```bash
bash scripts/upload-macos-release.sh v1.0.x
```

## Release Checklist

1. Bump the version in all three files (must match):

   | File | Field |
   |------|-------|
   | `app/package.json` | `"version"` |
   | `app/src-tauri/Cargo.toml` | `version = "..."` |
   | `app/src-tauri/tauri.conf.json` | `"version"` |

2. Run a production build to verify no errors:
   ```bash
   cd app && npm run tauri:build
   ```

3. Commit with message `chore: release x.x.x` and push — CI builds Windows + Linux and creates the GitHub release automatically.

4. Wait for CI to finish, then upload the macOS DMG:
   ```bash
   bash scripts/upload-macos-release.sh vx.x.x
   ```

## Code Map

For a detailed per-file module map, dependency graph, all Tauri commands, and Rust ↔ frontend event table see `CLAUDE.md` at the project root.

For the high-level architecture (threads, overlay lifecycle, platform model) see [architecture.md](architecture.md).
