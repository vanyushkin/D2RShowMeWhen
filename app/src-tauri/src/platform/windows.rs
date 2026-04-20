use crate::core::models::{BackendAdapterInfo, OverlayCapabilities};

pub fn descriptor() -> (OverlayCapabilities, BackendAdapterInfo) {
    (
        OverlayCapabilities {
            global_hotkeys: true,
            transparent_overlay: true,
            click_through: true,
            notes: vec![
                "Windows parity will rely on Win32 hooks and layered transparent windows.".into(),
            ],
        },
        BackendAdapterInfo {
            name: "Win32 adapter".into(),
            stage: "planned".into(),
            next_steps: vec![
                "Mirror macOS runtime contracts for hotkeys and overlay timers.".into(),
                "Add layered-window overlay renderer.".into(),
            ],
        },
    )
}
