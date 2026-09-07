//! HUD projection: formatted race data, HUD/overlay UI, and countdown display.

use bevy::prelude::*;
use topdown_racer_core::simulation::{CarSnapshot, RacePhase, DEFAULT_COUNTDOWN_TICKS, FIXED_DT};

use crate::fmt::{format_opt_lap_time, format_time, ordinal};
use crate::ShellSimulation;

/// Formatted HUD strings derived from simulation snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudData {
    pub lap: String,
    pub position: String,
    pub current_lap_time: String,
    pub best_lap_time: String,
    pub speed: String,
}

/// Pure projection function mapping a player snapshot to HUD display data.
/// Holds zero local game rules or race state logic.
pub fn format_hud_data(player_snap: &CarSnapshot, total_cars: usize) -> HudData {
    use topdown_racer_core::simulation::TOTAL_LAPS;
    let current_lap = (player_snap.completed_laps + 1).min(TOTAL_LAPS);
    let lap = format!("LAP {}/{}", current_lap, TOTAL_LAPS);

    let position = format!("POS {}/{}", ordinal(player_snap.position), total_cars);

    let current_lap_time = format!("LAP TIME {}", format_time(player_snap.current_lap_time));

    let best_lap_time = format!("BEST {}", format_opt_lap_time(player_snap.best_lap_time));

    let speed_val = (player_snap.forward_speed.max(0.0).round()) as u32;
    let speed = format!("SPEED {} u/s", speed_val);

    HudData {
        lap,
        position,
        current_lap_time,
        best_lap_time,
        speed,
    }
}

/// Component tagging which HUD field a text element displays.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudElement {
    Lap,
    Position,
    CurrentTime,
    BestTime,
    Speed,
}

/// Spawns the HUD root and the centered countdown overlay text.
pub(crate) fn setup_hud(mut commands: Commands) {
    let text_style = TextStyle {
        font_size: 20.0,
        color: Color::WHITE,
        ..default()
    };

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::SpaceBetween,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(16.0)),
                    ..default()
                },
                visibility: Visibility::Hidden,
                ..default()
            },
            HudRoot,
        ))
        .with_children(|root| {
            // Top bar
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                ..default()
            })
            .with_children(|top_bar| {
                // Top-left
                top_bar
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|left| {
                        left.spawn((
                            TextBundle::from_section("LAP 1/3", text_style.clone()),
                            HudElement::Lap,
                        ));
                        left.spawn((
                            TextBundle::from_section("POS 1st/4", text_style.clone()),
                            HudElement::Position,
                        ));
                    });

                // Top-right
                top_bar
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexEnd,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|right| {
                        right.spawn((
                            TextBundle::from_section("LAP TIME 00:00.00", text_style.clone()),
                            HudElement::CurrentTime,
                        ));
                        right.spawn((
                            TextBundle::from_section("BEST --:--.--", text_style.clone()),
                            HudElement::BestTime,
                        ));
                    });
            });

            // Bottom bar
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                ..default()
            })
            .with_children(|bottom_bar| {
                bottom_bar.spawn((
                    TextBundle::from_section("SPEED 0 u/s", text_style.clone()),
                    HudElement::Speed,
                ));
            });
        });

    // Centered countdown overlay shown during the countdown phase.
    commands.spawn((
        TextBundle {
            text: Text::from_section(
                "3",
                TextStyle {
                    font_size: 96.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(40.0),
                ..default()
            },
            visibility: Visibility::Hidden,
            ..default()
        },
        CountdownText,
    ));
}

/// Updates HUD text from the player snapshot.
pub(crate) fn update_hud(
    shell: Res<ShellSimulation>,
    mut hud_query: Query<(&HudElement, &mut Text)>,
) {
    let Some(player_snap) = shell.curr_snapshots.first() else {
        return;
    };
    let hud = format_hud_data(player_snap, shell.curr_snapshots.len());

    for (element, mut text) in hud_query.iter_mut() {
        match element {
            HudElement::Lap => text.sections[0].value = hud.lap.clone(),
            HudElement::Position => text.sections[0].value = hud.position.clone(),
            HudElement::CurrentTime => text.sections[0].value = hud.current_lap_time.clone(),
            HudElement::BestTime => text.sections[0].value = hud.best_lap_time.clone(),
            HudElement::Speed => text.sections[0].value = hud.speed.clone(),
        }
    }
}

/// Marker for the HUD root node so it can hide behind the menu.
#[derive(Component)]
pub struct HudRoot;

/// Marker for the countdown overlay text shown during the countdown phase.
#[derive(Component)]
pub struct CountdownText;

/// Hides the HUD and countdown overlay while the menu is shown.
pub(crate) fn hide_hud(
    mut hud_q: Query<&mut Visibility, With<HudRoot>>,
    mut countdown_q: Query<&mut Visibility, (With<CountdownText>, Without<HudRoot>)>,
) {
    for mut vis in hud_q.iter_mut() {
        *vis = Visibility::Hidden;
    }
    for mut vis in countdown_q.iter_mut() {
        *vis = Visibility::Hidden;
    }
}

/// Shows the HUD and countdown overlay when a race starts.
pub(crate) fn show_hud(
    mut hud_q: Query<&mut Visibility, With<HudRoot>>,
    mut countdown_q: Query<&mut Visibility, (With<CountdownText>, Without<HudRoot>)>,
) {
    for mut vis in hud_q.iter_mut() {
        *vis = Visibility::Visible;
    }
    for mut vis in countdown_q.iter_mut() {
        *vis = Visibility::Visible;
    }
}

/// Maps a race phase to the countdown overlay text shown on screen.
pub fn countdown_display(phase: RacePhase) -> &'static str {
    match phase {
        RacePhase::Countdown { ticks_remaining } => {
            let elapsed = DEFAULT_COUNTDOWN_TICKS.saturating_sub(ticks_remaining);
            let third = DEFAULT_COUNTDOWN_TICKS / 3;
            if elapsed < third {
                "3"
            } else if elapsed < third * 2 {
                "2"
            } else {
                "1"
            }
        }
        RacePhase::Racing => "GO!",
        RacePhase::Finished => "",
    }
}

/// Updates the countdown overlay text from the simulation phase.
pub(crate) fn update_countdown_overlay(
    shell: Res<ShellSimulation>,
    mut countdown_q: Query<&mut Text, With<CountdownText>>,
) {
    let text = match shell.sim.phase() {
        // Clear the green flash once the race is underway (time since green,
        // not the per-lap timer which resets at every lap line).
        RacePhase::Racing if shell.sim.racing_ticks() as f32 * FIXED_DT > 2.0 => "",
        phase => countdown_display(phase),
    };
    for mut t in countdown_q.iter_mut() {
        t.sections[0].value = text.to_owned();
    }
}

/// A neutral CarSnapshot fixture shared by the hud and results test modules.
#[cfg(test)]
pub(crate) fn sample_snapshot() -> CarSnapshot {
    use topdown_racer_core::simulation::RacePhase;
    use topdown_racer_core::track::Surface;
    CarSnapshot {
        pose: Vec2::ZERO,
        heading: 0.0,
        velocity: Vec2::ZERO,
        forward_speed: 0.0,
        surface: Surface::Road,
        wall_contact: false,
        drifting: false,
        phase: RacePhase::Racing,
        completed_laps: 0,
        lap_times: [None; 3],
        current_lap_time: 0.0,
        best_lap_time: None,
        position: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use topdown_racer_core::simulation::{CarInput, Sim};
    use topdown_racer_core::track::{Track, SAMPLE_CIRCUIT};

    #[test]
    fn hud_shows_lap_position_time_best_and_speed_formatted_from_snapshot() {
        let mut snap = sample_snapshot();
        snap.velocity = Vec2::new(24.2, 0.0);
        snap.forward_speed = 24.2;
        snap.completed_laps = 1;
        snap.lap_times = [Some(18.45), None, None];
        snap.current_lap_time = 12.34;
        snap.best_lap_time = Some(18.45);

        let hud = format_hud_data(&snap, 4);
        assert_eq!(hud.lap, "LAP 2/3");
        assert_eq!(hud.position, "POS 1st/4");
        assert_eq!(hud.current_lap_time, "LAP TIME 00:12.34");
        assert_eq!(hud.best_lap_time, "BEST 00:18.45");
        assert_eq!(hud.speed, "SPEED 24 u/s");
    }

    #[test]
    fn hud_position_updates_when_cars_overtake_each_other() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();

        let mut sim = Sim::new(track, 4);
        sim.enable_ai_opponents();

        // Initially Car 0 (player) is at (0, 0) in 1st place
        let snaps0 = sim.tick(&[CarInput::default()]);
        assert_eq!(snaps0[0].position, 1);
        let hud0 = format_hud_data(&snaps0[0], 4);
        assert_eq!(hud0.position, "POS 1st/4");

        // AI Car 1 accelerates ahead while Car 0 stays still
        for _ in 0..100 {
            sim.tick(&[CarInput::default()]);
        }

        let snaps1 = sim.tick(&[CarInput::default()]);
        // AI Car 1 has overtaken Car 0, so Car 0 drops in position
        assert!(
            snaps1[0].position > 1,
            "player position must drop when overtaken"
        );
        let hud1 = format_hud_data(&snaps1[0], 4);
        assert_ne!(
            hud1.position, "POS 1st/4",
            "HUD position must update after being overtaken"
        );
    }

    #[test]
    fn hud_values_track_snapshot_exactly_without_local_shell_logic() {
        let mut snap = sample_snapshot();
        snap.position = 3;

        let hud_initial = format_hud_data(&snap, 4);
        assert_eq!(hud_initial.lap, "LAP 1/3");
        assert_eq!(hud_initial.position, "POS 3rd/4");
        assert_eq!(hud_initial.best_lap_time, "BEST --:--.--");
        assert_eq!(hud_initial.speed, "SPEED 0 u/s");

        // Mutate snapshot directly and ensure 1:1 reflection in HUD
        snap.completed_laps = 2;
        snap.position = 2;
        snap.current_lap_time = 65.25;
        snap.best_lap_time = Some(61.80);
        snap.forward_speed = 31.7;

        let hud_updated = format_hud_data(&snap, 4);
        assert_eq!(hud_updated.lap, "LAP 3/3");
        assert_eq!(hud_updated.position, "POS 2nd/4");
        assert_eq!(hud_updated.current_lap_time, "LAP TIME 01:05.25");
        assert_eq!(hud_updated.best_lap_time, "BEST 01:01.80");
        assert_eq!(hud_updated.speed, "SPEED 32 u/s");
    }
    #[test]
    fn countdown_display_shows_three_two_one_then_go() {
        use topdown_racer_core::simulation::{RacePhase, DEFAULT_COUNTDOWN_TICKS};
        assert_eq!(
            countdown_display(RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS
            }),
            "3"
        );
        assert_eq!(
            countdown_display(RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS * 2 / 3 + 1
            }),
            "3"
        );
        assert_eq!(
            countdown_display(RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS / 2
            }),
            "2"
        );
        assert_eq!(
            countdown_display(RacePhase::Countdown { ticks_remaining: 1 }),
            "1"
        );
        assert_eq!(countdown_display(RacePhase::Racing), "GO!");
    }
}
