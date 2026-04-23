# D2R Show Me When — User Guide

## Config File Location

Profiles and all settings are stored as plain JSON. You can back it up, move it between machines, or edit it manually.

| Platform | Path |
|----------|------|
| macOS    | `~/Library/Application Support/com.vanyushkin.d2rshowmewhen/profiles.json` |
| Windows  | `%APPDATA%\com.vanyushkin.d2rshowmewhen\profiles.json` |
| Linux    | `~/.local/share/com.vanyushkin.d2rshowmewhen/profiles.json` |

To back up: copy `profiles.json` somewhere safe.  
To restore: replace `profiles.json` and restart the app.

## Hotkey Format Reference

Hotkeys are stored as lowercase strings. Supported formats:

| Input | Example |
|-------|---------|
| Single key | `f3`, `f9`, `space`, `enter` |
| Key + modifier(s) | `ctrl+f3`, `cmd+1`, `alt+shift+f5` |
| Mouse button | `mouse_left`, `mouse_right`, `mouse_middle`, `mouse_x1`, `mouse_x2` |

Modifier names: `ctrl`, `alt`, `shift`, `cmd` (macOS) / `win` (Windows)

### Mouse Button Hotkeys

The UI hotkey recorder only captures keyboard combos. To assign a mouse button hotkey, type the string directly into the hotkey field (e.g. `mouse_x1`) — it will be accepted and will fire correctly.

## Migration from the Legacy Python App (macOS only)

On first launch the app automatically imports profiles from the old Python-based D2R Show Me When if the file exists at:

```
~/Library/Application Support/D2R_Show_Me_When_Mac/profiles.json
```

No manual action required. If migration ran, the profiles appear in the profile selector immediately.

## Known Limitations

- **Wayland**: global hotkeys are blocked by the Wayland security model. Use an X11 session. On Steam Deck use Desktop Mode (which runs X11).
- **macOS unsigned builds**: the app is not notarized. Input Monitoring permission must be granted manually in System Settings → Privacy & Security → Input Monitoring. The app shows a red banner with instructions if the hotkey listener fails to start.
- **Mouse button hotkey recording**: the UI recorder captures keyboard only. Mouse button hotkeys must be typed manually as described above.
- **macOS close button**: clicking the red traffic-light button (or pressing ⌘Q) fully quits the app — there is no hide-to-dock behaviour.
- **Linux overlay transparency**: requires a running compositor (picom, KWin, Mutter, etc.). Without one, WebKit2GTK draws overlays on a solid white background.
- **Steam Deck under Gamescope (Game Mode)**: overlay windows may not appear above the game. Desktop Mode is the supported path.
