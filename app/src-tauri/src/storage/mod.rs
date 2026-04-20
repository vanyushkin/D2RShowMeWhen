use std::{
    fs,
    path::{Path, PathBuf},
};

use tauri::{AppHandle, Manager};
use thiserror::Error;

use crate::core::models::{AppState, IconAsset};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct LoadedState {
    pub state: AppState,
    pub storage_path: PathBuf,
    pub migrated_from: Option<PathBuf>,
}

// ── Icons ─────────────────────────────────────────────────────────────────────

/// Load icons from bundled resources (production) or from app/assets/icons (dev).
///
/// Priority:
///   1. <resource_dir>/icons/  — set by `bundle.resources` in tauri.conf.json, present in all builds
///   2. <workspace_root>/app/assets/icons/  — dev-only fallback, requires the source tree
pub fn icons_from_resources(app: &AppHandle) -> Vec<IconAsset> {
    let icon_dir = resolve_icon_dir(app);
    collect_icons(&icon_dir)
}

/// Return the absolute path to a single icon file by filename.
/// Returns an empty string if the file cannot be found.
pub fn icon_file_path(app: &AppHandle, file_name: &str) -> String {
    if file_name.is_empty() {
        return String::new();
    }
    let dir = resolve_icon_dir(app);
    let path = dir.join(file_name);
    if path.exists() {
        path.to_string_lossy().to_string()
    } else {
        String::new()
    }
}

pub fn resolve_icon_dir(app: &AppHandle) -> PathBuf {
    // In a production bundle the icons are at <resource_dir>/icons/
    if let Ok(res_dir) = app.path().resource_dir() {
        let bundled = res_dir.join("icons");
        if bundled.exists() {
            return bundled;
        }
    }

    // Dev fallback: source tree app/assets/icons
    dev_icons_dir(app)
}

fn dev_icons_dir(app: &AppHandle) -> PathBuf {
    // Walk up from resource_dir looking for app/assets/icons in the source tree.
    if let Ok(res_dir) = app.path().resource_dir() {
        for ancestor in res_dir.ancestors() {
            let candidate = ancestor.join("app").join("assets").join("icons");
            if candidate.exists() {
                return candidate;
            }
        }
    }

    // Last resort: look next to the running binary.
    // (Works for cargo run in-tree without a proper resource_dir.)
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("assets")
        .join("icons")
}

fn collect_icons(icon_dir: &Path) -> Vec<IconAsset> {
    let mut assets = Vec::new();

    if let Ok(entries) = fs::read_dir(icon_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("png") {
                continue;
            }

            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let label = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .replace('_', " ");

            assets.push(IconAsset {
                file_name,
                label,
                file_path: path.to_string_lossy().to_string(),
                asset_url: None,
            });
        }
    }

    assets.sort_by(|a, b| a.label.cmp(&b.label));
    assets
}

// ── State load / save ─────────────────────────────────────────────────────────

pub fn load_state(app: &AppHandle, default_icon: Option<&str>) -> Result<LoadedState, StorageError> {
    let target = storage_path(app)?;

    if let Some(state) = read_state_from_file(&target, default_icon)? {
        return Ok(LoadedState { state, storage_path: target, migrated_from: None });
    }

    if let Some(migration_source) = first_existing_migration_source() {
        if let Some(state) = read_state_from_file(&migration_source, default_icon)? {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&migration_source, &target)?;
            return Ok(LoadedState {
                state,
                storage_path: target,
                migrated_from: Some(migration_source),
            });
        }
    }

    Ok(LoadedState {
        state: AppState::with_default_icon(default_icon),
        storage_path: target,
        migrated_from: None,
    })
}

pub fn save_state(app: &AppHandle, state: &AppState) -> Result<PathBuf, StorageError> {
    let target = storage_path(app)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, serde_json::to_string_pretty(state)?)?;
    Ok(target)
}

// ── Internals ─────────────────────────────────────────────────────────────────

fn read_state_from_file(
    path: &Path,
    default_icon: Option<&str>,
) -> Result<Option<AppState>, StorageError> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    let parsed = serde_json::from_str::<AppState>(&content)?;
    Ok(Some(parsed.normalize(default_icon)))
}

/// Canonical profile storage path.
///
/// Platform defaults via Tauri:
///   macOS  → ~/Library/Application Support/com.vanyushkin.d2rshowmewhen/profiles.json
///   Windows → %APPDATA%\com.vanyushkin.d2rshowmewhen\profiles.json
///   Linux   → ~/.local/share/com.vanyushkin.d2rshowmewhen/profiles.json
fn storage_path(app: &AppHandle) -> Result<PathBuf, StorageError> {
    let app_data = app
        .path()
        .app_data_dir()
        // Second choice: platform data dir / app name
        .ok()
        .or_else(|| dirs::data_local_dir().map(|d| d.join("D2RShowMeWhen")))
        // Last resort: next to the binary
        .unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".d2rsmw")
        });

    fs::create_dir_all(&app_data)?;
    Ok(app_data.join("profiles.json"))
}

/// Try to migrate from a legacy state file (macOS Python app only).
fn first_existing_migration_source() -> Option<PathBuf> {
    migration_candidates().into_iter().find(|p| p.exists())
}

fn migration_candidates() -> Vec<PathBuf> {
    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }

    #[cfg(target_os = "macos")]
    {
        let mut paths = Vec::new();

        if let Some(home) = dirs::home_dir() {
            paths.push(
                home.join("Library")
                    .join("Application Support")
                    .join("D2R_Show_Me_When_Mac")
                    .join("profiles.json"),
            );
        }

        paths
    }
}
