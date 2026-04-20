# D2R Show Me When — Build & Run Guide

Based on D2R Show Me When v3.0 by GlassCannon — rewritten in Tauri/Rust for
cross-platform support (macOS Apple Silicon, Windows, Linux/Steam Deck).
Current release: **1.0.0**

---

## Requirements

### All platforms
| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.77+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | 18+ | https://nodejs.org |
| npm | bundled with Node | — |
| Tauri CLI | 2.x | `cargo install tauri-cli --version "^2"` |

### macOS (Apple Silicon — primary target)
- Xcode Command Line Tools: `xcode-select --install`
- No extra libraries needed — WKWebView is bundled with macOS

### Windows
- WebView2 is pre-installed on Windows 10 (1803+) and Windows 11
- Visual Studio Build Tools 2022 with "Desktop development with C++" workload  
  (needed to compile Rust with MSVC target)
- Or install via `winget install Microsoft.VisualStudio.2022.BuildTools`

### Linux / Steam Deck
```bash
# Ubuntu / Debian
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
                 libssl-dev libayatana-appindicator3-dev

# Arch / SteamOS
sudo pacman -S webkit2gtk-4.1 gtk3 librsvg openssl

# Steam Deck (SteamOS — use pacman inside desktop mode)
sudo steamos-readonly disable
sudo pacman -S webkit2gtk-4.1 gtk3 librsvg openssl
sudo steamos-readonly enable
```

**Note (Linux):** rdev (global hotkeys) uses X11 on Steam Deck/Gamescope.
Wayland restricts global input monitoring by design. If your session is Wayland,
hotkeys may not fire; use `XDG_SESSION_TYPE=x11` to force X11 fallback.

---

## Development

```bash
cd app
npm install
npm run tauri:dev
```

The app will open with hot-reload. Rust changes require a restart.

### macOS — Input Monitoring permission (required for global hotkeys)

On first launch the OS will prompt for Input Monitoring permission.
If it doesn't appear automatically:

1. Open **System Settings → Privacy & Security → Input Monitoring**
2. Click **+** and add the app (or `cargo tauri dev` process during development)
3. Restart the app

Without this permission hotkeys will silently not fire. The app shows a red
banner if the rdev listener fails to start.

---

## Production Build

```bash
cd app
npm run tauri:build
```

Output is in `app/src-tauri/target/release/bundle/`:

| Platform | Output |
|----------|--------|
| macOS    | `macos/D2RShowMeWhen.app` + `.dmg` |
| Windows  | `msi/D2RShowMeWhen_*.msi` + `nsis/D2RShowMeWhen_*-setup.exe` |
| Linux    | `deb/d2rshowmewhen_*.deb` + `appimage/D2RShowMeWhen_*.AppImage` |

The macOS `.app` bundle is self-contained (no installer needed — just copy to
`/Applications`). The `.dmg` is the standard distribution format.

The Windows `.exe` from NSIS is a standalone installer. After install the app
runs without any extra dependencies — WebView2 is embedded via bootstrapper.

The Linux `.AppImage` is the most portable single-file option (no install
needed — just `chmod +x` and run).

---

## Cross-Compilation

**macOS → Windows** is not supported directly due to MSVC linker requirements.
Options:

1. **Build on Windows** — use any Windows 10/11 machine with the toolchain above
2. **GitHub Actions** — use a `windows-latest` runner (free for public repos):

```yaml
# .github/workflows/build.yml
jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: '20' }
      - uses: dtolnay/rust-toolchain@stable
      - run: cd app && npm ci && npm run tauri:build
      - uses: actions/upload-artifact@v4
        with:
          name: windows-build
          path: app/src-tauri/target/release/bundle/
```

**macOS → Linux** is theoretically possible with a cross-compilation toolchain
but is not set up in this project. Use a Linux machine or a GitHub Actions
`ubuntu-latest` runner with the packages from the Requirements section.

---

## Config File Location

Profiles and settings are stored at:

| Platform | Path |
|----------|------|
| macOS    | `~/Library/Application Support/com.vanyushkin.d2rshowmewhen/profiles.json` |
| Windows  | `%APPDATA%\com.vanyushkin.d2rshowmewhen\profiles.json` |
| Linux    | `~/.local/share/com.vanyushkin.d2rshowmewhen/profiles.json` |

The config is plain JSON — you can edit it manually or back it up freely.

### Migration from the old Python app (macOS only)

On first launch the app automatically imports profiles from the legacy macOS
Python app if the file exists at:
- `~/Library/Application Support/D2R_Show_Me_When_Mac/profiles.json`

---

## Hotkey Format Reference

Hotkeys are stored as lowercase strings. Supported formats:

| Input | Example |
|-------|---------|
| Single key | `f3`, `f9`, `space`, `enter` |
| Key + modifier(s) | `ctrl+f3`, `cmd+1`, `alt+shift+f5` |
| Mouse button | `mouse_left`, `mouse_right`, `mouse_middle`, `mouse_x1`, `mouse_x2` |

Modifier names: `ctrl`, `alt`, `shift`, `cmd` (macOS) / `win` (Windows)

**Note:** Mouse button hotkeys are stored and fired correctly, but the UI hotkey
recorder only captures keyboard combos. To use a mouse button, type the hotkey
string manually (e.g. `mouse_x1`) directly in a text field — the UI recorder
does not support mouse input.

---

## Architecture Notes

```
app/
├── src/                    TypeScript frontend (Vite)
│   ├── main.ts             Entry point — imports CSS, calls bootstrapApp()
│   ├── bootstrap.ts        Startup: wires DI, subscribes Tauri events, first render
│   ├── ctx.ts              Single mutable ctx object — all runtime state
│   ├── types.ts            All shared TypeScript types
│   ├── i18n.ts             EN/RU translations: t(), tf(), setLang()
│   ├── utils.ts            Pure helpers (activeProfile, defaultTimer, …)
│   ├── hotkeys.ts          Hotkey serialisation (no internal deps)
│   ├── render.ts           render(), sub-renderers, setSaveMsg()
│   ├── actions.ts          State mutations + all Tauri invoke() calls
│   ├── events.ts           bindEvents(), handleButtonClick(), handleGlobalKeydown()
│   ├── overlay.ts          Standalone overlay window: canvas draw loop, edit-mode
│   └── styles.css          Compact dark theme
└── src-tauri/
    └── src/
        ├── hotkeys/mod.rs  Hotkey listener + timer state machine (3 threads)
        ├── hotkeys/macos_tap.rs  macOS CGEventTap (replaces rdev on macOS 14+)
        ├── core/models.rs  Domain types (TimerConfig, AppState, …)
        ├── storage/mod.rs  Profile load/save, migration, icon discovery
        ├── platform/       Platform detection (macOS/Windows/Linux)
        └── lib.rs          Tauri commands, app entry point
```

The hotkey listener runs in three threads:
1. **OS capture** — raw events via CGEventTap (macOS) / SetWindowsHookEx (Windows) / X11 (Linux)
2. **hotkey matching** — checks fired events against registered hotkeys, starts timers
3. **tick loop** — emits `timer_tick` events every ~250 ms to drive the UI countdown

Overlay windows are always-on-top transparent canvas windows that render running
timers on top of the game. Each timer gets its own window; positions are
drag-saved per-profile. See `overlay.ts` and `lib.rs` (`open_overlays` command).

For the full module map and dependency graph see `CLAUDE.md` at the project root.

---

## Release Checklist

Before tagging a release, bump the version in all three places (they must match):

| File | Field |
|------|-------|
| `app/package.json` | `"version"` |
| `app/src-tauri/Cargo.toml` | `version = "..."` |
| `app/src-tauri/tauri.conf.json` | `"version"` |

All three must match before tagging. Then run a production build to verify:

```bash
cd app
npm run tauri:build
```

---

## Known Limitations

- **Wayland**: global hotkeys are restricted by the Wayland security model.
  Use X11 session or Gamescope on Steam Deck.
- **macOS unsigned builds**: Input Monitoring requires manual grant in System Settings.
  A notarized build would prompt automatically.
- **Mouse button hotkey recording**: the UI recorder only captures keyboard combos.
  Mouse button hotkeys must be typed manually (e.g. `mouse_x1`).
- **macOS window style**: The app uses `titleBarStyle: Overlay` so the native traffic-light
  buttons float over the app's own titlebar. On Windows and Linux the native titlebar sits
  above the WebView as normal (no custom titlebar padding is applied on those platforms).
- **macOS close button**: Clicking the red close button (or pressing ⌘Q) fully quits the
  app. There is no hide-to-background/dock behaviour.
