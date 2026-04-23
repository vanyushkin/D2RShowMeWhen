mod core;
mod hotkeys;
mod platform;
mod storage;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use core::models::{AppState, BootstrapPayload, OverlayTimerConfig, SaveResult};
use hotkeys::{ActiveTimers, GlobalHotkeys, HotkeyRegistration, Registrations, Watching};
use tauri::{Emitter, Manager};

// ── Overlay state ─────────────────────────────────────────────────────────────

/// Configs keyed by timer_index — populated when open_overlays is called.
type OverlayConfigs = Arc<Mutex<HashMap<usize, OverlayTimerConfig>>>;

/// Physical pixel widths reported by overlay JS (window.devicePixelRatio * winSize).
/// Stored when apply_circular_clip is called so show_overlay and
/// set_overlays_edit_mode can apply the Win32 clip BEFORE win.show().
type PhysicalWidths = Arc<Mutex<HashMap<usize, u32>>>;

struct MigrationRecord {
    migrated_from: Option<String>,
}

// ── Bootstrap ─────────────────────────────────────────────────────────────────

#[tauri::command]
fn get_bootstrap_payload(
    app: tauri::AppHandle,
    migration: tauri::State<Mutex<MigrationRecord>>,
) -> Result<BootstrapPayload, String> {
    let icons = storage::icons_from_resources(&app);
    let default_icon = icons.first().map(|icon| icon.file_name.as_str());
    let loaded = storage::load_state(&app, default_icon).map_err(|e| e.to_string())?;
    let platform = platform::detect_platform();

    if let Ok(mut record) = migration.lock() {
        record.migrated_from = loaded
            .migrated_from
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());
    }

    Ok(BootstrapPayload {
        state: loaded.state,
        icons,
        platform,
        storage_path: loaded.storage_path.to_string_lossy().to_string(),
        migrated_from: loaded
            .migrated_from
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
    })
}

// ── Save ──────────────────────────────────────────────────────────────────────

#[tauri::command]
fn save_app_state(app: tauri::AppHandle, payload: AppState) -> Result<SaveResult, String> {
    let icons = storage::icons_from_resources(&app);
    let default_icon = icons.first().map(|icon| icon.file_name.as_str());
    let normalized = payload.normalize(default_icon);
    let target = storage::save_state(&app, &normalized).map_err(|e| e.to_string())?;
    Ok(SaveResult {
        storage_path: target.to_string_lossy().to_string(),
    })
}

// ── Hotkey registration (called after bootstrap and after each save) ──────────

#[tauri::command]
fn update_hotkey_registrations(
    state_payload: AppState,
    registrations: tauri::State<Registrations>,
) -> Result<usize, String> {
    let profile = state_payload
        .profiles
        .get(&state_payload.active_profile)
        .ok_or_else(|| "Active profile not found".to_string())?;

    let new_regs: Vec<HotkeyRegistration> = profile
        .timers
        .iter()
        .enumerate()
        .filter(|(_, timer)| timer.enabled)
        .flat_map(|(i, timer)| {
            let dur   = timer.duration as u64;
            let blink = timer.blink_threshold as u64;
            let mut regs = Vec::new();
            if !timer.hotkey.is_empty() {
                if let Some(parsed) = hotkeys::parse_hotkey_string(&timer.hotkey) {
                    regs.push(HotkeyRegistration {
                        timer_index: i,
                        hotkey: parsed,
                        duration_secs: dur,
                        blink_threshold_secs: blink,
                    });
                }
            }
            if !timer.hotkey2.is_empty() {
                if let Some(parsed) = hotkeys::parse_hotkey_string(&timer.hotkey2) {
                    regs.push(HotkeyRegistration {
                        timer_index: i,
                        hotkey: parsed,
                        duration_secs: dur,
                        blink_threshold_secs: blink,
                    });
                }
            }
            regs
        })
        .collect();

    let count = new_regs.len();
    *registrations.lock().map_err(|e| e.to_string())? = new_regs;
    log::info!("Hotkey registrations updated: {} active", count);
    Ok(count)
}

// ── Global hotkey config update ───────────────────────────────────────────────

/// Called from JS after save or on bootstrap to sync global action hotkeys
/// (hide/show overlays, layout edit toggle) with the Rust hotkey thread.
#[tauri::command]
fn update_global_hotkeys(
    state_payload: AppState,
    global_hotkeys: tauri::State<GlobalHotkeys>,
) -> Result<(), String> {
    let mut cfg = global_hotkeys.lock().map_err(|e| e.to_string())?;
    cfg.hide_show  = hotkeys::parse_hotkey_string(&state_payload.hide_show_hotkey);
    cfg.hide_show2 = hotkeys::parse_hotkey_string(&state_payload.hide_show_hotkey2);
    // Auto-enable: the hotkey is active whenever any key string (primary or alt) is configured.
    // The JS toggle now controls live label/edit-mode state, not the enabled flag.
    cfg.hide_show_enabled = !state_payload.hide_show_hotkey.trim().is_empty()
        || !state_payload.hide_show_hotkey2.trim().is_empty();
    cfg.layout_edit  = hotkeys::parse_hotkey_string(&state_payload.layout_edit_hotkey);
    cfg.layout_edit2 = hotkeys::parse_hotkey_string(&state_payload.layout_edit_hotkey2);
    cfg.layout_edit_enabled = !state_payload.layout_edit_hotkey.trim().is_empty()
        || !state_payload.layout_edit_hotkey2.trim().is_empty();
    log::info!(
        "Global hotkeys updated — hide_show: {:?} ({}), layout_edit: {:?} ({})",
        state_payload.hide_show_hotkey,
        cfg.hide_show_enabled,
        state_payload.layout_edit_hotkey,
        cfg.layout_edit_enabled,
    );
    Ok(())
}

// ── Manual timer trigger (from UI "test" button) ──────────────────────────────

#[tauri::command]
fn trigger_timer(
    timer_index: usize,
    duration_secs: u64,
    blink_threshold_secs: u64,
    active_timers: tauri::State<ActiveTimers>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use hotkeys::{ActiveTimer, TimerTickPayload};
    use std::time::Instant;
    use tauri::Emitter;

    let mut timers = active_timers.lock().map_err(|e| e.to_string())?;
    timers.insert(
        timer_index,
        ActiveTimer {
            start: Instant::now(),
            duration_secs,
            blink_threshold_secs,
            last_emitted_remaining: duration_secs + 1,
        },
    );
    drop(timers);

    app.emit(
        "timer_tick",
        TimerTickPayload {
            timer_index,
            remaining_secs: duration_secs,
            total_secs: duration_secs,
            blinking: false,
            finished: false,
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// ── Overlay management ────────────────────────────────────────────────────────

const BASE_OVERLAY_SIZE: u32 = 48;

fn compute_win_size(timer_size: u16) -> u32 {
    let size_px = std::cmp::max(28, (BASE_OVERLAY_SIZE * timer_size as u32) / 100);
    size_px + 16
}

fn set_overlay_clickthrough(win: &tauri::WebviewWindow, ignore: bool) {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = win.set_ignore_cursor_events(ignore);
    }

    #[cfg(target_os = "windows")]
    {
        let _ = (win, ignore);
    }
}

// ── Overlay circular appearance ───────────────────────────────────────────────
//
// Two-layer approach:
//   1. CSS `clip-path: circle(calc(50% - 8px) at center)` in overlay.html
//      clips the WebView2-rendered content to a circle.  DPI-agnostic.
//   2. Win32 SetWindowRgn clips the Win32 window itself, removing the white
//      WebView2 background visible outside the CSS clip area.
//
// SetWindowRgn MUST be called AFTER win.show() so the window has received
// WM_DPICHANGED and inner_size() reflects the true physical dimensions.
// Callers use a spawned thread with a 200 ms delay for this reason.

#[cfg(target_os = "windows")]
extern "system" {
    fn CreateEllipticRgn(nLeftRect: i32, nTopRect: i32, nRightRect: i32, nBottomRect: i32) -> isize;
    fn SetWindowRgn(hWnd: isize, hRgn: isize, bRedraw: i32) -> i32;
    fn GetWindowRect(hWnd: isize, lpRect: *mut WinRect) -> i32;
    fn ClientToScreen(hWnd: isize, lpPoint: *mut WinPoint) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: isize,
        dw_attribute: u32,
        pv_attribute: *const std::ffi::c_void,
        cb_attribute: u32,
    ) -> i32;
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct WinRect { left: i32, top: i32, right: i32, bottom: i32 }

#[cfg(target_os = "windows")]
#[repr(C)]
struct WinPoint { x: i32, y: i32 }

/// Apply a Win32 circular clip region to an overlay HWND using a known physical
/// pixel width.
///
/// The clip circle matches the CSS `clip-path: circle(calc(50% - 8px) at center)`
/// and the canvas draw circle (both have 8 logical-px / `pad_phys` physical-px
/// inset from each edge).  Without this inset the Win32 region is 8px larger
/// than the CSS circle on every side, leaving a ring that is inside the Win32
/// clip but outside the CSS clip.  When DWM first composites the window
/// (on uncloak or edit-mode reveal) it shows one frame with the window class
/// background brush (white) inside the Win32 clip — the ring flashes white,
/// most visibly at the top where DWM starts compositing.  Matching the circles
/// eliminates the ring entirely.
///
/// Safe to call on a hidden window — the HWND geometry is valid as soon as
/// the window is created.
#[cfg(target_os = "windows")]
fn apply_win32_circular_clip(win: &tauri::WebviewWindow, phys: i32) {
    if phys <= 0 { return; }
    match win.hwnd() {
        Ok(hwnd) => {
            let hwnd_raw = hwnd.0 as isize;
            // Measure client-area origin within the HWND rectangle.
            // For transparent windows with DWM invisible borders this offset
            // is typically (8, 0) or (8, 8) on Windows 10/11.
            let (off_x, off_y) = unsafe {
                let mut win_rect = WinRect { left: 0, top: 0, right: 0, bottom: 0 };
                let mut client_pt = WinPoint { x: 0, y: 0 };
                GetWindowRect(hwnd_raw, &mut win_rect);
                ClientToScreen(hwnd_raw, &mut client_pt);
                (client_pt.x - win_rect.left, client_pt.y - win_rect.top)
            };
            // Inset the ellipse bounds by the canvas pad (8 logical px) so the
            // Win32 clip circle matches the CSS / canvas circle exactly.
            // This removes the ~8-px ring that would otherwise flash white when
            // DWM first composites the window after uncloaking.
            let scale    = win.scale_factor().unwrap_or(1.0);
            let pad_phys = (8.0 * scale).round() as i32;
            // Guard: if phys is too small for the inset, fall back to 1-px circle.
            let diameter = (phys - 2 * pad_phys).max(1);
            let x0 = off_x + pad_phys;
            let y0 = off_y + pad_phys;
            let x1 = x0 + diameter + 1; // +1 for GDI exclusive convention
            let y1 = y0 + diameter + 1;
            log::debug!(
                "apply_win32_circular_clip: phys={} scale={:.2} pad_phys={} offset=({},{}) rgn=({},{},{},{})",
                phys, scale, pad_phys, off_x, off_y, x0, y0, x1, y1
            );
            unsafe {
                let rgn = CreateEllipticRgn(x0, y0, x1, y1);
                if rgn != 0 {
                    SetWindowRgn(hwnd_raw, rgn, 1);
                } else {
                    log::warn!("apply_win32_circular_clip: CreateEllipticRgn returned NULL");
                }
            }
        }
        Err(_) => log::warn!("apply_win32_circular_clip: could not get hwnd"),
    }
}

#[cfg(not(target_os = "windows"))]
fn apply_win32_circular_clip(_win: &tauri::WebviewWindow, _phys: i32) {}

/// Cloak or uncloak an overlay window using DWMWA_CLOAK (attribute 13).
///
/// A cloaked window retains WS_VISIBLE so WebView2 renders continuously
/// into its surface — but DWM does not composite it to the screen.
/// This eliminates the white flash that appears when a previously-hidden
/// (SW_HIDE) window is shown: WebView2 suspends rendering for hidden windows,
/// so the first visible frame after SW_SHOW briefly shows a blank/white
/// background before the canvas paint is composited.
///
/// Workflow:
///   create window (visible=false)
///   → set_window_cloak(win, true)   — cloak first
///   → win.show()                    — WS_VISIBLE; WebView2 renders, DWM hides
///   → (timer fires, canvas is ready)
///   → apply_win32_circular_clip(win, phys)
///   → set_window_cloak(win, false)  — instant reveal, no white flash
#[cfg(target_os = "windows")]
fn set_window_cloak(win: &tauri::WebviewWindow, cloak: bool) {
    if let Ok(hwnd) = win.hwnd() {
        let cloaked: i32 = if cloak { 1 } else { 0 };
        unsafe {
            DwmSetWindowAttribute(
                hwnd.0 as isize,
                13, // DWMWA_CLOAK
                &cloaked as *const i32 as *const std::ffi::c_void,
                std::mem::size_of::<i32>() as u32,
            );
        }
    }
    // win.show() is intentionally NOT called here.
    // Overlay windows are kept WS_VISIBLE at all times (set once during
    // creation).  Calling win.show() on an already-visible window triggers
    // WebView2's put_IsVisible(TRUE) even when it was already TRUE, which
    // forces a repaint cycle and causes the white flash artifact.
    // User-visible state is controlled exclusively via DWM cloaking.
}

/// On non-Windows platforms cloaking is not available; fall back to
/// the standard show/hide mechanism.
#[cfg(not(target_os = "windows"))]
fn set_window_cloak(win: &tauri::WebviewWindow, cloak: bool) {
    if cloak {
        let _ = win.hide();
    } else {
        let _ = win.show();
    }
}

fn apply_overlay_window_config(
    win: &tauri::WebviewWindow,
    pos: [i32; 2],
    win_size: u32,
) {
    use tauri::LogicalSize;

    let _ = win.set_size(LogicalSize::new(win_size as f64, win_size as f64));
    let _ = win.set_position(tauri::LogicalPosition::new(pos[0] as f64, pos[1] as f64));
    let _ = win.set_always_on_top(true);
    // NOTE: SetWindowRgn is NOT applied here.  The overlay JS calls
    // apply_circular_clip (via invoke) with window.innerWidth * devicePixelRatio,
    // which is always the correct physical size regardless of Win32 DPI APIs.
}

/// Create a single hidden overlay window.
///
/// On Windows, WebviewWindowBuilder::build() must be called from the main
/// thread (it dispatches to the Win32 message loop internally via
/// run_on_main_thread). Calling it from a Tauri synchronous command handler
/// deadlocks because those handlers run in the WebView2 IPC callback, which
/// itself occupies the main thread. This function is safe to call from:
///   - setup()          — runs on the main thread before the event loop
///   - async commands   — run on a tokio thread; run_on_main_thread works
/// It is NOT safe from synchronous command handlers on Windows.
fn create_single_overlay_window(app: &tauri::AppHandle, index: usize, win_size: u32, pos: [i32; 2]) {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let label = format!("overlay-{}", index);
    if app.get_webview_window(&label).is_some() {
        return; // already exists
    }

    match WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App(format!("overlay.html?index={}", index).into()),
    )
    .title("")
    .inner_size(win_size as f64, win_size as f64)
    .position(pos[0] as f64, pos[1] as f64)
    .decorations(false)
    .resizable(false)
    .transparent(true)
    .background_color(tauri::webview::Color(0, 0, 0, 0))
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()
    {
        Ok(win) => {
            log::info!("Created overlay window overlay-{}", index);
            // Cloak before showing: DWM hides the window while WebView2
            // renders into its surface continuously (WS_VISIBLE is set).
            // The correct circular clip is applied by show_overlay /
            // set_overlays_edit_mode just before uncloaking.
            set_window_cloak(&win, true);
            let _ = win.show(); // WS_VISIBLE → WebView2 starts rendering
        }
        Err(e) => log::error!("Failed to create overlay window overlay-{}: {}", index, e),
    }
}

fn ensure_overlay_window(
    app: &tauri::AppHandle,
    index: usize,
    overlay_configs: &OverlayConfigs,
) -> Result<tauri::WebviewWindow, String> {
    let cfg = {
        let configs = overlay_configs.lock().map_err(|e| e.to_string())?;
        configs
            .get(&index)
            .cloned()
            .ok_or_else(|| format!("No overlay config for index {}", index))?
    };

    let label = format!("overlay-{}", index);
    if let Some(win) = app.get_webview_window(&label) {
        // Window already exists and was configured by open_overlays.
        // Do NOT call apply_overlay_window_config here — set_size on a
        // visible window triggers a WebView2 buffer recreation (white flash).
        return Ok(win);
    }

    // Window not found. This can happen if setup() pre-creation was skipped
    // (e.g. first run, no saved config) and the timer was added at runtime.
    // We attempt creation here; this is only safe from async command contexts.
    log::warn!("overlay-{} not pre-created; attempting build() now", index);
    let win = tauri::WebviewWindowBuilder::new(
        app,
        &label,
        tauri::WebviewUrl::App(format!("overlay.html?index={}", index).into()),
    )
    .title("")
    .inner_size(cfg.win_size as f64, cfg.win_size as f64)
    .position(cfg.position[0] as f64, cfg.position[1] as f64)
    .decorations(false)
    .resizable(false)
    .transparent(true)
    .background_color(tauri::webview::Color(0, 0, 0, 0))
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()
    .map_err(|e: tauri::Error| e.to_string())?;

    apply_overlay_window_config(&win, cfg.position, cfg.win_size);
    set_overlay_clickthrough(&win, false);
    // Cloak + show so WebView2 renders immediately but window stays invisible.
    set_window_cloak(&win, true);
    let _ = win.show();
    Ok(win)
}

/// Hide all overlay windows without destroying them.
///
/// We use `hide()` instead of `close()` throughout so that the Tauri window label
/// stays registered. `close()` is async — the label remains occupied until the
/// event loop processes the close, so an immediate `open_overlays` call would fail
/// with "webview with label X already exists". With `hide()`/`show()` the window
/// is created once per session and simply shown or hidden.
fn hide_all_overlays(app: &tauri::AppHandle) {
    use core::models::MAX_TIMERS;
    for i in 0..MAX_TIMERS {
        let label = format!("overlay-{}", i);
        if let Some(win) = app.get_webview_window(&label) {
            set_overlay_clickthrough(&win, false);
            set_window_cloak(&win, true);
        }
    }
}

/// open_overlays is async so it runs on a tokio thread.
///
/// On Windows, WebviewWindowBuilder::build() dispatches to the main event
/// loop via run_on_main_thread. From a tokio thread this works correctly.
/// From a synchronous command handler (which runs in the WebView2 IPC
/// callback on the main thread) it deadlocks. Making this command async
/// moves execution off the main thread and avoids the deadlock.
#[tauri::command]
async fn open_overlays(
    app: tauri::AppHandle,
    state_payload: AppState,
) -> Result<usize, String> {
    use core::models::MAX_TIMERS;

    // Get overlay_configs from managed state (avoids lifetime issues with
    // State<'_, T> across async boundaries).
    let overlay_configs: OverlayConfigs = app.state::<OverlayConfigs>().inner().clone();

    let profile = state_payload
        .profiles
        .get(&state_payload.active_profile)
        .ok_or_else(|| "Active profile not found".to_string())?;

    // ── Phase 1: build configs, release the mutex BEFORE touching windows.
    // Prevents deadlock: overlay JS calls get_overlay_config (needs lock) while
    // open_overlays is still holding it during window creation.
    struct PendingOverlay {
        index: usize,
        pos: [i32; 2],
        win_size: u32,
        cfg: OverlayTimerConfig,
    }

    let mut pending: Vec<PendingOverlay> = Vec::new();
    let mut enabled_indices = std::collections::HashSet::<usize>::new();

    // ── Collect enabled timers with their saved (or provisional) positions.
    // saved_x is used only for sorting — final x is reassigned sequentially below.
    struct RawPending {
        index: usize,
        saved_x: i32, // for sort order; replaced with sequential x after sort
        saved_y: i32,
        win_size: u32,
        icon_path: String,
        timer_ref_idx: usize, // same as index, kept for clarity
    }

    let mut raw: Vec<RawPending> = Vec::new();
    let mut seq = 0usize;
    for (i, timer) in profile.timers.iter().enumerate() {
        if !timer.enabled {
            continue;
        }
        let win_size = compute_win_size(timer.size);
        // Use saved position if available; otherwise a provisional sequential default.
        let (sx, sy) = profile
            .positions
            .get(&i.to_string())
            .map(|p| (p[0], p[1]))
            .unwrap_or((100 + seq as i32 * (win_size as i32 + 8), 100));
        raw.push(RawPending {
            index: i,
            saved_x: sx,
            saved_y: sy,
            win_size,
            icon_path: storage::icon_file_path(&app, &timer.icon),
            timer_ref_idx: i,
        });
        seq += 1;
    }

    // Sort by saved x so the Edit Layout left-to-right order is preserved.
    raw.sort_by_key(|r| r.saved_x);

    for r in raw {
        let timer = &profile.timers[r.timer_ref_idx];
        let win_size = r.win_size;
        enabled_indices.insert(r.index);
        pending.push(PendingOverlay {
            index: r.index,
            pos: [r.saved_x, r.saved_y],
            win_size,
            cfg: OverlayTimerConfig {
                timer_index: r.index,
                icon_path: r.icon_path,
                position: [r.saved_x, r.saved_y],
                duration: timer.duration,
                blink_threshold: timer.blink_threshold,
                blink_color: timer.blink_color.clone(),
                blink: timer.blink,
                size: timer.size,
                opacity: timer.opacity,
                hotkey: timer.hotkey.clone(),
                hotkey2: timer.hotkey2.clone(),
                show_hotkey_labels: profile.show_hotkey_labels,
                win_size,
            },
        });
    }

    {
        let mut configs = overlay_configs.lock().map_err(|e| e.to_string())?;
        configs.clear();
        for p in &pending {
            configs.insert(p.index, p.cfg.clone());
        }
    } // lock released here

    // ── Phase 2: cloak windows for now-disabled timers.
    // Use set_window_cloak (not win.hide()) to keep all overlay windows
    // WS_VISIBLE so WebView2 renders continuously.  win.hide() would make
    // them WS_INVISIBLE, violating the invariant that show/hide is done
    // exclusively via DWM cloaking.
    for i in 0..MAX_TIMERS {
        if !enabled_indices.contains(&i) {
            let label = format!("overlay-{}", i);
            if let Some(win) = app.get_webview_window(&label) {
                set_overlay_clickthrough(&win, false);
                set_window_cloak(&win, true);
            }
        }
    }

    // ── Phase 3: refresh existing windows; create missing ones.
    // Windows are normally pre-created in setup() (main thread). If a timer
    // was added at runtime, its window won't exist yet. We create it here.
    // This is safe because open_overlays is async → runs on a tokio thread,
    // where build() → run_on_main_thread() works without deadlock.
    let mut count = 0usize;
    for p in &pending {
        let label = format!("overlay-{}", p.index);

        if let Some(win) = app.get_webview_window(&label) {
            // Cloak BEFORE resizing so any currently-visible overlay is
            // hidden by DWM before set_size triggers a WebView2 buffer
            // recreation — the root cause of the white strip on Save.
            set_window_cloak(&win, true);
            apply_overlay_window_config(&win, p.pos, p.win_size);
            set_overlay_clickthrough(&win, false);
            let _ = win.emit("config-updated", ());
            count += 1;
        } else {
            // Not pre-created — create now (async context, safe on Windows).
            create_single_overlay_window(&app, p.index, p.win_size, p.pos);
            count += 1;
        }
    }

    log::info!(
        "Overlay system armed for {} timers ({} windows refreshed/created)",
        enabled_indices.len(),
        count
    );
    Ok(enabled_indices.len())
}

#[tauri::command]
fn close_overlays(app: tauri::AppHandle) -> Result<(), String> {
    hide_all_overlays(&app);
    Ok(())
}

/// Called by overlay.ts when its timer starts — reveal this overlay window.
/// Apply the circular Win32 clip then uncloak: because the window retained
/// WS_VISIBLE (cloaked) since creation, WebView2 has a rendered frame ready
/// and there is no white flash on reveal.
#[tauri::command]
fn show_overlay(index: usize, app: tauri::AppHandle, phys_widths: tauri::State<PhysicalWidths>) {
    let overlays: OverlayConfigs = app.state::<OverlayConfigs>().inner().clone();
    if let Ok(win) = ensure_overlay_window(&app, index, &overlays) {
        let phys = phys_widths.lock().ok()
            .and_then(|m| m.get(&index).copied())
            .map(|w| w as i32)
            .or_else(|| win.inner_size().ok().map(|s| s.width as i32))
            .unwrap_or(64);
        apply_win32_circular_clip(&win, phys);
        set_window_cloak(&win, false); // uncloak (also calls win.show() if needed)
        let _ = win.set_always_on_top(true);
        set_overlay_clickthrough(&win, true);
    }
}

/// Called by overlay.ts when its timer finishes — cloak this overlay window.
#[tauri::command]
fn hide_overlay(index: usize, app: tauri::AppHandle) {
    let label = format!("overlay-{}", index);
    if let Some(win) = app.get_webview_window(&label) {
        set_overlay_clickthrough(&win, false);
        set_window_cloak(&win, true);
    }
}

/// Updates the show_hotkey_labels flag in all overlay configs and broadcasts
/// the change to every overlay window. Called from JS when the user toggles
/// the "Key labels" checkbox or the hide_show global hotkey fires.
/// Using Rust-side app.emit() ensures all webview windows (including those
/// whose JS may have initialised after the JS-side emit was called) receive it.
#[tauri::command]
fn update_overlay_hotkey_labels(
    show: bool,
    overlay_configs: tauri::State<OverlayConfigs>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    {
        let mut configs = overlay_configs.lock().map_err(|e| e.to_string())?;
        for cfg in configs.values_mut() {
            cfg.show_hotkey_labels = show;
        }
    }
    let _ = app.emit("hotkey-label-changed", show);
    Ok(())
}

#[tauri::command]
fn set_watching(
    enabled: bool,
    watching: tauri::State<Watching>,
) {
    watching.store(enabled, std::sync::atomic::Ordering::Relaxed);
    log::debug!("Watching: {}", enabled);
}

#[tauri::command]
fn set_overlays_edit_mode(
    app: tauri::AppHandle,
    enabled: bool,
    overlay_configs: tauri::State<OverlayConfigs>,
    active_timers: tauri::State<ActiveTimers>,
    phys_widths: tauri::State<PhysicalWidths>,
) -> Result<(), String> {
    let index_sizes: Vec<(usize, u32)> = {
        let configs = overlay_configs.lock().map_err(|e| e.to_string())?;
        configs.iter().map(|(&k, v)| (k, v.win_size)).collect()
    };
    let stored_phys: HashMap<usize, u32> = phys_widths.lock()
        .map(|m| m.clone())
        .unwrap_or_default();
    let running_indices: std::collections::HashSet<usize> = {
        let timers = active_timers.lock().map_err(|e| e.to_string())?;
        timers.keys().copied().collect()
    };

    for (index, win_size) in &index_sizes {
        let label = format!("overlay-{}", index);
        let win = if enabled {
            ensure_overlay_window(&app, *index, overlay_configs.inner())?
        } else if let Some(win) = app.get_webview_window(&label) {
            win
        } else {
            continue;
        };

        if enabled {
            set_overlay_clickthrough(&win, false);
            let phys = stored_phys.get(index).copied()
                .map(|w| w as i32)
                .or_else(|| win.inner_size().ok().map(|s| s.width as i32))
                .unwrap_or(*win_size as i32);
            apply_win32_circular_clip(&win, phys);
            set_window_cloak(&win, false); // uncloak → instant reveal, no flash
            let _ = win.set_always_on_top(true);
        } else if running_indices.contains(index) {
            set_window_cloak(&win, false); // keep visible for running timer
            let _ = win.set_always_on_top(true);
            set_overlay_clickthrough(&win, true);
        } else {
            // Cloak overlays whose timer is not currently running.
            set_overlay_clickthrough(&win, false);
            set_window_cloak(&win, true);
        }
        let _ = win.emit("edit-mode-changed", enabled);
    }

    // When exiting edit mode, read all window positions from the OS and
    // send them to the main window so it can persist them.
    // outer_position() returns physical pixels; divide by scale_factor to get
    // logical pixels, which is what WebviewWindowBuilder::position() expects.
    if !enabled {
        let mut positions: HashMap<usize, [i32; 2]> = HashMap::new();
        for (index, _) in &index_sizes {
            let label = format!("overlay-{}", index);
            if let Some(win) = app.get_webview_window(&label) {
                if let Ok(pos) = win.outer_position() {
                    let scale = win.scale_factor().unwrap_or(1.0);
                    positions.insert(*index, [
                        (pos.x as f64 / scale).round() as i32,
                        (pos.y as f64 / scale).round() as i32,
                    ]);
                }
            }
        }
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.emit("overlay-positions-saved", &positions);
        }
    }
    Ok(())
}

/// Store the JS-reported physical width and apply the Win32 circular clip.
///
/// Called by overlay.ts after it knows window.devicePixelRatio (always correct
/// in WebView2).  Storing physW here lets show_overlay and set_overlays_edit_mode
/// apply the clip BEFORE win.show() so there is no visible flash.
#[tauri::command]
fn apply_circular_clip(
    index: usize,
    physical_width: u32,
    phys_widths: tauri::State<PhysicalWidths>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if let Ok(mut m) = phys_widths.lock() {
        m.insert(index, physical_width);
    }
    if let Some(win) = app.get_webview_window(&format!("overlay-{}", index)) {
        apply_win32_circular_clip(&win, physical_width as i32);
    }
    log::debug!("apply_circular_clip[{}]: physical_width={}", index, physical_width);
    Ok(())
}

#[tauri::command]
fn get_overlay_config(
    index: usize,
    overlay_configs: tauri::State<OverlayConfigs>,
) -> Result<OverlayTimerConfig, String> {
    let configs = overlay_configs.lock().map_err(|e| e.to_string())?;
    configs
        .get(&index)
        .cloned()
        .ok_or_else(|| format!("No overlay config for index {}", index))
}

/// Polling endpoint for overlay windows.
/// Returns the current countdown state for a timer if it's running, or None.
#[tauri::command]
fn get_active_timer_state(
    timer_index: usize,
    active_timers: tauri::State<ActiveTimers>,
) -> Option<hotkeys::TimerTickPayload> {
    use std::time::Instant;
    let timers = active_timers.lock().ok()?;
    let timer = timers.get(&timer_index)?;
    let elapsed = Instant::now().duration_since(timer.start).as_secs_f64();
    let remaining_f = (timer.duration_secs as f64 - elapsed).max(0.0);
    let remaining = remaining_f.ceil() as u64;
    Some(hotkeys::TimerTickPayload {
        timer_index,
        remaining_secs: remaining,
        total_secs: timer.duration_secs,
        blinking: remaining <= timer.blink_threshold_secs,
        finished: remaining == 0,
    })
}


// ── Stop all active timers (on profile switch) ────────────────────────────────

/// Clears all running timers and hides their overlay windows.
/// Called when the user switches profiles so timers from the old profile
/// don't bleed into the new one.
#[tauri::command]
fn stop_all_timers(
    app: tauri::AppHandle,
    active_timers: tauri::State<ActiveTimers>,
) -> Result<(), String> {
    let mut timers = active_timers.lock().map_err(|e| e.to_string())?;
    let indices: Vec<usize> = timers.keys().copied().collect();
    timers.clear();
    drop(timers);
    for index in indices {
        let label = format!("overlay-{}", index);
        if let Some(win) = app.get_webview_window(&label) {
            set_window_cloak(&win, true);
        }
    }
    Ok(())
}

// ── macOS privacy settings helper ────────────────────────────────────────────

/// Resets the Input Monitoring TCC permission for this app using `tccutil`.
#[tauri::command]
fn reset_input_monitoring_permission() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("tccutil")
            .args(["reset", "ListenEvent", "com.vanyushkin.d2rshowmewhen"])
            .output();
        hotkeys::request_input_monitoring_access();
    }
    Ok(())
}

/// Opens System Settings → Privacy & Security → Input Monitoring on macOS.
#[tauri::command]
fn open_privacy_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
            .spawn()
            .map_err(|e| format!("Failed to open System Settings: {}", e))?;
    }
    Ok(())
}

// ── App entry point ───────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let registrations: Registrations = Default::default();
    let active_timers: ActiveTimers = Default::default();
    let overlay_configs: OverlayConfigs = Default::default();
    let phys_widths: PhysicalWidths = Default::default();
    let watching: Watching = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let global_hotkeys: GlobalHotkeys = Default::default();

    // Clone overlay_configs Arc so setup() can pre-populate it while the
    // original is moved into Tauri's managed state.
    let setup_overlay_configs = overlay_configs.clone();

    tauri::Builder::default()
        .manage(Mutex::new(MigrationRecord { migrated_from: None }))
        .manage(registrations.clone())
        .manage(active_timers.clone())
        .manage(overlay_configs)
        .manage(phys_widths)
        .manage(watching.clone())
        .manage(global_hotkeys.clone())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::Stdout,
                    ),
                    tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::LogDir {
                            file_name: Some("d2rshowmewhen".to_string()),
                        },
                    ),
                ])
                .build(),
        )
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    // Close all overlay windows first so their WebView2
                    // instances can begin graceful shutdown.
                    use core::models::MAX_TIMERS;
                    for i in 0..MAX_TIMERS {
                        if let Some(win) = window.app_handle()
                            .get_webview_window(&format!("overlay-{}", i))
                        {
                            let _ = win.close();
                        }
                    }

                    // On macOS the app must call exit(0) explicitly — without
                    // it the process stays alive (Dock icon remains).
                    // On Windows we let Tauri's normal window lifecycle handle
                    // shutdown. Calling exit(0) immediately races with WebView2
                    // cleanup and leaves msedgewebview2.exe lingering, which
                    // blocks the next launch. Instead we spawn a short-lived
                    // thread that exits after WebView2 has had time to release
                    // its user-data directory lock (~400 ms is enough in practice).
                    #[cfg(target_os = "macos")]
                    window.app_handle().exit(0);

                    #[cfg(target_os = "windows")]
                    {
                        let handle = window.app_handle().clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(400));
                            handle.exit(0);
                        });
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_bootstrap_payload,
            save_app_state,
            update_hotkey_registrations,
            update_global_hotkeys,
            trigger_timer,
            open_overlays,
            close_overlays,
            show_overlay,
            hide_overlay,
            set_overlays_edit_mode,
            apply_circular_clip,
            get_overlay_config,
            get_active_timer_state,
            set_watching,
            update_overlay_hotkey_labels,
            stop_all_timers,
            open_privacy_settings,
            reset_input_monitoring_permission,
        ])
        .setup(move |app| {
            if cfg!(debug_assertions) {
                log::info!(
                    "D2RShowMeWhen booted: {:?}",
                    app.handle().package_info().name
                );
                // Auto-open DevTools in debug builds so console errors are
                // immediately visible without right-click → Inspect.
                if let Some(main_window) = app.get_webview_window("main") {
                    main_window.open_devtools();
                }
            }

            // On Windows, remove the native OS title bar so the app uses its
            // custom HTML titlebar (matching the macOS titleBarStyle:"Overlay"
            // approach).  The custom titlebar provides drag, minimize, and close
            // via data-tauri-drag-region / win-minimize / win-close buttons.
            #[cfg(target_os = "windows")]
            if let Some(main_win) = app.get_webview_window("main") {
                let _ = main_win.set_decorations(false);
            }

            // ── Pre-create overlay windows on the main thread ─────────────────
            // WebviewWindowBuilder::build() internally dispatches to the main
            // event loop (run_on_main_thread). On Windows, Tauri IPC callbacks
            // also run on the main thread, so calling build() from any command
            // handler (sync or async) would deadlock.
            //
            // setup() runs on the main thread before the event loop starts
            // accepting IPC, so build() is safe here. We load the saved state
            // and pre-create a hidden overlay window for every enabled timer in
            // the active profile. ensure_overlay_window() then finds them via
            // get_webview_window() and never needs to call build() at runtime.
            {
                let icons = storage::icons_from_resources(app.handle());
                let default_icon = icons.first().map(|i| i.file_name.as_str());
                // load_state always returns Ok (returns default state on first run)
                if let Ok(loaded) = storage::load_state(app.handle(), default_icon) {
                    let state = &loaded.state;
                    if let Some(profile) = state.profiles.get(&state.active_profile) {
                        // Collect timer data before acquiring the configs lock.
                        let timer_data: Vec<(usize, OverlayTimerConfig)> = profile
                            .timers
                            .iter()
                            .enumerate()
                            .filter(|(_, t)| t.enabled)
                            .map(|(i, t)| {
                                let win_size = compute_win_size(t.size);
                                let pos = profile
                                    .positions
                                    .get(&i.to_string())
                                    .map(|p| [p[0], p[1]])
                                    .unwrap_or([
                                        100 + i as i32 * (win_size as i32 + 8),
                                        100,
                                    ]);
                                let icon_path =
                                    storage::icon_file_path(app.handle(), &t.icon);
                                (
                                    i,
                                    OverlayTimerConfig {
                                        timer_index: i,
                                        icon_path,
                                        position: pos,
                                        duration: t.duration,
                                        blink_threshold: t.blink_threshold,
                                        blink_color: t.blink_color.clone(),
                                        blink: t.blink,
                                        size: t.size,
                                        opacity: t.opacity,
                                        hotkey: t.hotkey.clone(),
                                        hotkey2: t.hotkey2.clone(),
                                        show_hotkey_labels: profile.show_hotkey_labels,
                                        win_size,
                                    },
                                )
                            })
                            .collect();

                        // Populate overlay_configs BEFORE creating windows so
                        // that overlay.ts init() can call get_overlay_config
                        // immediately after the window loads.
                        if let Ok(mut configs) = setup_overlay_configs.lock() {
                            for (i, cfg) in &timer_data {
                                configs.insert(*i, cfg.clone());
                            }
                        }

                        // Create hidden overlay windows (safe: main thread).
                        for (i, cfg) in &timer_data {
                            create_single_overlay_window(
                                app.handle(),
                                *i,
                                cfg.win_size,
                                cfg.position,
                            );
                        }
                        log::info!(
                            "Pre-created {} overlay window(s) for profile {:?}",
                            timer_data.len(),
                            state.active_profile
                        );
                    }
                }
            }

            hotkeys::start(app.handle().clone(), registrations, active_timers, watching, global_hotkeys);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
