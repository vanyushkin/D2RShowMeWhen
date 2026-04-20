use crate::core::models::{BackendAdapterInfo, OverlayCapabilities};

pub fn descriptor() -> (OverlayCapabilities, BackendAdapterInfo) {
    (
        OverlayCapabilities {
            global_hotkeys: true,
            transparent_overlay: true,
            click_through: true,
            notes: vec![
                "The current Windows runtime uses rdev for global hotkeys and Tauri transparent windows for overlays.".into(),
                "Packaged builds still need validation for transparent window lifecycle and click-through behaviour.".into(),
            ],
        },
        BackendAdapterInfo {
            name: "Windows desktop adapter".into(),
            stage: "experimental".into(),
            next_steps: vec![
                "Harden packaged-release behaviour for show/hide and first-run overlay creation.".into(),
                "Add Windows-specific diagnostics around WebView and transparent overlay failures.".into(),
            ],
        },
    )
}
