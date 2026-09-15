//! Pause/focus orchestration. Placement and record eligibility remain in Sim.
use crate::{controls::SteeringFilter, AppState, ControlGate, PlayerInput, ShellSimulation};
use bevy::{prelude::*, window::WindowFocused};

pub struct PausePlugin;
#[derive(Component)]
struct PauseOverlay;
#[derive(Component)]
struct PauseMessage;
#[derive(Resource, Default)]
struct PauseNotice(String);
#[derive(Component, Clone, Copy)]
enum Action {
    Resume,
    Restart,
    Menu,
    Recover,
}

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<WindowFocused>()
            .init_resource::<PauseNotice>()
            .add_systems(
                PreUpdate,
                pause_actions
                    .after(bevy::input::InputSystem)
                    .before(crate::toggle_autopilot_system)
                    .before(crate::read_keyboard_input)
                    .run_if(in_state(AppState::Race)),
            )
            .add_systems(Update, pause_overlay.run_if(in_state(AppState::Race)))
            .add_systems(OnExit(AppState::Race), clear_overlay);
    }
}

fn pause_actions(
    mut focused: EventReader<WindowFocused>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
    mut shell: ResMut<ShellSimulation>,
    (mut input, mut gate, mut steering): (
        ResMut<PlayerInput>,
        ResMut<ControlGate>,
        ResMut<SteeringFilter>,
    ),
    mut next: ResMut<NextState<AppState>>,
    mut notice: ResMut<PauseNotice>,
) {
    let focus_lost = focused.read().any(|event| !event.focused);
    if focus_lost || keys.just_pressed(KeyCode::Escape) {
        shell.sim.pause();
        input.0 = Default::default();
        gate.0 = true;
        steering.reset();
        keys.reset_all();
        return;
    }
    if !shell.sim.is_paused() {
        return;
    }
    let action = buttons
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Pressed)
        .map(|(_, action)| *action)
        .or_else(|| {
            if keys.just_pressed(KeyCode::Enter) {
                Some(Action::Resume)
            } else if keys.just_pressed(KeyCode::KeyR) {
                Some(Action::Restart)
            } else if keys.just_pressed(KeyCode::KeyM) {
                Some(Action::Menu)
            } else if keys.just_pressed(KeyCode::KeyC) {
                Some(Action::Recover)
            } else {
                None
            }
        });
    let Some(action) = action else {
        return;
    };
    match action {
        Action::Resume => {
            shell.sim.resume();
            notice.0.clear();
        }
        Action::Restart => {
            shell.reset_to_fresh_race();
            notice.0.clear();
        }
        Action::Menu => next.set(AppState::Menu),
        Action::Recover => match shell.sim.recover_player() {
            Ok(()) => {
                shell.curr_snapshots = shell.sim.snapshots();
                shell.prev_snapshots = shell.curr_snapshots.clone();
                shell.generation += 1;
                notice.0 = "Car recovered. This lap cannot set a record. Resume when ready.".into();
            }
            Err(error) => notice.0 = error.message().into(),
        },
    }
    input.0 = Default::default();
    gate.0 = true;
    steering.reset();
    keys.reset_all();
}

fn pause_overlay(
    mut commands: Commands,
    shell: Res<ShellSimulation>,
    notice: Res<PauseNotice>,
    roots: Query<Entity, With<PauseOverlay>>,
    mut messages: Query<&mut Text, With<PauseMessage>>,
) {
    if !shell.sim.is_paused() {
        for root in &roots {
            commands.entity(root).despawn_recursive();
        }
        return;
    }
    if roots.is_empty() {
        crate::overlay::spawn_overlay_root(&mut commands, 12.0, 0.94)
            .insert(PauseOverlay)
            .with_children(|root| {
                root.spawn(TextBundle::from_section(
                    "PAUSED",
                    TextStyle {
                        font_size: 38.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
                root.spawn((
                    TextBundle::from_section(
                        "",
                        TextStyle {
                            font_size: 18.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    PauseMessage,
                ));
                for (action, label) in [
                    (Action::Resume, "RESUME (Enter)"),
                    (Action::Restart, "RESTART (R)"),
                    (
                        Action::Recover,
                        "RECOVER CAR (C) - below 1 u/s; lap becomes ineligible",
                    ),
                    (Action::Menu, "RETURN TO MENU (M)"),
                ] {
                    root.spawn((
                        ButtonBundle {
                            style: Style {
                                padding: UiRect::all(Val::Px(10.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgb(0.15, 0.2, 0.25)),
                            ..default()
                        },
                        action,
                    ))
                    .with_children(|button| {
                        button.spawn(TextBundle::from_section(
                            label,
                            TextStyle {
                                font_size: 20.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        ));
                    });
                }
            });
    }
    for mut text in &mut messages {
        text.sections[0].value = notice.0.clone();
    }
}

fn clear_overlay(mut commands: Commands, roots: Query<Entity, With<PauseOverlay>>) {
    for root in &roots {
        commands.entity(root).despawn_recursive();
    }
}
