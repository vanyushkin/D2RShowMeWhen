//! Windows-specific keyboard & mouse polling via GetAsyncKeyState.
//!
//! rdev's WH_KEYBOARD_LL hook silently fails on some Windows 10/11 systems
//! (blocked by security software or Windows session policy while
//! WH_MOUSE_LL still works).  GetAsyncKeyState polling is simpler, more
//! reliable, and the standard approach used by game overlay utilities.
//!
//! Latency is at most one poll interval (10 ms), imperceptible for timers.

use rdev::{Button, Event, EventType, Key};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::SystemTime;

const POLL_MS: u64 = 10;

/// (Windows Virtual Key code, rdev Key)
///
/// Covers every key supported by `parse_key_name()` in this crate.
/// VK codes from: https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
const KEYBOARD_MAP: &[(i32, Key)] = &[
    // Letters A–Z  (VK_A = 0x41 … VK_Z = 0x5A)
    (0x41, Key::KeyA), (0x42, Key::KeyB), (0x43, Key::KeyC), (0x44, Key::KeyD),
    (0x45, Key::KeyE), (0x46, Key::KeyF), (0x47, Key::KeyG), (0x48, Key::KeyH),
    (0x49, Key::KeyI), (0x4A, Key::KeyJ), (0x4B, Key::KeyK), (0x4C, Key::KeyL),
    (0x4D, Key::KeyM), (0x4E, Key::KeyN), (0x4F, Key::KeyO), (0x50, Key::KeyP),
    (0x51, Key::KeyQ), (0x52, Key::KeyR), (0x53, Key::KeyS), (0x54, Key::KeyT),
    (0x55, Key::KeyU), (0x56, Key::KeyV), (0x57, Key::KeyW), (0x58, Key::KeyX),
    (0x59, Key::KeyY), (0x5A, Key::KeyZ),
    // Top number row  (VK_0..VK_9 = 0x30..0x39)
    (0x30, Key::Num0), (0x31, Key::Num1), (0x32, Key::Num2), (0x33, Key::Num3),
    (0x34, Key::Num4), (0x35, Key::Num5), (0x36, Key::Num6), (0x37, Key::Num7),
    (0x38, Key::Num8), (0x39, Key::Num9),
    // Function keys  (VK_F1..VK_F12 = 0x70..0x7B)
    (0x70, Key::F1),  (0x71, Key::F2),  (0x72, Key::F3),  (0x73, Key::F4),
    (0x74, Key::F5),  (0x75, Key::F6),  (0x76, Key::F7),  (0x77, Key::F8),
    (0x78, Key::F9),  (0x79, Key::F10), (0x7A, Key::F11), (0x7B, Key::F12),
    // Navigation / editing
    (0x20, Key::Space),
    (0x0D, Key::Return),
    (0x1B, Key::Escape),
    (0x09, Key::Tab),
    (0x08, Key::Backspace),
    (0x2E, Key::Delete),
    (0x26, Key::UpArrow),    (0x28, Key::DownArrow),
    (0x25, Key::LeftArrow),  (0x27, Key::RightArrow),
    (0x24, Key::Home),       (0x23, Key::End),
    (0x21, Key::PageUp),     (0x22, Key::PageDown),
    (0x2D, Key::Insert),
    (0x14, Key::CapsLock),
    // Punctuation (US ANSI OEM codes)
    (0xBD, Key::Minus),          // VK_OEM_MINUS
    (0xBB, Key::Equal),          // VK_OEM_PLUS  (= sign)
    (0xC0, Key::BackQuote),      // VK_OEM_3     (` ~)
    (0xDB, Key::LeftBracket),    // VK_OEM_4     ([ {)
    (0xDD, Key::RightBracket),   // VK_OEM_6     (] })
    (0xBA, Key::SemiColon),      // VK_OEM_1     (; :)
    (0xDE, Key::Quote),          // VK_OEM_7     (' ")
    (0xDC, Key::BackSlash),      // VK_OEM_5     (\ |)
    (0xBC, Key::Comma),          // VK_OEM_COMMA (, <)
    (0xBE, Key::Dot),            // VK_OEM_PERIOD(. >)
    (0xBF, Key::Slash),          // VK_OEM_2     (/ ?)
    // Numpad
    (0x60, Key::Kp0), (0x61, Key::Kp1), (0x62, Key::Kp2), (0x63, Key::Kp3),
    (0x64, Key::Kp4), (0x65, Key::Kp5), (0x66, Key::Kp6), (0x67, Key::Kp7),
    (0x68, Key::Kp8), (0x69, Key::Kp9),
    (0x6D, Key::KpMinus), (0x6B, Key::KpPlus),
    (0x6A, Key::KpMultiply), (0x6F, Key::KpDivide),
    // Modifier keys (left + right variants)
    (0xA2, Key::ControlLeft), (0xA3, Key::ControlRight),
    (0xA4, Key::Alt),          // VK_LMENU
    (0xA5, Key::AltGr),        // VK_RMENU  (AltGr on EU / RU keyboards)
    (0xA0, Key::ShiftLeft),   (0xA1, Key::ShiftRight),
    (0x5B, Key::MetaLeft),    (0x5C, Key::MetaRight),
];

/// (Windows Virtual Key code, rdev Button)
const MOUSE_MAP: &[(i32, Button)] = &[
    (0x01, Button::Left),          // VK_LBUTTON
    (0x02, Button::Right),         // VK_RBUTTON
    (0x04, Button::Middle),        // VK_MBUTTON
    (0x05, Button::Unknown(1)),    // VK_XBUTTON1
    (0x06, Button::Unknown(2)),    // VK_XBUTTON2
];

extern "system" {
    fn GetAsyncKeyState(nVirtKey: i32) -> i16;
}

/// Returns true if the virtual key is currently physically down.
/// The high-order bit of the return value is set when the key is down.
fn is_down(vk: i32) -> bool {
    (unsafe { GetAsyncKeyState(vk) } as u16) & 0x8000 != 0
}

/// Poll all tracked keys and mouse buttons and forward state-change events
/// through `tx`.  Runs forever until the receiving end of the channel closes.
pub fn poll(tx: Sender<Event>) {
    let mut key_prev: HashMap<i32, bool> = HashMap::new();
    let mut btn_prev: HashMap<i32, bool> = HashMap::new();

    loop {
        std::thread::sleep(std::time::Duration::from_millis(POLL_MS));

        // ── Keyboard ────────────────────────────────────────────────────────
        for &(vk, ref key) in KEYBOARD_MAP {
            let down = is_down(vk);
            let was = *key_prev.get(&vk).unwrap_or(&false);
            if down == was {
                continue;
            }
            key_prev.insert(vk, down);
            let ev_type = if down {
                EventType::KeyPress(key.clone())
            } else {
                EventType::KeyRelease(key.clone())
            };
            if tx.send(Event { time: SystemTime::now(), name: None, event_type: ev_type }).is_err() {
                return;
            }
        }

        // ── Mouse buttons ───────────────────────────────────────────────────
        for &(vk, ref btn) in MOUSE_MAP {
            let down = is_down(vk);
            let was = *btn_prev.get(&vk).unwrap_or(&false);
            if down == was {
                continue;
            }
            btn_prev.insert(vk, down);
            // Only ButtonPress matters for hotkey triggering.
            if down {
                let ev = Event {
                    time: SystemTime::now(),
                    name: None,
                    event_type: EventType::ButtonPress(btn.clone()),
                };
                if tx.send(ev).is_err() {
                    return;
                }
            }
        }
    }
}
