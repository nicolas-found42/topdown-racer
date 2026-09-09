//! Results projection: ordered results table and the results screen UI.

use bevy::prelude::*;
use topdown_racer_core::simulation::{CarSnapshot, RacePhase};

use crate::fmt::{format_opt_lap_time, ordinal};
use crate::overlay::spawn_overlay_root;
use crate::ShellSimulation;

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

/// Marker for the results screen root entity.
#[derive(Component)]
pub struct ResultsUi;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::sample_snapshot;
    use topdown_racer_core::simulation::{CarInput, Sim};
    use topdown_racer_core::track::{Track, SAMPLE_CIRCUIT};

    fn results_snapshot(
        position: usize,
        lap_times: [Option<f32>; 3],
        best_lap_time: Option<f32>,
    ) -> CarSnapshot {
        let mut snap = sample_snapshot();
        snap.position = position;
        snap.completed_laps = 3;
        snap.lap_times = lap_times;
        snap.best_lap_time = best_lap_time;
        snap.phase = topdown_racer_core::simulation::RacePhase::Finished;
        snap
    }

    #[test]
    fn results_show_correct_finishing_order_and_per_car_lap_times() {
        // Snapshots arrive in car order; positions come from the simulation.
        let snaps = vec![
            results_snapshot(2, [Some(20.5), Some(19.5), Some(21.0)], Some(19.5)),
            results_snapshot(1, [Some(18.0), Some(18.5), Some(17.5)], Some(17.5)),
            results_snapshot(4, [Some(25.0), Some(24.0), Some(26.0)], Some(24.0)),
            results_snapshot(3, [Some(22.0), Some(21.0), Some(23.0)], Some(21.0)),
        ];

        let rows = format_results(&snaps);
        assert_eq!(rows.len(), 4);
        let car_order: Vec<usize> = rows.iter().map(|r| r.car_number).collect();
        assert_eq!(car_order, vec![2, 1, 4, 3]);
        assert_eq!(rows[0].position, 1);
        assert_eq!(rows[0].lap_times, vec!["00:18.00", "00:18.50", "00:17.50"]);
        assert_eq!(rows[0].best_lap, "00:17.50");
        assert_eq!(rows[3].position, 4);
        assert_eq!(rows[3].best_lap, "00:24.00");
    }

    #[test]
    fn race_cannot_finish_before_three_laps_and_results_appear_exactly_at_finish() {
        use topdown_racer_core::simulation::RacePhase;
        assert!(!should_show_results(RacePhase::Countdown {
            ticks_remaining: 10
        }));
        assert!(!should_show_results(RacePhase::Racing));
        assert!(should_show_results(RacePhase::Finished));

        // A sim that completed a single lap is still racing, never finished.
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut sim = Sim::new(track.clone(), 1);
        let waypoints = &track.points[..track.points.len() - 1];
        let mut current_wp = 1;
        let mut last_pose = sim.tick(&[CarInput::default()])[0].pose;
        let mut last_heading = 0.0f32;
        let mut saw_lap_one_racing = false;
        for _ in 0..1800 {
            let target = waypoints[current_wp];
            let to_target = target - last_pose;
            let target_angle = to_target.y.atan2(to_target.x);
            let angle_diff =
                topdown_racer_core::simulation::wrap_angle(target_angle - last_heading);
            let snap = sim.tick(&[CarInput {
                throttle: if angle_diff.abs() > 0.4 { 0.5 } else { 1.0 },
                brake: 0.0,
                steer: (angle_diff * 2.5).clamp(-1.0, 1.0),
                handbrake: false,
            }])[0];
            last_pose = snap.pose;
            last_heading = snap.heading;
            if snap.completed_laps == 1 {
                assert_eq!(snap.phase, RacePhase::Racing);
                assert!(!should_show_results(snap.phase));
                saw_lap_one_racing = true;
                break;
            }
            if (target - snap.pose).length() < 14.0 {
                current_wp = (current_wp + 1) % waypoints.len();
            }
        }
        assert!(
            saw_lap_one_racing,
            "must complete one lap while still racing"
        );
    }
}
