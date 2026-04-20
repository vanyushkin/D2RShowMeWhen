use crate::core::models::{BackendAdapterInfo, OverlayCapabilities};

pub fn descriptor() -> (OverlayCapabilities, BackendAdapterInfo) {
    (
        OverlayCapabilities {
            global_hotkeys: true,
            transparent_overlay: true,
            click_through: true,
            notes: vec![
                "Quartz event taps and native macOS overlay windows are the primary target.".into(),
                "The Tauri shell is now ready for moving listener and overlay logic into Rust.".into(),
            ],
        },
        BackendAdapterInfo {
            name: "Quartz macOS adapter".into(),
            stage: "scaffolded".into(),
            next_steps: vec![
                "Implement accessibility and input-monitoring diagnostics.".into(),
                "Add global hotkey capture via Quartz event tap.".into(),
                "Render transparent click-through overlay windows from Rust.".into(),
            ],
        },
    )
}
