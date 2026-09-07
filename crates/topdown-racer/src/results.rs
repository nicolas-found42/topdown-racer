//! Results projection: ordered results table, finish detection, and the
//! results screen UI.

use bevy::prelude::*;
use topdown_racer_core::simulation::{CarSnapshot, RacePhase};

use crate::hud::format_time;
use crate::overlay::spawn_overlay_root;
use crate::{AppState, ShellSimulation};

/// Returns true only when the race phase warrants showing the results screen.
pub fn should_show_results(phase: RacePhase) -> bool {
    phase == RacePhase::Finished
}

/// One row of the results table: finishing position, car number, and lap times.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultRow {
    pub position: usize,
    pub car_number: usize,
    pub lap_times: Vec<String>,
    pub best_lap: String,
}

/// Pure projection of race snapshots to an ordered results table.
/// Holds zero local game rules: ordering reuses the simulation's live positions.
pub fn format_results(snaps: &[CarSnapshot]) -> Vec<ResultRow> {
    let mut ordered: Vec<(usize, &CarSnapshot)> = snaps.iter().enumerate().collect();
    ordered.sort_by_key(|(_, snap)| snap.position);
    ordered
        .into_iter()
        .map(|(car_index, snap)| {
            let lap_times = snap
                .lap_times
                .iter()
                .map(|t| format_opt_lap_time(*t))
                .collect();
            let best_lap = format_opt_lap_time(snap.best_lap_time);
            ResultRow {
                position: snap.position,
                car_number: car_index + 1,
                lap_times,
                best_lap,
            }
        })
        .collect()
}

/// Formats an optional lap time, showing a placeholder when no lap is recorded.
pub fn format_opt_lap_time(seconds: Option<f32>) -> String {
    match seconds {
        Some(secs) => format_time(secs),
        None => "--:--.--".to_owned(),
    }
}

/// Formats a 1-indexed position with its ordinal suffix.
pub(crate) fn ordinal(position: usize) -> String {
    match position {
        1 => "1st".to_owned(),
        2 => "2nd".to_owned(),
        3 => "3rd".to_owned(),
        n => format!("{n}th"),
    }
}

/// Marker for the results screen root entity.
#[derive(Component)]
pub struct ResultsUi;

/// Transitions to the results screen exactly when the race finishes.
pub fn detect_race_finish(
    shell: Res<ShellSimulation>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if should_show_results(shell.sim.phase()) {
        next_state.set(AppState::Results);
    }
}

/// Spawns the results screen from simulation snapshots.
pub fn spawn_results_ui(mut commands: Commands, shell: Res<ShellSimulation>) {
    let rows = format_results(&shell.curr_snapshots);
    spawn_overlay_root(&mut commands, 8.0, 0.94)
        .insert(ResultsUi)
        .with_children(|results| {
            results.spawn(TextBundle::from_section(
                "RACE FINISHED",
                TextStyle {
                    font_size: 40.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
            for row in &rows {
                let laps = row.lap_times.join("  ");
                results.spawn(TextBundle::from_section(
                    format!(
                        "{} — Car {} — {} — Best {}",
                        ordinal(row.position),
                        row.car_number,
                        laps,
                        row.best_lap
                    ),
                    TextStyle {
                        font_size: 20.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
            }
            results.spawn(TextBundle::from_section(
                "Press R to restart, ESC for menu",
                TextStyle {
                    font_size: 20.0,
                    color: Color::srgb(0.7, 0.9, 0.7),
                    ..default()
                },
            ));
        });
}

/// Instant-restarts a fresh race (R / Enter) or returns to the menu (ESC).
pub fn results_action_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) || keyboard.just_pressed(KeyCode::Enter) {
        // OnEnter(Race) rebuilds the fresh countdown race; no menu pass-through.
        next_state.set(AppState::Race);
    } else if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Menu);
    }
}
