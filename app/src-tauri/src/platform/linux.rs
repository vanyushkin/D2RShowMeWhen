use crate::core::models::{BackendAdapterInfo, OverlayCapabilities};

pub fn x11_descriptor() -> (OverlayCapabilities, BackendAdapterInfo) {
    (
        OverlayCapabilities {
            global_hotkeys: true,
            transparent_overlay: true,
            click_through: false,
            notes: vec![
                "The current X11 runtime uses the shared desktop path: rdev hotkeys plus Tauri overlay windows.".into(),
                "Steam Deck and compositor-specific behaviour still need live validation before this can be considered stable.".into(),
            ],
        },
        BackendAdapterInfo {
            name: "Linux X11 adapter".into(),
            stage: "experimental".into(),
            next_steps: vec![
                "Validate overlay visibility and input behaviour on Steam Deck Desktop Mode and gamescope-adjacent setups.".into(),
                "Decide whether click-through remains compositor-dependent or gets an explicit X11-specific implementation.".into(),
            ],
        },
    )
}

pub fn wayland_descriptor() -> (OverlayCapabilities, BackendAdapterInfo) {
    (
        OverlayCapabilities {
            global_hotkeys: false,
            transparent_overlay: false,
            click_through: false,
            notes: vec![
                "Wayland imposes major security restrictions on global hooks and overlays.".into(),
                "A compositor-specific strategy is likely required and remains a later phase.".into(),
            ],
        },
        BackendAdapterInfo {
            name: "Wayland adapter".into(),
            stage: "research".into(),
            next_steps: vec![
                "Map compositor-specific constraints for Steam Deck and desktop Linux targets.".into(),
                "Decide whether Wayland support is native, portal-based, or explicitly unsupported.".into(),
            ],
        },
    )
}
