use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const MAX_TIMERS: usize = 20;
pub const DEFAULT_DURATION: u16 = 10;
pub const DEFAULT_BLINK_THRESHOLD: u8 = 5;
pub const DEFAULT_SIZE: u16 = 100;
pub const DEFAULT_OPACITY: u8 = 100;
pub const DEFAULT_BLINK_COLOR: &str = "#ff5b5b";

fn default_enabled() -> bool {
    true
}

fn default_hotkey() -> String {
    String::new()
}

fn default_hotkey2() -> String {
    String::new()
}

fn default_duration() -> u16 {
    DEFAULT_DURATION
}

fn default_icon() -> String {
    String::new()
}

fn default_size() -> u16 {
    DEFAULT_SIZE
}

fn default_opacity() -> u8 {
    DEFAULT_OPACITY
}

fn default_blink() -> bool {
    true
}

fn default_blink_threshold() -> u8 {
    DEFAULT_BLINK_THRESHOLD
}

fn default_blink_color() -> String {
    DEFAULT_BLINK_COLOR.to_string()
}

fn default_show_hotkey_labels() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default = "default_hotkey2")]
    pub hotkey2: String,
    #[serde(default = "default_duration")]
    pub duration: u16,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default = "default_size")]
    pub size: u16,
    #[serde(default = "default_opacity")]
    pub opacity: u8,
    #[serde(default = "default_blink")]
    pub blink: bool,
    #[serde(default = "default_blink_threshold", alias = "blink_threshold")]
    pub blink_threshold: u8,
    #[serde(default = "default_blink_color", alias = "blink_color")]
    pub blink_color: String,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            hotkey: default_hotkey(),
            hotkey2: default_hotkey2(),
            duration: default_duration(),
            icon: default_icon(),
            size: default_size(),
            opacity: default_opacity(),
            blink: default_blink(),
            blink_threshold: default_blink_threshold(),
            blink_color: default_blink_color(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(default)]
    pub timers: Vec<TimerConfig>,
    #[serde(default)]
    pub positions: BTreeMap<String, [i32; 2]>,
    #[serde(default = "default_show_hotkey_labels")]
    pub show_hotkey_labels: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    #[serde(default, alias = "active_profile")]
    pub active_profile: String,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
    #[serde(default)]
    pub hide_show_hotkey: String,
    #[serde(default)]
    pub hide_show_hotkey2: String,
    #[serde(default)]
    pub hide_show_enabled: bool,
    #[serde(default)]
    pub layout_edit_hotkey: String,
    #[serde(default)]
    pub layout_edit_hotkey2: String,
    #[serde(default)]
    pub layout_edit_enabled: bool,
    /// When true, overlays open automatically on app launch.
    #[serde(default)]
    pub auto_show_overlays: bool,
}

impl AppState {
    pub fn with_default_icon(default_icon: Option<&str>) -> Self {
        let mut profiles = BTreeMap::new();
        let mut default_timer = TimerConfig::default();
        if let Some(icon) = default_icon {
            default_timer.icon = icon.to_string();
        }

        profiles.insert(
            "Default".to_string(),
            Profile {
                timers: vec![default_timer],
                positions: BTreeMap::new(),
                show_hotkey_labels: true,
            },
        );

        Self {
            active_profile: "Default".to_string(),
            profiles,
            ..Default::default()
        }
    }

    pub fn normalize(mut self, default_icon: Option<&str>) -> Self {
        if self.profiles.is_empty() {
            return Self::with_default_icon(default_icon);
        }

        for profile in self.profiles.values_mut() {
            profile.timers = profile
                .timers
                .iter()
                .take(MAX_TIMERS)
                .cloned()
                .map(|timer| timer.normalize())
                .collect();

            if profile.timers.is_empty() {
                let mut timer = TimerConfig::default();
                if let Some(icon) = default_icon {
                    timer.icon = icon.to_string();
                }
                profile.timers.push(timer);
            }
        }

        if !self.profiles.contains_key(&self.active_profile) {
            if let Some(first) = self.profiles.keys().next().cloned() {
                self.active_profile = first;
            }
        }

        self
    }
}

impl TimerConfig {
    pub fn normalize(mut self) -> Self {
        self.hotkey = self.hotkey.trim().to_lowercase();
        self.hotkey2 = self.hotkey2.trim().to_lowercase();
        self.duration = self.duration.clamp(1, 36_000);
        self.size = self.size.clamp(40, 250);
        self.opacity = self.opacity.clamp(15, 100);
        self.blink_threshold = self.blink_threshold.clamp(1, 60);
        if self.blink_color.trim().is_empty() {
            self.blink_color = DEFAULT_BLINK_COLOR.to_string();
        }
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconAsset {
    pub file_name: String,
    pub label: String,
    pub file_path: String,
    pub asset_url: Option<String>,
}

// ── Overlay window config (returned to overlay.ts on load) ───────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTimerConfig {
    pub timer_index: usize,
    pub icon_path: String,
    pub duration: u16,
    pub blink_threshold: u8,
    pub blink_color: String,
    pub blink: bool,
    pub size: u16,
    pub opacity: u8,
    pub hotkey: String,
    pub hotkey2: String,
    /// Whether to draw the hotkey label on the overlay canvas.
    pub show_hotkey_labels: bool,
    /// Pixel dimensions of the overlay window (square).
    pub win_size: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayCapabilities {
    pub global_hotkeys: bool,
    pub transparent_overlay: bool,
    pub click_through: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendAdapterInfo {
    pub name: String,
    pub stage: String,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    pub kind: String,
    pub arch: String,
    pub overlay: OverlayCapabilities,
    pub adapter: BackendAdapterInfo,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPayload {
    pub state: AppState,
    pub icons: Vec<IconAsset>,
    pub platform: PlatformInfo,
    pub storage_path: String,
    pub migrated_from: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub storage_path: String,
}
