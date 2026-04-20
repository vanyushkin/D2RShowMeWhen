mod linux;
mod macos;
mod windows;

use crate::core::models::{BackendAdapterInfo, OverlayCapabilities, PlatformInfo};

pub fn detect_platform() -> PlatformInfo {
    let kind = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            "linux-wayland"
        } else {
            "linux-x11"
        }
    } else {
        "unknown"
    };

    let (overlay, adapter) = match kind {
        "macos" => macos::descriptor(),
        "windows" => windows::descriptor(),
        "linux-x11" => linux::x11_descriptor(),
        "linux-wayland" => linux::wayland_descriptor(),
        _ => unknown_descriptor(),
    };

    PlatformInfo {
        kind: kind.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        overlay,
        adapter,
    }
}

fn unknown_descriptor() -> (OverlayCapabilities, BackendAdapterInfo) {
    (
        OverlayCapabilities {
            global_hotkeys: false,
            transparent_overlay: false,
            click_through: false,
            notes: vec!["Unknown runtime platform. Native adapter is not implemented yet.".into()],
        },
        BackendAdapterInfo {
            name: "Unknown adapter".into(),
            stage: "unavailable".into(),
            next_steps: vec!["Run the project on a supported desktop target to continue native backend work.".into()],
        },
    )
}
