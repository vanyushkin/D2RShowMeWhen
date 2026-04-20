use crate::core::models::{BackendAdapterInfo, OverlayCapabilities};

pub fn x11_descriptor() -> (OverlayCapabilities, BackendAdapterInfo) {
    (
        OverlayCapabilities {
            global_hotkeys: true,
            transparent_overlay: true,
            click_through: false,
            notes: vec![
                "X11 is the first Linux target for Steam Deck and gamescope-style usage.".into(),
                "Click-through semantics depend on compositor behavior and need dedicated validation.".into(),
            ],
        },
        BackendAdapterInfo {
            name: "X11 adapter".into(),
            stage: "planned".into(),
            next_steps: vec![
                "Validate gamescope/X11 assumptions on Steam Deck.".into(),
                "Implement hotkey listener and transparent overlay path for X11.".into(),
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
