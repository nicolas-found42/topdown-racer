//! Local records keyed by Track content, handling/control policy and discipline.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Write, path::Path};
use topdown_racer_core::track::Track;

#[derive(Default, Serialize, Deserialize)]
pub struct RecordBook {
    entries: BTreeMap<String, f32>,
}

impl RecordBook {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }
    pub fn best(&self, key: &str) -> Option<f32> {
        self.entries
            .get(key)
            .copied()
            .filter(|v| v.is_finite() && *v > 0.0)
    }
    pub fn consider(&mut self, key: &str, seconds: f32, eligible: bool) -> bool {
        if !eligible
            || !seconds.is_finite()
            || seconds <= 0.0
            || self.best(key).is_some_and(|v| v <= seconds)
        {
            return false;
        }
        self.entries.insert(key.to_owned(), seconds);
        true
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension(format!("{}.tmp", std::process::id()));
        let result = (|| {
            let mut file = std::fs::File::create(&temp)?;
            file.write_all(&serde_json::to_vec(self)?)?;
            file.sync_all()?;
            std::fs::rename(&temp, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(temp);
        }
        result
    }
}

/// Deterministic FNV-1a identity over the full typed Track content. Changes to
/// geometry, widths, surfaces or authored presentation invalidate comparisons.
pub fn track_identity(track: &Track) -> u64 {
    format!("{track:?}")
        .bytes()
        .fold(0xcbf29ce484222325, |hash, b| {
            (hash ^ b as u64).wrapping_mul(0x100000001b3)
        })
}

pub fn race_key(track: &Track, steering: crate::controls::SteeringResponse) -> String {
    format!(
        "race:{:016x}:handling1:manual-flying-gate2:{steering:?}",
        track_identity(track)
    )
}

#[derive(Resource)]
pub struct LocalRecords {
    pub book: RecordBook,
    pub notice: String,
}

impl Default for LocalRecords {
    fn default() -> Self {
        Self {
            book: RecordBook::load(&record_path()),
            notice: String::new(),
        }
    }
}

pub fn record_path() -> std::path::PathBuf {
    crate::default_best_lap_path().with_file_name("records-v2.json")
}

pub(crate) fn persist_completed_laps(
    shell: Res<crate::ShellSimulation>,
    mut records: ResMut<LocalRecords>,
    mut saved: ResMut<crate::SavedBestLap>,
) {
    if let Some(attempt) = &shell.practice {
        if let topdown_racer_core::practice::PracticeStatus::Finished { seconds, .. } =
            attempt.status()
        {
            let key = crate::practice::record_key(&shell);
            if records.book.consider(&key, seconds, true) {
                records.notice = match records.book.save(&record_path()) {
                    Ok(()) => String::new(),
                    Err(_) => "Practice record kept for this session; local save failed".into(),
                };
            }
        }
        return;
    }
    let key = race_key(shell.sim.track(), shell.steering_response);
    if let Some(best) = shell
        .curr_snapshots
        .first()
        .and_then(|car| car.best_manual_lap_time)
    {
        if records.book.consider(&key, best, true) {
            records.notice = match records.book.save(&record_path()) {
                Ok(()) => String::new(),
                Err(_) => "Record kept for this session; local save failed".to_owned(),
            };
        }
    }
    saved.0 = records.book.best(&key);
}
