//! Best-lap persistence: local file storage of the player's best lap time.

use bevy::prelude::*;

use crate::hud::format_time;
use crate::ShellSimulation;

/// Best lap loaded from local storage, shown on the menu as the target to beat.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct SavedBestLap(pub Option<f32>);

/// File name holding the persisted best lap inside the app config directory.
pub const BEST_LAP_FILE_NAME: &str = "best_lap.txt";

/// Platform config base directory for local-only storage. Falls back to the current
/// working directory (`"."`) when standard platform environment variables (HOME / APPDATA / XDG) are unset.
fn config_base_dir() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    }
    #[cfg(target_os = "macos")]
    {
        home_dir().join("Library/Application Support")
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(xdg)
        } else {
            home_dir().join(".config")
        }
    }
}

#[cfg(target_os = "windows")]
fn home_dir() -> std::path::PathBuf {
    std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

#[cfg(not(target_os = "windows"))]
fn home_dir() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

/// Local-only file path for the persisted best lap.
pub fn default_best_lap_path() -> std::path::PathBuf {
    let mut path = config_base_dir();
    path.push("topdown-racer");
    path.push(BEST_LAP_FILE_NAME);
    path
}

/// Loads the saved best lap in seconds. Returns `None` when no valid save exists.
pub fn load_best_lap(path: &std::path::Path) -> Option<f32> {
    let text = std::fs::read_to_string(path).ok()?;
    let secs: f32 = text.trim().parse().ok()?;
    if secs.is_finite() && secs > 0.0 {
        Some(secs)
    } else {
        None
    }
}

/// Writes `candidate` as the saved best lap only when it beats the stored value.
/// Returns the best lap now stored (the previous value when the candidate is slower).
pub fn maybe_save_best_lap(path: &std::path::Path, candidate: f32) -> std::io::Result<Option<f32>> {
    let current = load_best_lap(path);
    let better = match current {
        None => true,
        Some(best) => candidate < best,
    };
    if better {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, format!("{candidate}\n"))?;
        Ok(Some(candidate))
    } else {
        Ok(current)
    }
}

/// Formats the menu target line from the saved best lap.
pub fn format_best_target(saved: Option<f32>) -> String {
    match saved {
        Some(best) => format!("TARGET TO BEAT — BEST {}", format_time(best)),
        None => "TARGET TO BEAT — no best lap yet".to_owned(),
    }
}

/// Persists the player's best lap when a race finishes.
pub fn persist_best_lap_on_finish(shell: Res<ShellSimulation>, mut saved: ResMut<SavedBestLap>) {
    let Some(player_snap) = shell.curr_snapshots.first() else {
        return;
    };
    if let Some(best) = player_snap.best_lap_time {
        match maybe_save_best_lap(&default_best_lap_path(), best) {
            Ok(stored) => saved.0 = stored,
            Err(err) => warn!("failed to persist best lap: {err}"),
        }
    }
}
