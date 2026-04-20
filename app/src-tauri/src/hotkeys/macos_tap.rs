/// macOS-specific global event tap using CGEventTap directly.
///
/// rdev on macOS 14+ calls TSMGetInputSourceProperty (Text Services Manager)
/// from a background thread on every key event, triggering a dispatch_assert_queue
/// failure (SIGTRAP). This module replaces rdev::listen on macOS with a raw
/// CGEventTap that maps CGKeyCode → rdev::Key using a static lookup table,
/// completely bypassing TSM.
///
/// On other platforms, rdev::listen is used as-is (see hotkeys/mod.rs).
use std::ffi::c_void;
use std::sync::mpsc;
use std::time::SystemTime;

use rdev::{Button, Event, EventType, Key};

// ── FFI types ─────────────────────────────────────────────────────────────────

type CGEventRef = *mut c_void;
type CFMachPortRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFAllocatorRef = *const c_void;

// ── CoreGraphics framework ────────────────────────────────────────────────────

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: unsafe extern "C" fn(
            proxy: *mut c_void,
            event_type: u32,
            event: CGEventRef,
            user_info: *mut c_void,
        ) -> CGEventRef,
        user_info: *mut c_void,
    ) -> CFMachPortRef;

    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventGetFlags(event: CGEventRef) -> u64;
}

// ── IOKit framework ───────────────────────────────────────────────────────────

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    /// Request Input Monitoring access. Shows the macOS system permission dialog
    /// when the entry is absent or denied. Returns true if already granted.
    fn IOHIDRequestAccess(request_type: u32) -> bool;
    /// Check current Input Monitoring access status.
    /// Returns 0 = granted, 1 = denied, 2 = unknown.
    fn IOHIDCheckAccess(request_type: u32) -> u32;
}

/// kIOHIDRequestTypeListenEvent = 1  (Input Monitoring)
const KIOHID_REQUEST_TYPE_LISTEN_EVENT: u32 = 1;

/// Trigger the macOS system permission dialog for Input Monitoring.
/// Safe to call from any thread on macOS 10.15+.
pub fn request_input_monitoring_access() {
    unsafe { IOHIDRequestAccess(KIOHID_REQUEST_TYPE_LISTEN_EVENT); }
}

// ── CoreFoundation framework ──────────────────────────────────────────────────

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFRunLoopCommonModes: *const c_void;

    fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;

    fn CFMachPortInvalidate(port: CFMachPortRef);

    fn CFRunLoopGetCurrent() -> CFRunLoopRef;

    fn CFRunLoopAddSource(
        rl: CFRunLoopRef,
        source: CFRunLoopSourceRef,
        mode: *const c_void,
    );

    fn CFRunLoopRun();

    fn CFRelease(cf: *const c_void);
}

// ── Constants ─────────────────────────────────────────────────────────────────

// CGEventTapLocation
const KCG_HID_EVENT_TAP: u32 = 0;
// CGEventTapPlacement
const KCG_HEAD_INSERT_EVENT_TAP: u32 = 0;
// CGEventTapOptions
const KCG_EVENT_TAP_OPTION_LISTEN_ONLY: u32 = 1;

// CGEventType values
const EV_LEFT_MOUSE_DOWN: u32 = 1;
const EV_LEFT_MOUSE_UP: u32 = 2;
const EV_RIGHT_MOUSE_DOWN: u32 = 3;
const EV_RIGHT_MOUSE_UP: u32 = 4;
const EV_KEY_DOWN: u32 = 10;
const EV_KEY_UP: u32 = 11;
const EV_FLAGS_CHANGED: u32 = 12;
const EV_OTHER_MOUSE_DOWN: u32 = 25;
const EV_OTHER_MOUSE_UP: u32 = 26;

// CGEventField values (context-specific: same integer, different meaning per event type)
const FIELD_KEYBOARD_KEYCODE: u32 = 9;
const FIELD_MOUSE_BUTTON_NUMBER: u32 = 9;

// CGEventFlags bits for modifier keys
const FLAG_CAPS_LOCK: u64 = 0x0001_0000;
const FLAG_SHIFT: u64 = 0x0002_0000;
const FLAG_CONTROL: u64 = 0x0004_0000;
const FLAG_ALTERNATE: u64 = 0x0008_0000; // Option / Alt
const FLAG_COMMAND: u64 = 0x0010_0000; // Cmd / Meta

fn event_mask() -> u64 {
    (1u64 << EV_KEY_DOWN)
        | (1u64 << EV_KEY_UP)
        | (1u64 << EV_FLAGS_CHANGED)
        | (1u64 << EV_LEFT_MOUSE_DOWN)
        | (1u64 << EV_LEFT_MOUSE_UP)
        | (1u64 << EV_RIGHT_MOUSE_DOWN)
        | (1u64 << EV_RIGHT_MOUSE_UP)
        | (1u64 << EV_OTHER_MOUSE_DOWN)
        | (1u64 << EV_OTHER_MOUSE_UP)
}

// ── Key mapping ───────────────────────────────────────────────────────────────

/// Map macOS virtual key code (CGKeyCode / Carbon kVK_* constants) to rdev::Key.
///
/// These are hardware scan-code based and do NOT depend on the keyboard layout
/// or the Text Services Manager — they are stable across all locales.
fn keycode_to_key(code: u16) -> Key {
    match code {
        // Letters (ANSI positions — physical key, not the character)
        0x00 => Key::KeyA,
        0x01 => Key::KeyS,
        0x02 => Key::KeyD,
        0x03 => Key::KeyF,
        0x04 => Key::KeyH,
        0x05 => Key::KeyG,
        0x06 => Key::KeyZ,
        0x07 => Key::KeyX,
        0x08 => Key::KeyC,
        0x09 => Key::KeyV,
        0x0B => Key::KeyB,
        0x0C => Key::KeyQ,
        0x0D => Key::KeyW,
        0x0E => Key::KeyE,
        0x0F => Key::KeyR,
        0x10 => Key::KeyY,
        0x11 => Key::KeyT,
        0x1F => Key::KeyO,
        0x20 => Key::KeyU,
        0x22 => Key::KeyI,
        0x23 => Key::KeyP,
        0x25 => Key::KeyL,
        0x26 => Key::KeyJ,
        0x28 => Key::KeyK,
        0x2D => Key::KeyN,
        0x2E => Key::KeyM,
        // Digit row punctuation (between digit row and Backspace)
        0x1B => Key::Minus,        // -/_
        0x18 => Key::Equal,        // =/+
        // Punctuation / symbol keys
        0x32 => Key::BackQuote,    // `/~
        0x21 => Key::LeftBracket,  // [/{
        0x1E => Key::RightBracket, // ]/}
        0x29 => Key::SemiColon,    // ;/:
        0x27 => Key::Quote,        // '/""
        0x2A => Key::BackSlash,    // \/|
        0x2B => Key::Comma,        // ,/<
        0x2F => Key::Dot,          // ./>
        0x2C => Key::Slash,        // /?
        // Numpad operators
        0x4E => Key::KpMinus,
        0x45 => Key::KpPlus,
        0x43 => Key::KpMultiply,
        0x4B => Key::KpDivide,
        // Digit row
        0x12 => Key::Num1,
        0x13 => Key::Num2,
        0x14 => Key::Num3,
        0x15 => Key::Num4,
        0x16 => Key::Num6,
        0x17 => Key::Num5,
        0x19 => Key::Num9,
        0x1A => Key::Num7,
        0x1C => Key::Num8,
        0x1D => Key::Num0,
        // Special keys
        0x24 => Key::Return,
        0x30 => Key::Tab,
        0x31 => Key::Space,
        0x33 => Key::Backspace,
        0x35 => Key::Escape,
        0x72 => Key::Insert,
        0x73 => Key::Home,
        0x74 => Key::PageUp,
        0x75 => Key::Delete,
        0x77 => Key::End,
        0x79 => Key::PageDown,
        0x7B => Key::LeftArrow,
        0x7C => Key::RightArrow,
        0x7D => Key::DownArrow,
        0x7E => Key::UpArrow,
        // Function keys
        0x7A => Key::F1,
        0x78 => Key::F2,
        0x63 => Key::F3,
        0x76 => Key::F4,
        0x60 => Key::F5,
        0x61 => Key::F6,
        0x62 => Key::F7,
        0x64 => Key::F8,
        0x65 => Key::F9,
        0x6D => Key::F10,
        0x67 => Key::F11,
        0x6F => Key::F12,
        // Modifiers
        0x38 => Key::ShiftLeft,
        0x3C => Key::ShiftRight,
        0x3B => Key::ControlLeft,
        0x3E => Key::ControlRight,
        0x3A => Key::Alt,    // Option Left
        0x3D => Key::AltGr,  // Option Right
        0x37 => Key::MetaLeft,
        0x36 => Key::MetaRight,
        0x39 => Key::CapsLock,
        // Numpad
        0x52 => Key::Kp0,
        0x53 => Key::Kp1,
        0x54 => Key::Kp2,
        0x55 => Key::Kp3,
        0x56 => Key::Kp4,
        0x57 => Key::Kp5,
        0x58 => Key::Kp6,
        0x59 => Key::Kp7,
        0x5B => Key::Kp8,
        0x5C => Key::Kp9,
        // Anything else: pass raw code through
        other => Key::Unknown(other as u32),
    }
}

/// Return the CGEventFlags bitmask for a given modifier keycode, or 0 if unknown.
fn keycode_to_flag(code: u16) -> u64 {
    match code {
        0x38 | 0x3C => FLAG_SHIFT,
        0x3B | 0x3E => FLAG_CONTROL,
        0x3A | 0x3D => FLAG_ALTERNATE,
        0x37 | 0x36 => FLAG_COMMAND,
        0x39 => FLAG_CAPS_LOCK,
        _ => 0,
    }
}

// ── Callback context ──────────────────────────────────────────────────────────

struct TapContext {
    tx: mpsc::Sender<Event>,
}

// SAFETY: TapContext is only accessed from the single CFRunLoop thread.
unsafe impl Send for TapContext {}

// ── Event tap callback ────────────────────────────────────────────────────────

unsafe extern "C" fn tap_callback(
    _proxy: *mut c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    let ctx = &*(user_info as *const TapContext);
    let time = SystemTime::now();

    let maybe_event_type: Option<EventType> = match event_type {
        EV_KEY_DOWN => {
            let code = CGEventGetIntegerValueField(event, FIELD_KEYBOARD_KEYCODE) as u16;
            Some(EventType::KeyPress(keycode_to_key(code)))
        }
        EV_KEY_UP => {
            let code = CGEventGetIntegerValueField(event, FIELD_KEYBOARD_KEYCODE) as u16;
            Some(EventType::KeyRelease(keycode_to_key(code)))
        }
        EV_FLAGS_CHANGED => {
            // Modifier key pressed or released.
            // We determine press vs release by checking whether the corresponding
            // flag bit is currently set in the event's CGEventFlags.
            let code = CGEventGetIntegerValueField(event, FIELD_KEYBOARD_KEYCODE) as u16;
            let flags = CGEventGetFlags(event);
            let flag_bit = keycode_to_flag(code);
            let is_press = (flags & flag_bit) != 0;
            let key = keycode_to_key(code);
            Some(if is_press {
                EventType::KeyPress(key)
            } else {
                EventType::KeyRelease(key)
            })
        }
        EV_LEFT_MOUSE_DOWN => Some(EventType::ButtonPress(Button::Left)),
        EV_LEFT_MOUSE_UP => Some(EventType::ButtonRelease(Button::Left)),
        EV_RIGHT_MOUSE_DOWN => Some(EventType::ButtonPress(Button::Right)),
        EV_RIGHT_MOUSE_UP => Some(EventType::ButtonRelease(Button::Right)),
        EV_OTHER_MOUSE_DOWN => {
            let n = CGEventGetIntegerValueField(event, FIELD_MOUSE_BUTTON_NUMBER) as u32;
            let btn = match n {
                2 => Button::Middle,
                other => Button::Unknown(other as u8),
            };
            Some(EventType::ButtonPress(btn))
        }
        EV_OTHER_MOUSE_UP => {
            let n = CGEventGetIntegerValueField(event, FIELD_MOUSE_BUTTON_NUMBER) as u32;
            let btn = match n {
                2 => Button::Middle,
                other => Button::Unknown(other as u8),
            };
            Some(EventType::ButtonRelease(btn))
        }
        _ => None,
    };

    if let Some(et) = maybe_event_type {
        let _ = ctx.tx.send(Event { event_type: et, time, name: None });
    }

    // In listen-only mode the return value is ignored by the OS, but must be
    // the original event pointer.
    event
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Install a CGEventTap and block the calling thread on CFRunLoopRun().
///
/// Returns `Err` if the tap could not be created (Input Monitoring not granted).
/// On success this function never returns (runs until process exit).
pub fn listen(tx: mpsc::Sender<Event>) -> Result<(), String> {
    let ctx = Box::new(TapContext { tx });
    let ctx_ptr = Box::into_raw(ctx) as *mut c_void;

    unsafe {
        let tap = CGEventTapCreate(
            KCG_HID_EVENT_TAP,
            KCG_HEAD_INSERT_EVENT_TAP,
            KCG_EVENT_TAP_OPTION_LISTEN_ONLY,
            event_mask(),
            tap_callback,
            ctx_ptr,
        );

        if tap.is_null() {
            drop(Box::from_raw(ctx_ptr as *mut TapContext));

            // Automatically trigger the macOS permission dialog.
            // - First launch / never granted → dialog appears immediately.
            // - Permission revoked → dialog appears.
            // - Stale entry after rebuild (TCC says "granted" but tap failed) →
            //   IOHIDCheckAccess returns 0 (granted), so no dialog appears here;
            //   user should click "Reset & Request" in the error banner.
            IOHIDRequestAccess(KIOHID_REQUEST_TYPE_LISTEN_EVENT);

            let msg = if IOHIDCheckAccess(KIOHID_REQUEST_TYPE_LISTEN_EVENT) == 0 {
                // TCC reports granted but tap still failed → stale code-signature entry.
                "Input Monitoring permission is stale after a rebuild. \
                 Click \"Reset & Request\" in the banner below, then restart the app."
            } else {
                // Not granted — system dialog was triggered above.
                "Input Monitoring permission requested. \
                 Click Allow in the system dialog, then restart the app."
            };
            return Err(msg.to_string());
        }

        let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
        if source.is_null() {
            CFMachPortInvalidate(tap);
            CFRelease(tap);
            drop(Box::from_raw(ctx_ptr as *mut TapContext));
            return Err("CFMachPortCreateRunLoopSource failed".to_string());
        }

        let rl = CFRunLoopGetCurrent();
        // SAFETY: kCFRunLoopCommonModes is a valid CFStringRef for the lifetime of the process.
        CFRunLoopAddSource(rl, source, kCFRunLoopCommonModes);
        CFRelease(source);
        // The run loop retains the source; tap is referenced by the source.
        CFRelease(tap);

        // Block this thread processing events until process exit.
        CFRunLoopRun();

        // Unreachable in practice — cleanup for completeness.
        drop(Box::from_raw(ctx_ptr as *mut TapContext));
    }

    Ok(())
}
