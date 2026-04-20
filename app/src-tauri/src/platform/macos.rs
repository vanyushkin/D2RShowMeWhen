use crate::core::models::{BackendAdapterInfo, OverlayCapabilities};

pub fn descriptor() -> (OverlayCapabilities, BackendAdapterInfo) {
    (
        OverlayCapabilities {
            global_hotkeys: true,
            transparent_overlay: true,
            click_through: true,
            notes: vec![
                "macOS uses a custom Quartz event tap for global input and Tauri overlay windows for rendering.".into(),
                "This is the most mature target, but it still relies on runtime permission and window-behaviour validation.".into(),
            ],
        },
        BackendAdapterInfo {
            name: "Quartz macOS adapter".into(),
            stage: "implemented".into(),
            next_steps: vec![
                "Improve Input Monitoring diagnostics and recovery UX.".into(),
                "Continue hardening overlay behaviour across unsigned builds and app updates.".into(),
            ],
        },
    )
}
