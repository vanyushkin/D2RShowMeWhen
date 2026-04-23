# D2R Show Me When — Architecture

## Overview

Tauri 2 desktop app. A TypeScript/Vite frontend renders the settings UI and overlay windows; a Rust backend handles global hotkeys, timer state, config persistence, and platform-specific window management.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| UI framework | Tauri 2 (WebView2 / WKWebView / WebKit2GTK) |
| Frontend | TypeScript + Vite (no JS framework) |
| Backend | Rust 1.77+ |
| Global hotkeys | CGEventTap (macOS) / GetAsyncKeyState polling (Windows) / rdev X11 (Linux) |
| Persistence | JSON (`profiles.json`) via `dirs` crate |
| i18n | Static EN/RU tables in `app/src/i18n.ts` |

## Directory Layout

```
app/
├── src/                        TypeScript frontend (Vite)
│   ├── main.ts                 Entry: imports CSS, calls bootstrapApp()
│   ├── types.ts                All shared TS types
│   ├── ctx.ts                  Single mutable runtime state object
│   ├── i18n.ts                 EN/RU translations: t(), tf(), setLang()
│   ├── utils.ts                Pure helpers (activeProfile, defaultTimer, …)
│   ├── hotkeys.ts              Hotkey serialisation (no internal deps)
│   ├── render.ts               render(), sub-renderers, DOM templates
│   ├── actions.ts              State mutations + all Tauri invoke() calls
│   ├── events.ts               bindEvents(), button click and keydown handlers
│   ├── bootstrap.ts            Startup: wires DI, subscribes Tauri events, first render
│   ├── overlay.ts              Standalone overlay window: canvas draw loop, edit-mode
│   └── styles.css              Dark theme
└── src-tauri/src/
    ├── lib.rs                  All #[tauri::command] handlers, app entry point
    ├── core/models.rs          Domain types (TimerConfig, AppState, …)
    ├── hotkeys/mod.rs          Hotkey listener + timer state machine (3 threads)
    ├── hotkeys/macos_tap.rs    macOS CGEventTap (replaces rdev on 14+)
    ├── hotkeys/windows_keyboard.rs  Windows GetAsyncKeyState polling
    ├── storage/mod.rs          Profile load/save, migration, icon discovery
    └── platform/               Platform detection (mod.rs, macos.rs, windows.rs, linux.rs)
```

## Frontend Module Dependencies

No cycles. Dependency order (each module may only import from those above it):

```
types
  ↑
i18n    ctx
  ↑      ↑
  utils ←┘
    ↑
  render  ←  (bindEvents injected at runtime — no static import)
    ↑
  actions
    ↑
  events
    ↑
bootstrap ← main
```

`render.ts` and `events.ts` are decoupled via dependency injection: `bootstrap.ts` calls `setBindEventsCallback(bindEvents)` once at startup so `render` can trigger event re-binding after every DOM replacement without a circular import.

## Hotkey Listener (3 Threads)

```
OS capture thread
  CGEventTap (macOS) │ GetAsyncKeyState poll (Windows) │ rdev (Linux X11)
         │
         ▼ raw key events (channel)
Matching thread
  compares pressed keys against registered hotkeys
  on match: starts/resets timer in shared state, sends tick message
         │
         ▼ tick messages (channel)
Tick thread
  fires `timer_tick` Tauri event every ~250 ms → frontend handleTimerTick()
```

If the OS capture thread fails to start (permissions, Wayland, etc.) it emits `hotkey_listener_error` → `bootstrap.ts` sets `ctx.listenerError` → red banner appears in UI.

## Overlay Windows

Each timer gets its own always-on-top transparent window (`overlay.ts`). Lifecycle:

1. `open_overlays` (Rust) — pre-creates all overlay windows hidden
2. `timer_tick` (Rust event) → `handleTimerTick` (TS) → `show_overlay` (Rust command) — shows window on first tick
3. `applyTimerState` (TS) — drives canvas draw loop (progress arc, blink, hotkey label)
4. Timer finishes → `hide_overlay` (Rust command)
5. Edit mode → Rust shows all overlays; drag via `data-tauri-drag-region`; positions saved on exit

On Windows, `apply_circular_clip` clips the Win32 window region to an ellipse so the transparent canvas circle aligns with the OS hit-test region.

## Platform Capability Model

`detect_platform()` in `platform/mod.rs` returns `PlatformInfo`:

```rust
PlatformInfo {
  kind: "macos" | "windows" | "linux-x11" | "linux-wayland",
  arch: "aarch64" | "x86_64" | …,
  overlay_capabilities: OverlayCapabilities { click_through, circular_clip },
  backend_adapter: BackendAdapterInfo { name, hotkey_backend },
}
```

The frontend reads `ctx.bootstrap.platform.kind` to vary behaviour (e.g. error banner footer, Wayland pre-check in `bootstrap.ts`).

## Data Flow Summary

**Frontend → Rust** (Tauri `invoke`): `get_bootstrap_payload`, `save_app_state`, `update_hotkey_registrations`, `open_overlays`, `show_overlay`, `hide_overlay`, `apply_circular_clip`, `set_overlays_edit_mode`, `set_watching`, …

**Rust → Frontend** (Tauri `emit`): `timer_tick`, `hotkey_listener_error`, `hotkey_action`, `overlay-positions-saved`, `config-updated`, `edit-mode-changed`, `hotkey-label-changed`

For the full command/event table and per-file line counts see `CLAUDE.md` at the project root.
