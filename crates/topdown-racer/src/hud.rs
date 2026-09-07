//! HUD projection: formatted race data, HUD/overlay UI, and countdown display.

use bevy::prelude::*;
use topdown_racer_core::simulation::{CarSnapshot, RacePhase, DEFAULT_COUNTDOWN_TICKS, FIXED_DT};

use crate::results::{format_opt_lap_time, ordinal};
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

/// Formats a time duration in seconds into `MM:SS.hh`.
pub fn format_time(seconds: f32) -> String {
    let total_hundredths = (seconds.max(0.0) * 100.0).round() as u32;
    let hundredths = total_hundredths % 100;
    let total_seconds = total_hundredths / 100;
    let secs = total_seconds % 60;
    let mins = total_seconds / 60;
    format!("{:02}:{:02}.{:02}", mins, secs, hundredths)
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
        RacePhase::Racing if shell.racing_ticks as f32 * FIXED_DT > 2.0 => "",
        phase => countdown_display(phase),
    };
    for mut t in countdown_q.iter_mut() {
        t.sections[0].value = text.to_owned();
    }
}
