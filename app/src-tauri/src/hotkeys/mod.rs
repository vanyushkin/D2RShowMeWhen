use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use rdev::{Button, EventType, Key};
use serde::Serialize;
use tauri::Emitter;

// ── Public type aliases managed by Tauri ─────────────────────────────────────

pub type Registrations = Arc<Mutex<Vec<HotkeyRegistration>>>;
pub type ActiveTimers = Arc<Mutex<HashMap<usize, ActiveTimer>>>;
/// When false, hotkeys are captured and modifiers tracked, but timers are NOT fired.
pub type Watching = Arc<AtomicBool>;

/// Global action hotkeys: hide/show overlays and toggle layout-edit mode.
/// These fire regardless of the `Watching` flag.
#[derive(Debug, Default, Clone)]
pub struct GlobalHotkeyConfig {
    pub hide_show:           Option<ParsedHotkey>,
    pub hide_show2:          Option<ParsedHotkey>,
    pub hide_show_enabled:   bool,
    pub layout_edit:         Option<ParsedHotkey>,
    pub layout_edit2:        Option<ParsedHotkey>,
    pub layout_edit_enabled: bool,
}

pub type GlobalHotkeys = Arc<Mutex<GlobalHotkeyConfig>>;

// ── Domain types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HotkeyModifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HotkeyTrigger {
    Key(Key),
    Mouse(Button),
}

#[derive(Debug, Clone)]
pub struct ParsedHotkey {
    pub modifiers: HashSet<HotkeyModifier>,
    pub trigger: HotkeyTrigger,
}

#[derive(Debug, Clone)]
pub struct HotkeyRegistration {
    pub timer_index: usize,
    pub hotkey: ParsedHotkey,
    pub duration_secs: u64,
    pub blink_threshold_secs: u64,
}

#[derive(Debug, Clone)]
pub struct ActiveTimer {
    pub start: Instant,
    pub duration_secs: u64,
    pub blink_threshold_secs: u64,
    pub last_emitted_remaining: u64,
}

// ── Frontend event payloads ───────────────────────────────────────────────────

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerTickPayload {
    pub timer_index: usize,
    pub remaining_secs: u64,
    pub total_secs: u64,
    pub blinking: bool,
    pub finished: bool,
}

// ── Hotkey string parser ──────────────────────────────────────────────────────

/// Parse a hotkey string like "ctrl+f3", "f9", "mouse_left", "cmd+1".
pub fn parse_hotkey_string(s: &str) -> Option<ParsedHotkey> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let parts: Vec<&str> = s.split('+').map(str::trim).collect();
    let mut modifiers = HashSet::new();
    let mut trigger: Option<HotkeyTrigger> = None;

    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => {
                modifiers.insert(HotkeyModifier::Ctrl);
            }
            "alt" | "option" => {
                modifiers.insert(HotkeyModifier::Alt);
            }
            "shift" => {
                modifiers.insert(HotkeyModifier::Shift);
            }
            "cmd" | "meta" | "super" | "win" | "windows" => {
                modifiers.insert(HotkeyModifier::Meta);
            }
            other => {
                trigger = parse_trigger(other);
            }
        }
    }

    trigger.map(|t| ParsedHotkey { modifiers, trigger: t })
}

fn parse_trigger(s: &str) -> Option<HotkeyTrigger> {
    match s.to_lowercase().as_str() {
        "mouse_left" | "mouseleft" => Some(HotkeyTrigger::Mouse(Button::Left)),
        "mouse_right" | "mouseright" => Some(HotkeyTrigger::Mouse(Button::Right)),
        "mouse_middle" | "mousemiddle" => Some(HotkeyTrigger::Mouse(Button::Middle)),
        "mouse_x1" | "mousex1" => Some(HotkeyTrigger::Mouse(Button::Unknown(1))),
        "mouse_x2" | "mousex2" => Some(HotkeyTrigger::Mouse(Button::Unknown(2))),
        other => parse_key_name(other).map(HotkeyTrigger::Key),
    }
}

fn parse_key_name(s: &str) -> Option<Key> {
    match s {
        "a" => Some(Key::KeyA),
        "b" => Some(Key::KeyB),
        "c" => Some(Key::KeyC),
        "d" => Some(Key::KeyD),
        "e" => Some(Key::KeyE),
        "f" => Some(Key::KeyF),
        "g" => Some(Key::KeyG),
        "h" => Some(Key::KeyH),
        "i" => Some(Key::KeyI),
        "j" => Some(Key::KeyJ),
        "k" => Some(Key::KeyK),
        "l" => Some(Key::KeyL),
        "m" => Some(Key::KeyM),
        "n" => Some(Key::KeyN),
        "o" => Some(Key::KeyO),
        "p" => Some(Key::KeyP),
        "q" => Some(Key::KeyQ),
        "r" => Some(Key::KeyR),
        "s" => Some(Key::KeyS),
        "t" => Some(Key::KeyT),
        "u" => Some(Key::KeyU),
        "v" => Some(Key::KeyV),
        "w" => Some(Key::KeyW),
        "x" => Some(Key::KeyX),
        "y" => Some(Key::KeyY),
        "z" => Some(Key::KeyZ),
        "0" => Some(Key::Num0),
        "1" => Some(Key::Num1),
        "2" => Some(Key::Num2),
        "3" => Some(Key::Num3),
        "4" => Some(Key::Num4),
        "5" => Some(Key::Num5),
        "6" => Some(Key::Num6),
        "7" => Some(Key::Num7),
        "8" => Some(Key::Num8),
        "9" => Some(Key::Num9),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        "space" => Some(Key::Space),
        "enter" | "return" => Some(Key::Return),
        "esc" | "escape" => Some(Key::Escape),
        "tab" => Some(Key::Tab),
        "backspace" => Some(Key::Backspace),
        "delete" | "del" => Some(Key::Delete),
        "up" => Some(Key::UpArrow),
        "down" => Some(Key::DownArrow),
        "left" => Some(Key::LeftArrow),
        "right" => Some(Key::RightArrow),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" | "page_up" => Some(Key::PageUp),
        "pagedown" | "page_down" => Some(Key::PageDown),
        "insert" => Some(Key::Insert),
        "capslock" | "caps_lock" | "caps" => Some(Key::CapsLock),
        // Symbol / punctuation keys (US ANSI layout names)
        "-" | "minus"       => Some(Key::Minus),
        "=" | "equal"       => Some(Key::Equal),
        "`" | "backquote" | "grave" => Some(Key::BackQuote),
        "[" | "leftbracket"  => Some(Key::LeftBracket),
        "]" | "rightbracket" => Some(Key::RightBracket),
        ";" | "semicolon"    => Some(Key::SemiColon),
        "'" | "quote" | "apostrophe" => Some(Key::Quote),
        "\\" | "backslash"   => Some(Key::BackSlash),
        "," | "comma"        => Some(Key::Comma),
        "." | "dot" | "period" => Some(Key::Dot),
        "/" | "slash"        => Some(Key::Slash),
        // Numpad operators
        "kp-" | "kpminus" | "numminus"   => Some(Key::KpMinus),
        "kp+" | "kpplus"  | "numplus"    => Some(Key::KpPlus),
        "kp*" | "kpmultiply"             => Some(Key::KpMultiply),
        "kp/" | "kpdivide"               => Some(Key::KpDivide),
        "num0" | "kp0" => Some(Key::Kp0),
        "num1" | "kp1" => Some(Key::Kp1),
        "num2" | "kp2" => Some(Key::Kp2),
        "num3" | "kp3" => Some(Key::Kp3),
        "num4" | "kp4" => Some(Key::Kp4),
        "num5" | "kp5" => Some(Key::Kp5),
        "num6" | "kp6" => Some(Key::Kp6),
        "num7" | "kp7" => Some(Key::Kp7),
        "num8" | "kp8" => Some(Key::Kp8),
        "num9" | "kp9" => Some(Key::Kp9),
        _ => None,
    }
}

// ── Hotkey match helper ───────────────────────────────────────────────────────

fn matches_hotkey(hk: &ParsedHotkey, trigger: &HotkeyTrigger, pressed_mods: &HashSet<HotkeyModifier>) -> bool {
    if &hk.trigger != trigger {
        return false;
    }
    let effective: HashSet<HotkeyModifier> =
        if hk.modifiers.contains(&HotkeyModifier::Shift) {
            pressed_mods.clone()
        } else {
            pressed_mods.iter().filter(|m| **m != HotkeyModifier::Shift).cloned().collect()
        };
    hk.modifiers == effective
}

// ── Modifier state helpers ────────────────────────────────────────────────────

fn key_to_modifier(key: &Key) -> Option<HotkeyModifier> {
    match key {
        Key::ControlLeft | Key::ControlRight => Some(HotkeyModifier::Ctrl),
        Key::Alt | Key::AltGr => Some(HotkeyModifier::Alt),
        Key::ShiftLeft | Key::ShiftRight => Some(HotkeyModifier::Shift),
        Key::MetaLeft | Key::MetaRight => Some(HotkeyModifier::Meta),
        _ => None,
    }
}

// ── Platform-specific listener module ────────────────────────────────────────

/// On macOS, rdev calls TSMGetInputSourceProperty (Text Services Manager) on
/// every key event from a background thread, triggering a dispatch_assert_queue
/// SIGTRAP on macOS 14+. We replace rdev::listen with our own CGEventTap that
/// maps keycodes without going through TSM.
#[cfg(target_os = "macos")]
mod macos_tap;

/// Trigger the macOS system permission dialog for Input Monitoring.
/// No-op on non-macOS platforms.
pub fn request_input_monitoring_access() {
    #[cfg(target_os = "macos")]
    macos_tap::request_input_monitoring_access();
}

// ── Listener threads ──────────────────────────────────────────────────────────

pub fn start(
    app: tauri::AppHandle,
    registrations: Registrations,
    active_timers: ActiveTimers,
    watching: Watching,
    global_hotkeys: GlobalHotkeys,
) {
    let (event_tx, event_rx) = std::sync::mpsc::channel::<rdev::Event>();

    // Thread 1: OS input capture — sends raw events via channel.
    let error_app = app.clone();

    #[cfg(target_os = "macos")]
    std::thread::spawn(move || {
        if let Err(msg) = macos_tap::listen(event_tx) {
            log::error!("Global input listener failed to start: {}", msg);
            let _ = error_app.emit("hotkey_listener_error", msg);
        }
    });

    #[cfg(not(target_os = "macos"))]
    std::thread::spawn(move || {
        if let Err(err) = rdev::listen(move |ev| {
            let _ = event_tx.send(ev);
        }) {
            log::error!("Global input listener failed to start: {:?}", err);
            let msg = format!(
                "Input monitoring unavailable ({:?}). \
                 On macOS grant Input Monitoring in System Settings → Privacy.",
                err
            );
            let _ = error_app.emit("hotkey_listener_error", msg);
        }
    });

    // Thread 2: hotkey matching — reads events, fires timers when hotkeys match.
    let app2 = app.clone();
    let regs2 = registrations.clone();
    let timers2 = active_timers.clone();
    let watching2 = watching.clone();
    let global2 = global_hotkeys.clone();
    std::thread::spawn(move || {
        let mut pressed_mods: HashSet<HotkeyModifier> = HashSet::new();

        while let Ok(event) = event_rx.recv() {
            match &event.event_type {
                EventType::KeyPress(key) => {
                    if let Some(modifier) = key_to_modifier(key) {
                        // Always track modifiers, even when not watching.
                        pressed_mods.insert(modifier);
                    } else {
                        check_and_fire(
                            &app2,
                            &regs2,
                            &timers2,
                            &pressed_mods,
                            &HotkeyTrigger::Key(key.clone()),
                            &watching2,
                            &global2,
                        );
                    }
                }
                EventType::KeyRelease(key) => {
                    if let Some(modifier) = key_to_modifier(key) {
                        pressed_mods.remove(&modifier);
                    }
                }
                EventType::ButtonPress(btn) => {
                    check_and_fire(
                        &app2,
                        &regs2,
                        &timers2,
                        &pressed_mods,
                        &HotkeyTrigger::Mouse(btn.clone()),
                        &watching2,
                        &global2,
                    );
                }
                _ => {}
            }
        }
    });

    // Thread 3: timer tick loop — emits timer_tick events every ~250 ms.
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        tick_timers(&app, &active_timers);
    });
}

fn check_and_fire(
    app: &tauri::AppHandle,
    registrations: &Registrations,
    active_timers: &ActiveTimers,
    pressed_mods: &HashSet<HotkeyModifier>,
    trigger: &HotkeyTrigger,
    watching: &Watching,
    global_hotkeys: &GlobalHotkeys,
) {
    // Timer hotkeys — only fire when watching is active.
    if watching.load(Ordering::Relaxed) {
        let regs = registrations.lock().unwrap();
        for reg in regs.iter() {
            if &reg.hotkey.trigger != trigger {
                continue;
            }

            // Case-insensitive: ignore Shift in pressed modifiers when hotkey doesn't need it.
            let effective_pressed: HashSet<HotkeyModifier> =
                if reg.hotkey.modifiers.contains(&HotkeyModifier::Shift) {
                    pressed_mods.clone()
                } else {
                    pressed_mods
                        .iter()
                        .filter(|m| **m != HotkeyModifier::Shift)
                        .cloned()
                        .collect()
                };
            if reg.hotkey.modifiers != effective_pressed {
                continue;
            }

            // Start (or restart) this timer.
            {
                let mut timers = active_timers.lock().unwrap();
                timers.insert(
                    reg.timer_index,
                    ActiveTimer {
                        start: Instant::now(),
                        duration_secs: reg.duration_secs,
                        blink_threshold_secs: reg.blink_threshold_secs,
                        last_emitted_remaining: reg.duration_secs + 1, // force first emit
                    },
                );
            }

            let _ = app.emit(
                "timer_tick",
                TimerTickPayload {
                    timer_index: reg.timer_index,
                    remaining_secs: reg.duration_secs,
                    total_secs: reg.duration_secs,
                    blinking: false,
                    finished: false,
                },
            );
        }
    }

    // Global action hotkeys — fire independently of the watching flag.
    // These allow the user to show/hide overlays or toggle edit mode from the game.
    // Both primary and alt-layout (hotkey2) variants are checked.
    {
        let global = global_hotkeys.lock().unwrap();
        if global.hide_show_enabled {
            if let Some(ref hk) = global.hide_show {
                if matches_hotkey(hk, trigger, pressed_mods) {
                    let _ = app.emit("hotkey_action", "hide_show");
                }
            }
            if let Some(ref hk) = global.hide_show2 {
                if matches_hotkey(hk, trigger, pressed_mods) {
                    let _ = app.emit("hotkey_action", "hide_show");
                }
            }
        }
        if global.layout_edit_enabled {
            if let Some(ref hk) = global.layout_edit {
                if matches_hotkey(hk, trigger, pressed_mods) {
                    let _ = app.emit("hotkey_action", "layout_edit");
                }
            }
            if let Some(ref hk) = global.layout_edit2 {
                if matches_hotkey(hk, trigger, pressed_mods) {
                    let _ = app.emit("hotkey_action", "layout_edit");
                }
            }
        }
    }
}

fn tick_timers(app: &tauri::AppHandle, active_timers: &ActiveTimers) {
    let now = Instant::now();
    let mut finished_indices = Vec::new();

    {
        let mut timers = active_timers.lock().unwrap();
        for (index, timer) in timers.iter_mut() {
            let elapsed = now.duration_since(timer.start).as_secs_f64();
            let remaining_f = (timer.duration_secs as f64 - elapsed).max(0.0);
            let remaining = remaining_f.ceil() as u64;

            if remaining == timer.last_emitted_remaining {
                continue;
            }
            timer.last_emitted_remaining = remaining;

            let blinking = remaining <= timer.blink_threshold_secs;
            let finished = remaining == 0;

            let _ = app.emit(
                "timer_tick",
                TimerTickPayload {
                    timer_index: *index,
                    remaining_secs: remaining,
                    total_secs: timer.duration_secs,
                    blinking,
                    finished,
                },
            );

            if finished {
                finished_indices.push(*index);
            }
        }
        for index in finished_indices {
            timers.remove(&index);
        }
    }
}
