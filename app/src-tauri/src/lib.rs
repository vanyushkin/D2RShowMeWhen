mod core;
mod hotkeys;
mod platform;
mod storage;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use core::models::{AppState, BootstrapPayload, OverlayTimerConfig, SaveResult};
use hotkeys::{ActiveTimers, GlobalHotkeys, HotkeyRegistration, Registrations, Watching};
use tauri::Manager;

// ── Overlay state ─────────────────────────────────────────────────────────────

/// Configs keyed by timer_index — populated when open_overlays is called.
type OverlayConfigs = Arc<Mutex<HashMap<usize, OverlayTimerConfig>>>;

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
            let _ = win.set_ignore_cursor_events(false);
            let _ = win.hide();
        }
    }
}

#[tauri::command]
fn open_overlays(
    app: tauri::AppHandle,
    state_payload: AppState,
    overlay_configs: tauri::State<OverlayConfigs>,
) -> Result<usize, String> {
    use core::models::MAX_TIMERS;
    use tauri::{LogicalSize, WebviewUrl, WebviewWindowBuilder};

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

    // ── Phase 2: hide windows for now-disabled timers.
    for i in 0..MAX_TIMERS {
        if !enabled_indices.contains(&i) {
            let label = format!("overlay-{}", i);
            if let Some(win) = app.get_webview_window(&label) {
                let _ = win.hide();
            }
        }
    }

    // ── Phase 3: show or create windows for enabled timers.
    //
    // If the window already exists (was hidden by close_overlays or a previous
    // open_overlays call), we resize/reposition it, ensure always-on-top is set,
    // show it, then reload the webview so init() runs fresh against the new config.
    // Reload is used instead of config-updated because a previous init() failure
    // leaves no listeners — only a full reload recovers the window reliably.
    //
    // If it doesn't exist yet, we create it fresh.
    let mut count = 0usize;
    for p in pending {
        let label = format!("overlay-{}", p.index);

        if let Some(win) = app.get_webview_window(&label) {
            // Existing window — resize, reposition, restore settings, then
            // reload the webview so init() runs fresh with the latest config.
            // The window stays hidden; overlay.ts will show it when a timer fires
            // or when edit mode is enabled.
            let _ = win.set_size(LogicalSize::new(p.win_size as f64, p.win_size as f64));
            let _ = win.set_position(tauri::LogicalPosition::new(
                p.pos[0] as f64,
                p.pos[1] as f64,
            ));
            let _ = win.set_always_on_top(true);
            let _ = win.set_ignore_cursor_events(false);
            let _ = win.hide();
            // Force a full webview reload so the overlay script re-runs init().
            let _ = win.eval("window.location.reload()");
            count += 1;
            continue;
        }

        // New window — created hidden. overlay.ts shows it when a timer fires
        // or when the user enters edit mode to reposition overlays.
        let win = WebviewWindowBuilder::new(
            &app,
            &label,
            WebviewUrl::App(format!("overlay.html?index={}", p.index).into()),
        )
        .title("")
        .inner_size(p.win_size as f64, p.win_size as f64)
        .position(p.pos[0] as f64, p.pos[1] as f64)
        .decorations(false)
        .resizable(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()
        .map_err(|e: tauri::Error| e.to_string())?;

        // Re-apply after build to guard against any platform reset.
        // On Windows, transparent child windows are more reliable if their size
        // and position are reaffirmed after creation.
        let _ = win.set_size(LogicalSize::new(p.win_size as f64, p.win_size as f64));
        let _ = win.set_position(tauri::LogicalPosition::new(
            p.pos[0] as f64,
            p.pos[1] as f64,
        ));
        let _ = win.set_always_on_top(true);
        let _ = win.set_ignore_cursor_events(false);
        count += 1;
    }

    log::info!("Shown/created {} overlay windows", count);
    Ok(count)
}

#[tauri::command]
fn close_overlays(app: tauri::AppHandle) -> Result<(), String> {
    hide_all_overlays(&app);
    Ok(())
}

/// Called by overlay.ts when its timer starts — show this overlay window.
#[tauri::command]
fn show_overlay(index: usize, app: tauri::AppHandle) {
    let label = format!("overlay-{}", index);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        let _ = win.set_always_on_top(true);
        let _ = win.set_ignore_cursor_events(true);
    }
}

/// Called by overlay.ts when its timer finishes — hide this overlay window.
#[tauri::command]
fn hide_overlay(index: usize, app: tauri::AppHandle) {
    let label = format!("overlay-{}", index);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.set_ignore_cursor_events(false);
        let _ = win.hide();
    }
}

#[tauri::command]
fn set_watching(
    enabled: bool,
    watching: tauri::State<Watching>,
) {
    watching.store(enabled, std::sync::atomic::Ordering::Relaxed);
    log::info!("Watching: {}", enabled);
}

#[tauri::command]
fn set_overlays_edit_mode(
    app: tauri::AppHandle,
    enabled: bool,
    overlay_configs: tauri::State<OverlayConfigs>,
    active_timers: tauri::State<ActiveTimers>,
) -> Result<(), String> {
    use tauri::Emitter;

    let configs = overlay_configs.lock().map_err(|e| e.to_string())?;
    let timers  = active_timers.lock().map_err(|e| e.to_string())?;

    for index in configs.keys() {
        let label = format!("overlay-{}", index);
        if let Some(win) = app.get_webview_window(&label) {
            if enabled {
                let _ = win.set_ignore_cursor_events(false);
                // Show all overlays so the user can see and reposition them.
                let _ = win.show();
                let _ = win.set_always_on_top(true);
            } else {
                if timers.contains_key(index) {
                    let _ = win.show();
                    let _ = win.set_always_on_top(true);
                    let _ = win.set_ignore_cursor_events(true);
                } else {
                    // Hide overlays whose timer is not currently running.
                    // Rust is authoritative here — don't rely on JS event delivery.
                    let _ = win.set_ignore_cursor_events(false);
                    let _ = win.hide();
                }
            }
            let _ = win.emit("edit-mode-changed", enabled);
        }
    }

    // When exiting edit mode, read all window positions from the OS and
    // send them to the main window so it can persist them.
    // outer_position() returns physical pixels; divide by scale_factor to get
    // logical pixels, which is what WebviewWindowBuilder::position() expects.
    if !enabled {
        let mut positions: HashMap<usize, [i32; 2]> = HashMap::new();
        for index in configs.keys() {
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
            let _ = win.hide();
        }
    }
    Ok(())
}

// ── macOS privacy settings helper ────────────────────────────────────────────

/// Resets the Input Monitoring TCC permission for this app using `tccutil`.
///
/// On macOS the TCC permission entry is keyed to the app's code signature.
/// A rebuild changes the binary hash, making the existing permission entry
/// stale — the OS rejects CGEventTap even though the pane still shows "allowed".
///
/// Running `tccutil reset ListenEvent <bundle_id>` removes the stale entry so
/// the app can request fresh permission on next launch.  After calling this
/// command the user must quit and restart the app.
#[tauri::command]
fn reset_input_monitoring_permission() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // Step 1: Try to remove the stale TCC entry via tccutil.
        // This may silently fail on macOS 14+ from within an app, but is worth trying.
        let _ = std::process::Command::new("tccutil")
            .args(["reset", "ListenEvent", "com.vanyushkin.d2rshowmewhen"])
            .output();

        // Step 2: Trigger the native system permission dialog.
        // After tccutil removed the entry (or if it was already absent/denied),
        // IOHIDRequestAccess shows the "Allow D2RShowMeWhen to monitor input" dialog.
        // The user clicks Allow, then restarts the app once.
        hotkeys::request_input_monitoring_access();
    }
    Ok(())
}

/// Opens System Settings → Privacy & Security → Input Monitoring on macOS.
/// On other platforms this is a no-op that always succeeds.
///
/// On macOS 14+ (Sonoma and later) the bundle hash changes on every rebuild,
/// so Input Monitoring permission must be re-granted after each install/update.
/// This command gives the user a one-click way to open the relevant pane and
/// avoids the need to navigate deep into System Settings manually.
#[tauri::command]
fn open_privacy_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // Works on macOS 12 (Monterey) through macOS 15 (Sequoia).
        // On macOS 13+ the URL redirects to the new System Settings app.
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
    let watching: Watching = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let global_hotkeys: GlobalHotkeys = Default::default();

    tauri::Builder::default()
        .manage(Mutex::new(MigrationRecord { migrated_from: None }))
        .manage(registrations.clone())
        .manage(active_timers.clone())
        .manage(overlay_configs)
        .manage(watching.clone())
        .manage(global_hotkeys.clone())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .on_window_event(|window, event| {
            // Clicking the red close button (or ⌘Q) fully quits the app.
            // Overlay windows are decorationless and never receive CloseRequested.
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    window.app_handle().exit(0);
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
            get_overlay_config,
            get_active_timer_state,
            set_watching,
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
            }
            hotkeys::start(app.handle().clone(), registrations, active_timers, watching, global_hotkeys);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
