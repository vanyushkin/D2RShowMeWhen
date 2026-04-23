# D2R Show Me When

Countdown timer overlay for Diablo II: Resurrected. Press a hotkey — a timer appears on screen above the game.

## Features

- Global hotkeys (keyboard + mouse buttons) trigger countdown timers
- Draggable always-on-top overlay windows per timer
- Multiple profiles for different builds or characters
- EN / RU interface
- Windows, Linux, macOS

## Download

Go to [**Releases**](https://github.com/vanyushkin/D2RShowMeWhen/releases/latest) and grab the file for your platform:

| Platform | File |
|----------|------|
| Windows | `D2RShowMeWhen_x.x.x_x64_en-US.msi` |
| Linux / Steam Deck (AppImage) | `D2RShowMeWhen_x.x.x_amd64.AppImage` |
| Linux (deb) | `D2RShowMeWhen_x.x.x_amd64.deb` |
| macOS | `D2RShowMeWhen_x.x.x_aarch64.dmg` *(uploaded after each release)* |

## Installation

### Windows

Double-click the `.msi` file and follow the installer.

### Steam Deck

> **The app runs in Desktop Mode only.** Global hotkeys require X11 — they are blocked by Gamescope (Game Mode).

1. Hold **Power** → **Switch to Desktop**
2. Open **Firefox** → go to [Releases](https://github.com/vanyushkin/D2RShowMeWhen/releases/latest) → download the `.AppImage` file
3. Open **Dolphin** (file manager on the taskbar) → go to Downloads
4. Right-click the `.AppImage` → **Properties** → **Permissions** tab → tick **"Is executable"** → OK
5. Create a folder `Applications` in your home folder and move the AppImage there
6. Double-click to launch — no installation needed

**Optional — add to Steam library:**
In Steam (Desktop Mode) → **Games** → **Add a Non-Steam Game** → browse to the AppImage → **Add Selected Programs**.
Rename to "D2R Show Me When" in library properties.

### macOS

The app is not signed or notarized. After copying `D2RShowMeWhen.app` to `/Applications`, run once in Terminal:

```bash
sudo xattr -d com.apple.quarantine /Applications/D2RShowMeWhen.app
```

Then double-click to open. On first launch, grant **Input Monitoring** permission when prompted:
System Settings → Privacy & Security → Input Monitoring → add the app.

## Hotkey Format

| Input | Example |
|-------|---------|
| Single key | `f3`, `f9`, `space`, `enter` |
| Key + modifier | `ctrl+f3`, `alt+shift+f5`, `cmd+1` |
| Mouse button | `mouse_left`, `mouse_right`, `mouse_x1`, `mouse_x2` |

Modifiers: `ctrl`, `alt`, `shift`, `cmd` (macOS) / `win` (Windows)

> Mouse button hotkeys must be typed manually in the input field — the UI recorder only captures keyboard combos.

## Build from Source

**Requirements:** [Rust](https://rustup.rs/) 1.77+, Node.js 18+, Tauri CLI (`cargo install tauri-cli --version "^2"`)

**Linux additional packages:**
```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev libayatana-appindicator3-dev
```

**Build:**
```bash
cd app
npm install
npm run tauri:build
```

Output: `app/src-tauri/target/release/bundle/`

## Platform Support

| Platform | Status |
|----------|--------|
| **macOS** (Apple Silicon, 14+) | ✅ Tested by developer |
| **Windows** (10 / 11, x64) | ✅ Tested by developer |
| **Linux / Steam Deck** | ⚠️ Community-supported — not tested by developer |

Linux builds are provided and the code paths are in place, but overlay behaviour, hotkey reliability, and Steam Deck gamescope compatibility have not been validated on real hardware by the author. Bug reports and PRs from Linux users are very welcome.

## Known Limitations

- **Steam Deck / Wayland**: global hotkeys require X11. Use Desktop Mode on Steam Deck.
- **macOS unsigned**: Input Monitoring permission must be granted manually. The app shows a red banner if the hotkey listener fails to start.
- **Mouse hotkey recording**: the UI recorder captures keyboard only. Type mouse hotkeys manually (e.g. `mouse_x1`).

## License

[MIT](LICENSE)
