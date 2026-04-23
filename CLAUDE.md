# D2R Show Me When — AI Navigation Guide

Quick-reference for reading only what you need. Never read a file end-to-end
unless making changes to it; use this map to find the right module first.

---

## Project in one sentence

Tauri 2 desktop app (macOS/Windows/Linux): global hotkeys trigger countdown
timers that appear as draggable canvas overlays on top of Diablo II: Resurrected.

---

## Frontend module map  `app/src/`

| File | Lines | What's inside | Read when… |
|------|-------|---------------|------------|
| `main.ts` | 7 | Entry point — imports CSS, calls bootstrapApp | Never, it's trivial |
| `types.ts` | 85 | All shared TypeScript types (TimerConfig, AppState, BootstrapPayload, …) | Adding/changing any type |
| `i18n.ts` | 153 | EN/RU translation tables, `t()`, `tf()`, `setLang()` | Editing UI strings or adding a language |
| `ctx.ts` | 63 | Single mutable `ctx` object — all runtime state | Understanding what state exists; never import from here to avoid cycles |
| `utils.ts` | 87 | Pure helpers: `activeProfile()`, `defaultTimer()`, `resolveAssetUrl()`, `escapeHtml()`, `resolveImportName()` | Touching profile/asset/string helpers |
| `hotkeys.ts` | 30 | Pure hotkey serialisation: `serializeHotkey()`, `normalizeKey()` | Changing hotkey string format |
| `render.ts` | 496 | `render()`, `renderTimerRowWrap()`, `renderExpandedSettings()`, `renderModal()`, `updateTimerRowDisplay()`, `setSaveMsg()` | Changing any UI layout/template |
| `actions.ts` | 336 | State mutations + Tauri IPC: timer CRUD, profile CRUD, saveState, openOverlays, toggleOverlayEditMode, handleTimerTick, encodeProfiles/decodeProfiles, autoSave, applyUiScale | Adding features, changing Rust command calls |
| `events.ts` | 312 | `bindEvents()`, `handleButtonClick()`, `handleGlobalKeydown()` | Adding/removing a data-action button or input binding |
| `bootstrap.ts` | 150 | `bootstrapApp()` — wires DI slots, listens to Rust events, restores lang/uiScale/window size, first render, auto-open | Changing startup sequence or Tauri event subscriptions |
| `styles.css` | 929 | All CSS — see section index below | Any styling change |
| `overlay.ts` | 361 | Standalone overlay window: canvas draw loop, timer tick listener, edit-mode | Changing the overlay appearance or its Tauri IPC |

### Dependency graph (no cycles)

```
types  ←  i18n
types  ←  ctx
types  ←  utils   ← ctx
types  ←  hotkeys (no internal deps)
types  ←  render  ← ctx, i18n, utils          (bindEvents injected at runtime)
types  ←  actions ← ctx, render, utils, i18n
types  ←  events  ← ctx, render, actions, i18n, hotkeys, utils
types  ←  bootstrap ← ctx, utils, render, events, actions
main   ← bootstrap
```

### Dependency-injection pattern (render ↔ events)

`render.ts` calls `bindEvents()` after every DOM replacement, but cannot
import `events.ts` directly (would create a cycle).  Solution:

1. `render.ts` exposes `setBindEventsCallback(fn)`.
2. `bootstrap.ts` calls `setBindEventsCallback(bindEvents)` once at startup.
3. Same pattern for modal Enter/Escape: `setModalCallbacks(confirm, cancel)` in
   `render.ts`, wired by `actions.wireModalCallbacks()` from `bootstrap.ts`.

---

## Backend module map  `app/src-tauri/src/`

| File | Lines | What's inside | Read when… |
|------|-------|---------------|------------|
| `lib.rs` | 1107 | All `#[tauri::command]` handlers, Tauri builder, `run()`, Win32 clip/cloak helpers | Adding/changing a Rust command |
| `core/models.rs` | 290 | Rust structs: TimerConfig, Profile, AppState, BootstrapPayload, OverlayTimerConfig, OverlayCapabilities, BackendAdapterInfo, PlatformInfo | Changing data model |
| `hotkeys/mod.rs` | 512 | Hotkey parsing (`parse_hotkey_string`), 3-thread listener: OS input → matching → timer tick | Changing hotkey logic or timing |
| `hotkeys/macos_tap.rs` | 410 | macOS CGEventTap (replaces rdev on macOS 14+ to avoid SIGTRAP) | macOS-specific input issues |
| `hotkeys/windows_keyboard.rs` | 144 | Windows GetAsyncKeyState polling (replaces rdev WH_KEYBOARD_LL hook which fails on some systems) | Windows hotkey issues |
| `storage/mod.rs` | 239 | `load_state`, `save_state`, `icons_from_resources` | Config persistence or icon loading |
| `platform/mod.rs` | 52 | `detect_platform()` → `PlatformInfo` (kind, arch, OverlayCapabilities, BackendAdapterInfo) | Platform detection |
| `platform/macos.rs` | 23 | macOS capability descriptor (Quartz, click-through, stage: implemented) | Platform detection |
| `platform/windows.rs` | 24 | Windows capability descriptor (rdev, no click-through, stage: experimental) | Platform detection |
| `platform/linux.rs` | 45 | Linux X11 + Wayland capability descriptors (stage: experimental / research) | Platform detection |

### Tauri commands (frontend → Rust)

| Command | Where called | What it does |
|---------|-------------|--------------|
| `get_bootstrap_payload` | bootstrap.ts | Returns state + icons + platform info |
| `save_app_state` | actions.ts `saveState()` | Persist AppState to disk |
| `update_hotkey_registrations` | actions.ts, events.ts | Sync per-timer hotkeys with Rust thread |
| `update_global_hotkeys` | actions.ts, bootstrap.ts | Sync hide/show + layout-edit hotkeys |
| `trigger_timer` | events.ts `test-timer` | Manually fire a timer |
| `open_overlays` | actions.ts | Create/resize/show all overlay windows |
| `close_overlays` | actions.ts | Hide all overlay windows |
| `show_overlay` / `hide_overlay` | overlay.ts | Toggle single overlay window |
| `set_watching` | actions.ts | Enable/disable hotkey firing |
| `set_overlays_edit_mode` | actions.ts | Toggle drag mode on overlays |
| `get_overlay_config` | overlay.ts | Fetch per-timer config for canvas draw |
| `get_active_timer_state` | overlay.ts | Poll remaining time (fallback) |
| `update_overlay_hotkey_labels` | events.ts, bootstrap.ts | Update `show_hotkey_labels` in all overlay configs and broadcast `hotkey-label-changed` via Rust `app.emit()` |
| `apply_circular_clip` | overlay.ts | Apply Win32 circular window clip to overlay (Windows only; no-op on other platforms) |
| `stop_all_timers` | events.ts profile switch | Clear all running timers |
| `reset_input_monitoring_permission` | events.ts | macOS: reset TCC + re-request permission |
| `open_privacy_settings` | events.ts | macOS: open System Settings |

### Rust → frontend events

| Event | Emitted from | Handled in |
|-------|-------------|-----------|
| `timer_tick` | hotkeys/mod.rs tick thread | bootstrap.ts → `handleTimerTick` |
| `overlay-positions-saved` | lib.rs `set_overlays_edit_mode` | bootstrap.ts → saves positions |
| `hotkey_listener_error` | hotkeys/mod.rs | bootstrap.ts → sets `ctx.listenerError` |
| `hotkey_action` | hotkeys/mod.rs matching thread | bootstrap.ts → hide_show / layout_edit |
| `config-updated` | lib.rs `open_overlays` | overlay.ts → reloads config |
| `edit-mode-changed` | lib.rs `set_overlays_edit_mode` | overlay.ts → shows/hides border |
| `hotkey-label-changed` | lib.rs `update_overlay_hotkey_labels` | overlay.ts → shows/hides hotkey label |

---

## CSS section index  `app/src/styles.css`

| Lines | Section |
|-------|---------|
| 1–35 | CSS variables (colours, font) + reset + body |
| 36–152 | App shell + titlebar + language selector (drag region, platform-macos/windows offsets) |
| 153–219 | Buttons (.btn, .primary, .danger, .sm, .icon-only) |
| 220–264 | Profile bar + profile-actions-bar |
| 265–485 | Timer list header + timer rows (grid, duration cell, countdown, hotkey button, blink animation) |
| 486–540 | Toggle switch |
| 541–637 | Expanded timer settings (3-col grid, icon gallery) |
| 638–645 | Add-timer row |
| 646–663 | Overlay bar |
| 664–751 | Global controls section (.section-header-global, .control-row, .set-key-btn) |
| 752–784 | Status / save bar |
| 785–853 | Inline modal (profile create / rename / delete confirm) |
| 854–867 | Auto-show label in overlay bar |
| 868–898 | Listener error banner |
| 899–929 | Attribution + scrollbar + responsive |

Key grid definitions:
- `.timer-list-header` / `.timer-row`: `20px 28px 108px 54px 112px 1fr 1fr 38px 26px 26px`
- `.section-header-global` / `.control-row`: `1fr 34px 140px 140px`

---

## Shared state reference  `ctx.*`

```
ctx.bootstrap          — BootstrapPayload from Rust (icons, platform, storagePath)
ctx.state              — AppState (profiles, global hotkeys, flags)
ctx.selectedProfile    — currently visible profile name (may differ from activeProfile)

ctx.capturingHotkeyIndex   — timer row index being captured (null = not capturing)
ctx.capturingHotkeyField   — 'hotkey' | 'hotkey2'
ctx.capturingGlobalHotkey  — GlobalHotkeyField | null

ctx.expandedTimerIndex — which timer row has settings panel open
ctx.modalKind          — current inline modal ('none' = closed)
ctx.modalInputValue    — live value of modal text input / import textarea
ctx.exportBase64       — computed base64 for export-config modal

ctx.saveMsg / ctx.saveMsgKind   — bottom status bar content
ctx.listenerError               — macOS Input Monitoring error (survives renders)

ctx.overlaysOpen       — whether overlay windows are visible
ctx.overlayEditMode    — whether drag-to-reposition mode is active
ctx.watching           — whether hotkeys are actively firing timers
ctx.showHotkeyLabels   — whether overlays show hotkey label text
ctx.uiScale            — UI zoom level (100 = default); applied as CSS zoom and saved to AppState

ctx.iconMap            — Map<fileName, IconAsset>
ctx.runningTimers      — Map<timerIndex, {remainingSecs, totalSecs, blinking}>
ctx.root               — #app DOM element
ctx.globalKeydownBound — ensures keydown listener registered exactly once
```

---

## Common tasks → which files to touch

| Task | Files |
|------|-------|
| Change a UI label / add translation key | `i18n.ts` |
| Add a new timer property | `types.ts`, `ctx` models.rs, `render.ts` (row template), `actions.ts` (mutations) |
| Add a new button/action | `render.ts` (HTML), `events.ts` (`handleButtonClick` case) |
| Add a new Rust command | `lib.rs`, then call via `invoke()` in `actions.ts` or `events.ts` |
| Change overlay appearance | `overlay.ts` (`draw()` function) |
| Change hotkey string format | `hotkeys.ts` + `hotkeys/mod.rs` `parse_key_name` |
| Change config persistence path | `storage/mod.rs` |
| Fix macOS input monitoring | `hotkeys/macos_tap.rs`, `lib.rs` `reset_input_monitoring_permission` |
| Fix Linux hotkey issues | `hotkeys/mod.rs` Linux branch (rdev error message), `i18n.ts` (`msgLinuxInputHint`) |
| Resize a column | `styles.css` grid definition (see section index above) |
| Add a new language | `i18n.ts` — add a key to both `en` and `ru` objects (TS enforces symmetry via `as const`) |

---

## Windows notes

- **Custom titlebar**: `set_decorations(false)` is called in `setup()` to remove the native
  OS titlebar. The app uses a custom HTML titlebar with `data-tauri-drag-region` for dragging
  and `data-action="win-minimize"` / `data-action="win-close"` buttons. These buttons use
  `pointerdown` (not `click`) so they fire before Tauri's drag handler can intercept. The
  `.titlebar-drag-fill` spacer uses only the HTML attribute (no `-webkit-app-region: drag`
  CSS) to prevent WebView2 compositor-level drag regions from swallowing button events.
- **Tauri capabilities ACL**: `core:default` does NOT include all window operations.
  The following must be added explicitly to `capabilities/default.json` or they silently
  fail with no JS exception: `core:window:allow-minimize`, `core:window:allow-close`,
  `core:window:allow-set-size`. Any new `getCurrentWindow().*()` call that fails without
  an error should be the first thing to check.
- **Overlay creation**: Windows are pre-created (hidden) via `create_single_overlay_window`
  called from `open_overlays`. `ensure_overlay_window` (used in `show_overlay` and
  `set_overlays_edit_mode`) returns the existing window or creates one as a fallback.
  `WebviewWindowBuilder::build()` cannot be called from **synchronous** Tauri command
  handlers on Windows — it dispatches to the main event loop, which deadlocks (IPC
  calls stay "Pending" forever). Safe call sites: `setup()` (runs before the event loop)
  and async command handlers. `show_overlay` is a fire-and-forget `void invoke(...)`
  from JS (async), so it is safe.
- **Overlay transparency**: Each overlay window is created with `.transparent(true)` and
  `.background_color(tauri::webview::Color(0, 0, 0, 0))` to suppress WebView2's default
  white background. `apply_win32_circular_clip` (in `lib.rs`) clips the Win32 window to
  its inscribed circle via `SetWindowRgn + CreateEllipticRgn` (raw `extern "system"` —
  no extra crate needed). The physical pixel width is reported by overlay JS via the
  `apply_circular_clip` command (always correct in WebView2). The inset matches the 8
  logical-px canvas padding so the Win32 region and the CSS `clip-path` circle align exactly.
- **Click-through**: Disabled on Windows (`set_overlay_clickthrough` is a no-op).
  The overlay windows receive mouse events but are otherwise functional.
- **Exit/hang**: Overlay windows are explicitly closed before `exit(0)` to release
  the WebView2 user-data lock, preventing the next launch from hanging. On Windows a
  400 ms delay thread is used instead of an immediate `exit(0)` to let WebView2 clean up.
- **DevTools**: Auto-opened in debug builds (`open_devtools()` in `setup()`).
  Cargo feature `devtools` is enabled in Cargo.toml.

**Running locally on Windows:**
```bash
cd app && npm install && npm run tauri:dev
```
DevTools open automatically. Check the Console tab for JS errors on any action.
The Rust log goes to stdout in the terminal running `tauri:dev`.

**Config location on Windows:** `%APPDATA%\com.vanyushkin.d2rshowmewhen\profiles.json`

---

## Linux notes

- **Global hotkeys**: rdev uses `/dev/input/event*`. On most distros the user must be in the `input` group (`sudo usermod -aG input $USER`). If rdev fails to start, `hotkey_listener_error` is emitted and the red banner appears with the fix instruction.
- **Wayland**: `detect_platform()` returns `kind = "linux-wayland"` when `WAYLAND_DISPLAY` is set. `bootstrap.ts` detects this and sets `ctx.listenerError` immediately (before the hotkey thread even starts) using `t('msgWaylandError')`. No buttons are shown — the only fix is to switch to X11.
- **Overlay transparency**: requires a running compositor (picom, KWin, Mutter, etc.). Without one, WebKit2GTK draws overlays on a solid background. The Linux error banner footer (shown when rdev fails) includes `t('msgLinuxCompositorHint')` as a proactive note.
- **Steam Deck (gamescope)**: D2R under gamescope may not allow Tauri windows above the game. Not validated on real hardware. Desktop Mode (X11, no gamescope) is the supported path.
- **Click-through**: disabled on Linux (same as Windows). `set_overlay_clickthrough` is a no-op.
- **DevTools**: available in debug builds via `open_devtools()` same as other platforms.

**Running locally on Linux:**
```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev libayatana-appindicator3-dev
cd app && npm install && npm run tauri:dev
```

**Config location on Linux:** `~/.local/share/com.vanyushkin.d2rshowmewhen/profiles.json`

---

## Dev notes

**Titlebar**: The main window uses `titleBarStyle: "Overlay"` and `hiddenTitle: true`
(macOS) / `set_decorations(false)` (Windows) so both platforms use the custom HTML
titlebar. `bootstrap.ts` adds `platform-macos` to `<body>`; CSS rule
`.platform-macos .titlebar { padding-left: 78px }` reserves space for the traffic-light
buttons. On Linux the native titlebar remains. The `.app-title` span carries
`data-tauri-drag-region` (HTML attribute, required by Tauri 2); `.titlebar-drag-fill`
also has `data-tauri-drag-region` but deliberately omits `-webkit-app-region: drag`
(CSS) to avoid WebView2 compositor drag regions that would swallow adjacent button clicks
on Windows. `.titlebar-right` has `-webkit-app-region: no-drag` to keep all controls
interactive.

**macOS close button**: The red traffic-light button calls `app_handle.exit(0)` via
the `CloseRequested` window event handler in `lib.rs`. This fully quits the process
(overlays included). ⌘Q also works through the normal macOS app-quit path.

**Overlay window lifecycle**: Overlay windows are pre-created (hidden) when
`open_overlays` is called. They remain hidden until `show_overlay` is called by
`handleTimerTick` in actions.ts. The `open_overlays` command must be called before
any timer can trigger an overlay — this is done automatically by `openOverlays()` in
actions.ts, which sets `ctx.overlaysOpen = true`. The "Edit Layout" button also
forces all overlay windows visible (for positioning), regardless of timer state.

**Icon display during `cargo tauri dev`**: In dev mode Tauri doesn't bundle
resources, so `resolve_icon_dir()` falls back to `app/assets/icons/` via an
absolute path. `convertFileSrc()` converts that to an `asset://` URL under
`$HOME`. The production config restricts asset protocol scope to
`$RESOURCE/**` and `$APPDATA/**` only, so icon images won't render during
`cargo tauri dev`. To restore them temporarily, add `"$HOME/**"` to the
`assetProtocol.scope` array in `tauri.conf.json` (don't commit it).

---

## Developer docs  `docs/`

| File | Contents |
|------|----------|
| `docs/architecture.md` | Tech stack, directory layout, frontend dependency graph, hotkey threads, overlay lifecycle, platform model |
| `docs/developer-guide.md` | Requirements, dev setup, production build, CI/cross-compilation, release checklist |
| `docs/user-guide.md` | Config paths, hotkey format reference, migration from Python app, known limitations |
