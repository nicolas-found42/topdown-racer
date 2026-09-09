//! Menu screen: start button UI and keyboard shortcuts.

use bevy::prelude::*;

use crate::best_lap::{format_best_target, SavedBestLap};
use crate::overlay::spawn_overlay_root;

/// Marker for the menu screen root entity.
#[derive(Component)]
pub struct MenuUi;

/// Marker for the menu start button.
#[derive(Component)]
pub struct StartButton;

/// Spawns the menu screen with a start option and the saved best-lap target.
pub fn spawn_menu_ui(mut commands: Commands, saved: Res<SavedBestLap>, shell: Res<crate::ShellSimulation>) {
    let target_line = format_best_target(saved.0);
    spawn_overlay_root(&mut commands, 16.0, 0.92)
        .insert(MenuUi)
        .with_children(|menu| {
            menu.spawn(TextBundle::from_section(
                "TOPDOWN RACER",
                TextStyle {
                    font_size: 48.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
            menu.spawn(TextBundle::from_section(
                target_line,
                TextStyle {
                    font_size: 22.0,
                    color: Color::srgb(1.0, 0.85, 0.3),
                    ..default()
                },
            ));
            menu.spawn((TextBundle::from_section(driving_summary(&shell), TextStyle {
                font_size: 20.0, color: Color::srgb(1.0, 0.85, 0.3), ..default()
            }), DrivingSummary));
            menu.spawn(TextBundle::from_section(CONTROL_HELP, TextStyle {
                font_size: 18.0, color: Color::WHITE, ..default()
            }));
            for (i, pace) in [OpponentPace::Touring, OpponentPace::Club, OpponentPace::Race].into_iter().enumerate() {
                menu.spawn((ButtonBundle { style: Style { padding: UiRect::all(Val::Px(8.0)), ..default() },
                    background_color: BackgroundColor(Color::srgb(0.14, 0.2, 0.25)), ..default() }, PaceButton(pace)))
                    .with_children(|button| { button.spawn(TextBundle::from_section(
                        format!("{}: {} - {}", i + 1, pace.label(), pace.description()),
                        TextStyle { font_size: 18.0, color: Color::WHITE, ..default() })); });
            }
            menu.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect::axes(Val::Px(32.0), Val::Px(12.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.2, 0.55, 0.25)),
                    ..default()
                },
                StartButton,
            ))
            .with_children(|button| {
                button.spawn(TextBundle::from_section(
                    "START RACE (Enter)",
                    TextStyle {
                        font_size: 24.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
            });
        });
}

/// Shared visible identity and selected ownership on every screen.
#[derive(Component)]
pub(crate) struct DrivingSummary;

pub(crate) const CONTROL_HELP: &str = "T: Manual / Autopilot (release controls after switching)\nW / Up: throttle    S / Down: brake, hold to reverse\nA D / Left Right: steer    Space: handbrake\nEsc: menu";

pub(crate) fn driving_summary(shell: &crate::ShellSimulation) -> String {
    let assisted = shell.curr_snapshots.first().is_some_and(|snap| snap.current_lap_assisted);
    format!("YOU: BLUE CAR #1  |  {}{}\nOPPONENT PACE: {}", shell.sim.player_mode().label(),
        if assisted { "  |  LAP ASSISTED" } else { "" }, shell.selected_pace.label())
}

pub(crate) fn update_driving_summary(shell: Res<crate::ShellSimulation>, mut texts: Query<&mut Text, With<DrivingSummary>>) {
    for mut text in &mut texts { text.sections[0].value = driving_summary(&shell); }
}

use topdown_racer_core::ai::OpponentPace;

#[derive(Component)]
pub(crate) struct PaceButton(OpponentPace);

pub(crate) fn select_pace_system(keyboard: Res<ButtonInput<KeyCode>>, buttons: Query<(&Interaction, &PaceButton), Changed<Interaction>>, mut shell: ResMut<crate::ShellSimulation>) {
    for (key, pace) in [(KeyCode::Digit1, OpponentPace::Touring), (KeyCode::Digit2, OpponentPace::Club), (KeyCode::Digit3, OpponentPace::Race)] {
        if keyboard.just_pressed(key) { shell.selected_pace = pace; }
    }
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed { shell.selected_pace = button.0; }
    }
}
