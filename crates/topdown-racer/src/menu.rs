//! Menu screen: start button, keyboard shortcuts, and race-state transitions.

use bevy::prelude::*;

use crate::best_lap::{format_best_target, SavedBestLap};
use crate::overlay::spawn_overlay_root;
use crate::{AppState, ShellSimulation};

/// Marker for the menu screen root entity.
#[derive(Component)]
pub struct MenuUi;

/// Marker for the menu start button.
#[derive(Component)]
pub struct StartButton;

/// Rebuilds a fresh countdown race whenever entering the Race state.
pub fn reset_race_on_enter(mut shell: ResMut<ShellSimulation>) {
    shell.reset_to_fresh_race();
}

/// Spawns the menu screen with a start option and the saved best-lap target.
pub fn spawn_menu_ui(mut commands: Commands, saved: Res<SavedBestLap>) {
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

/// Starts the race from the menu via the button or the Enter key.
pub fn menu_action_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    button_q: Query<&Interaction, (Changed<Interaction>, With<StartButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let button_clicked = button_q
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    if button_clicked || keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(AppState::Race);
    }
}

/// Returns cleanly to the menu when ESC is pressed during a race.
pub fn esc_to_menu_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Menu);
    }
}
