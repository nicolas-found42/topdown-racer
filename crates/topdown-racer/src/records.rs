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
    /// Where the book is written. Defaults to the platform config path;
    /// tests inject a temp path so they never touch the user's config dir.
    pub path: std::path::PathBuf,
}

impl LocalRecords {
    /// Loads the book stored at `path`, remembering it for later saves.
    pub fn from_path(path: std::path::PathBuf) -> Self {
        Self {
            book: RecordBook::load(&path),
            notice: String::new(),
            path,
        }
    }
}

impl Default for LocalRecords {
    fn default() -> Self {
        Self::from_path(record_path())
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
                records.notice = match records.book.save(&records.path) {
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
            records.notice = match records.book.save(&records.path) {
                Ok(()) => String::new(),
                Err(_) => "Record kept for this session; local save failed".to_owned(),
            };
        }
    }
    saved.0 = records.book.best(&key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SavedBestLap;
    use topdown_racer_core::track::{Track, SAMPLE_CIRCUIT};

    fn temp_record_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("racer-records-{tag}-{}.json", std::process::id()))
    }

    /// A shell whose live snapshot carries an eligible manual best, as the sim
    /// would produce at the tick an eligible flying lap completes.
    fn shell_with_manual_best(best: Option<f32>) -> crate::ShellSimulation {
        let mut shell = crate::ShellSimulation::new(Track::parse(SAMPLE_CIRCUIT).unwrap());
        let mut snapshot = shell.sim.snapshots()[0];
        snapshot.best_manual_lap_time = best;
        shell.curr_snapshots = vec![snapshot];
        shell
    }

    #[test]
    fn an_eligible_manual_best_reaches_the_record_file_at_the_lap_completion_tick() {
        let path = temp_record_path("eligible-manual");
        let _ = std::fs::remove_file(&path);
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();

        let mut app = App::new();
        app.insert_resource(LocalRecords::from_path(path.clone()))
            .insert_resource(shell_with_manual_best(Some(28.5)))
            .init_resource::<SavedBestLap>()
            .add_systems(Update, persist_completed_laps);
        app.update();

        let key = race_key(&track, crate::controls::SteeringResponse::Raw);
        assert_eq!(
            RecordBook::load(&path).best(&key),
            Some(28.5),
            "an eligible manual best must be written to the records file"
        );
        assert_eq!(
            app.world().resource::<SavedBestLap>().0,
            Some(28.5),
            "the menu target must see the written best"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn an_assisted_race_never_writes_a_record_file() {
        let path = temp_record_path("assisted");
        let _ = std::fs::remove_file(&path);
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();

        let mut app = App::new();
        app.insert_resource(LocalRecords::from_path(path.clone()))
            .insert_resource(shell_with_manual_best(None))
            .init_resource::<SavedBestLap>()
            .add_systems(Update, persist_completed_laps);
        app.update();

        let key = race_key(&track, crate::controls::SteeringResponse::Raw);
        assert_eq!(
            RecordBook::load(&path).best(&key),
            None,
            "an assisted race must never create or update a record"
        );
        assert_eq!(
            app.world().resource::<SavedBestLap>().0,
            None,
            "an assisted race must not publish a menu target"
        );
    }

    #[test]
    fn failed_save_keeps_the_menu_target_after_restarting_the_race() {
        let root = temp_record_path("notice");
        std::fs::create_dir_all(&root).unwrap();
        // A regular file cannot contain the requested records file. This fails
        // reliably without relying on platform-specific permission semantics.
        let blocker = root.join("not-a-directory");
        std::fs::write(&blocker, "occupied").unwrap();
        let path = blocker.join("records-v2.json");
        let mut app = App::new();
        app.insert_resource(LocalRecords::from_path(path))
            .insert_resource(shell_with_manual_best(Some(28.5)))
            .init_resource::<SavedBestLap>()
            .add_systems(Update, persist_completed_laps);
        app.update();

        assert_eq!(app.world().resource::<SavedBestLap>().0, Some(28.5));
        assert!(!app.world().resource::<LocalRecords>().notice.is_empty());

        // Restart has no completed laps. The session best and failure notice
        // must survive it, even though nothing could be persisted to disk.
        app.insert_resource(crate::ShellSimulation::new(
            Track::parse(SAMPLE_CIRCUIT).unwrap(),
        ));
        app.update();
        assert_eq!(app.world().resource::<SavedBestLap>().0, Some(28.5));
        assert!(!app.world().resource::<LocalRecords>().notice.is_empty());
        assert_eq!(std::fs::read_to_string(&blocker).unwrap(), "occupied");
        std::fs::remove_file(blocker).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
