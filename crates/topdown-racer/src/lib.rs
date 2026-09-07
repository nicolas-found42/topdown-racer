//! Shell library for Topdown Racer: camera, rendering, and simulation glue.

use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};
use glam::Vec2;
use topdown_racer_core::{
    simulation::{
        CarInput, CarSnapshot, RacePhase, Sim, DEFAULT_COUNTDOWN_TICKS, FIXED_DT, FIXED_HZ,
    },
    track::{Surface, Track, SAMPLE_CIRCUIT},
};

/// Canonical aspect ratio for the game view (16:9).
pub const TARGET_ASPECT_RATIO: f32 = 16.0 / 9.0;

/// Fixed camera zoom (orthographic scale). Smaller value = closer zoom.
pub const CAMERA_ZOOM: f32 = 0.08;

/// Rectangular viewport region for letterboxing/pillarboxing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Computes the pixel viewport rectangle to letterbox or pillarbox
/// a target aspect ratio inside a window of size `(window_width, window_height)`.
pub fn compute_letterbox_viewport(
    window_width: u32,
    window_height: u32,
    target_aspect: f32,
) -> ViewportRect {
    if window_width == 0 || window_height == 0 || target_aspect <= 0.0 {
        return ViewportRect {
            x: 0,
            y: 0,
            width: window_width,
            height: window_height,
        };
    }

    let current_aspect = window_width as f32 / window_height as f32;

    if current_aspect > target_aspect {
        // Window is wider than target: pillarbox (black bars left and right).
        let viewport_width = (window_height as f32 * target_aspect).round() as u32;
        let x = (window_width.saturating_sub(viewport_width)) / 2;
        ViewportRect {
            x,
            y: 0,
            width: viewport_width.min(window_width),
            height: window_height,
        }
    } else {
        // Window is taller than target: letterbox (black bars top and bottom).
        let viewport_height = (window_width as f32 / target_aspect).round() as u32;
        let y = (window_height.saturating_sub(viewport_height)) / 2;
        ViewportRect {
            x: 0,
            y,
            width: window_width,
            height: viewport_height.min(window_height),
        }
    }
}

/// Wraps an angle delta to the range $[-\pi, \pi]$ taking the shortest rotational path.
pub fn wrap_angle(mut delta: f32) -> f32 {
    while delta > std::f32::consts::PI {
        delta -= 2.0 * std::f32::consts::PI;
    }
    while delta < -std::f32::consts::PI {
        delta += 2.0 * std::f32::consts::PI;
    }
    delta
}

/// Smoothly interpolates an angle in radians between `prev` and `curr` taking
/// the shortest rotational path across the $[-\pi, \pi]$ boundary.
pub fn interpolate_heading(prev: f32, curr: f32, alpha: f32) -> f32 {
    prev + wrap_angle(curr - prev) * alpha
}

/// Interpolates 2D world position.
pub fn interpolate_pose(prev: Vec2, curr: Vec2, alpha: f32) -> Vec2 {
    prev.lerp(curr, alpha)
}

/// Component tagging the follow camera.
#[derive(Component)]
pub struct FollowCamera;

/// Component tagging a rendered Car chassis and identifying its car index.
#[derive(Component)]
pub struct CarVisual {
    pub car_index: usize,
}

/// Total cars in the race: 1 player car + 3 AI opponents.
pub const TOTAL_RACE_CARS: usize = 4;

/// Resource holding simulation state and consecutive snapshots for render interpolation.
#[derive(Resource)]
pub struct ShellSimulation {
    pub sim: Sim,
    pub prev_snapshots: Vec<CarSnapshot>,
    pub curr_snapshots: Vec<CarSnapshot>,
    /// Fixed ticks elapsed since the race phase began (green), for overlay timing.
    pub racing_ticks: u32,
}

impl ShellSimulation {
    pub fn new(track: Track) -> Self {
        Self::from_sim(Sim::new_race(track, TOTAL_RACE_CARS))
    }

    fn from_sim(mut sim: Sim) -> Self {
        let initial = sim.tick(&[CarInput::default()]);
        Self {
            sim,
            prev_snapshots: initial.clone(),
            curr_snapshots: initial,
            racing_ticks: 0,
        }
    }

    /// Rebuilds a fresh race starting from the countdown phase.
    pub fn reset_to_fresh_race(&mut self) {
        let track = self.sim.track().clone();
        *self = Self::from_sim(Sim::new_race(track, TOTAL_RACE_CARS));
    }
}

/// Shell-level screen state: menu, an active race, or the results screen.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Menu,
    Race,
    Results,
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

/// Main game plugin wiring camera, track rendering, simulation fixed step, and motion interpolation.
pub struct RacerGamePlugin;

impl Plugin for RacerGamePlugin {
    fn build(&self, app: &mut App) {
        let track = Track::parse(SAMPLE_CIRCUIT).expect("sample circuit must parse");
        app.init_state::<AppState>()
            .insert_resource(Time::<Fixed>::from_hz(FIXED_HZ as f64))
            .insert_resource(ShellSimulation::new(track))
            .insert_resource(SavedBestLap(load_best_lap(&default_best_lap_path())))
            .init_resource::<PlayerInput>()
            .add_systems(Startup, (setup_camera, setup_track, setup_car, setup_hud))
            .add_systems(OnEnter(AppState::Menu), (spawn_menu_ui, hide_hud))
            .add_systems(OnExit(AppState::Menu), despawn_screens)
            .add_systems(
                OnEnter(AppState::Results),
                (spawn_results_ui, hide_hud, persist_best_lap_on_finish),
            )
            .add_systems(OnExit(AppState::Results), despawn_screens)
            .add_systems(Update, menu_action_system.run_if(in_state(AppState::Menu)))
            .add_systems(Update, esc_to_menu_system.run_if(in_state(AppState::Race)))
            .add_systems(
                Update,
                results_action_system.run_if(in_state(AppState::Results)),
            )
            .add_systems(Update, detect_race_finish.run_if(in_state(AppState::Race)))
            .add_systems(
                PreUpdate,
                read_keyboard_input.run_if(in_state(AppState::Race)),
            )
            .add_systems(
                FixedUpdate,
                step_simulation.run_if(in_state(AppState::Race)),
            )
            .add_systems(
                Update,
                (
                    update_letterbox,
                    interpolate_car_and_camera.run_if(in_state(AppState::Race)),
                    update_hud.run_if(in_state(AppState::Race)),
                    update_countdown_overlay.run_if(in_state(AppState::Race)),
                ),
            );
    }
}
fn setup_camera(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.projection.scale = CAMERA_ZOOM;
    commands.spawn((camera, FollowCamera));
}

fn setup_track(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sim: Res<ShellSimulation>,
) {
    let track = sim.sim.track();
    let points = &track.points;
    let n = points.len() - 1;

    // Background grass field behind the track
    let grass_mesh = meshes.add(Rectangle::new(1400.0, 900.0));
    let grass_mat = materials.add(Color::srgb(0.12, 0.35, 0.12));
    commands.spawn(ColorMesh2dBundle {
        mesh: grass_mesh.into(),
        material: grass_mat,
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });

    let road_mat = materials.add(Color::srgb(0.22, 0.22, 0.26));
    let gravel_mat = materials.add(Color::srgb(0.48, 0.40, 0.28));
    let wall_mat = materials.add(Color::srgb(0.75, 0.25, 0.25));
    let start_mat = materials.add(Color::srgb(0.95, 0.95, 0.95));

    let road_half_w = track.road_half_width();
    let wall_dist = track.wall_distance();

    // Spawn quads for each segment
    for i in 0..n {
        let p0 = points[i];
        let p1 = points[i + 1];
        let dir = (p1 - p0).normalize_or_zero();
        let normal = Vec2::new(-dir.y, dir.x);

        let mat = match track.surfaces.get(i).copied().unwrap_or(Surface::Road) {
            Surface::Road => road_mat.clone(),
            Surface::Gravel => gravel_mat.clone(),
            Surface::Grass => continue,
        };

        // Road segment mesh
        let road_mesh = create_quad_mesh(
            p0 + normal * road_half_w,
            p0 - normal * road_half_w,
            p1 - normal * road_half_w,
            p1 + normal * road_half_w,
        );
        commands.spawn(ColorMesh2dBundle {
            mesh: meshes.add(road_mesh).into(),
            material: mat,
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        });

        // Boundary walls: left and right wall line segments
        for side in &[-1.0f32, 1.0f32] {
            let wall_w = 0.6;
            let offset = *side * wall_dist;
            let w_p0 = p0 + normal * offset;
            let w_p1 = p1 + normal * offset;
            let wall_quad = create_quad_mesh(
                w_p0 + normal * (wall_w * 0.5),
                w_p0 - normal * (wall_w * 0.5),
                w_p1 - normal * (wall_w * 0.5),
                w_p1 + normal * (wall_w * 0.5),
            );
            commands.spawn(ColorMesh2dBundle {
                mesh: meshes.add(wall_quad).into(),
                material: wall_mat.clone(),
                transform: Transform::from_xyz(0.0, 0.0, 2.0),
                ..default()
            });
        }
    }

    // Start/finish line across the track at points[0]
    if n > 0 {
        let dir = (points[1] - points[0]).normalize_or_zero();
        let normal = Vec2::new(-dir.y, dir.x);
        let sf_mesh = create_quad_mesh(
            points[0] + normal * road_half_w + dir * 0.5,
            points[0] - normal * road_half_w + dir * 0.5,
            points[0] - normal * road_half_w - dir * 0.5,
            points[0] + normal * road_half_w - dir * 0.5,
        );
        commands.spawn(ColorMesh2dBundle {
            mesh: meshes.add(sf_mesh).into(),
            material: start_mat,
            transform: Transform::from_xyz(0.0, 0.0, 3.0),
            ..default()
        });
    }
}

fn create_quad_mesh(tl: Vec2, tr: Vec2, br: Vec2, bl: Vec2) -> Mesh {
    use bevy::render::mesh::{Indices, PrimitiveTopology};
    let positions = vec![
        [tl.x, tl.y, 0.0],
        [tr.x, tr.y, 0.0],
        [br.x, br.y, 0.0],
        [bl.x, bl.y, 0.0],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(indices);
    mesh
}

fn setup_car(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sim: Res<ShellSimulation>,
) {
    let car_mesh = meshes.add(Rectangle::new(4.0, 2.0));
    let colors = [
        Color::srgb(0.92, 0.22, 0.22), // Player: Red
        Color::srgb(0.22, 0.45, 0.92), // AI 1: Blue
        Color::srgb(0.92, 0.82, 0.22), // AI 2: Yellow
        Color::srgb(0.72, 0.22, 0.92), // AI 3: Purple
    ];

    for (i, snap) in sim.curr_snapshots.iter().enumerate() {
        let mat = materials.add(colors[i % colors.len()]);
        commands.spawn((
            ColorMesh2dBundle {
                mesh: car_mesh.clone().into(),
                material: mat,
                transform: Transform::from_xyz(snap.pose.x, snap.pose.y, 10.0 + i as f32 * 0.1)
                    .with_rotation(Quat::from_rotation_z(snap.heading)),
                ..default()
            },
            CarVisual { car_index: i },
        ));
    }
}
/// Player input mapped from keyboard devices.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerInput(pub CarInput);

/// Pure function mapping keyboard boolean states to `CarInput`.
///
/// Controls:
/// - `up` (ArrowUp / W): throttle 1.0
/// - `down` (ArrowDown / S): brake 1.0 (held at standstill engages reverse)
/// - `left` (ArrowLeft / A): steer +1.0 (counter-clockwise)
/// - `right` (ArrowRight / D): steer -1.0 (clockwise)
/// - `handbrake` (Space): handbrake true
pub fn map_keyboard_input(
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    handbrake: bool,
) -> CarInput {
    let throttle = if up { 1.0 } else { 0.0 };
    let brake = if down { 1.0 } else { 0.0 };
    let mut steer = 0.0;
    if left {
        steer += 1.0;
    }
    if right {
        steer -= 1.0;
    }
    CarInput {
        throttle,
        brake,
        steer,
        handbrake,
    }
}

/// Reads keyboard input from Bevy device resources and updates `PlayerInput`.
/// The simulation never reads keyboard devices.
pub fn read_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_input: ResMut<PlayerInput>,
) {
    let up = keyboard.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]);
    let down = keyboard.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]);
    let left = keyboard.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]);
    let right = keyboard.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]);
    let handbrake = keyboard.pressed(KeyCode::Space);
    player_input.0 = map_keyboard_input(up, down, left, right, handbrake);
}

/// Fixed step system: advances the simulation at 64 Hz using mapped player inputs.
fn step_simulation(mut shell: ResMut<ShellSimulation>, player_input: Res<PlayerInput>) {
    shell.prev_snapshots = shell.curr_snapshots.clone();

    let snaps = shell.sim.tick(&[player_input.0]);
    shell.curr_snapshots = snaps;
    if shell.sim.phase() == RacePhase::Racing {
        shell.racing_ticks += 1;
    }
}
/// Updates camera viewport letterboxing when the window size changes.
fn update_letterbox(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Camera, With<FollowCamera>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok(mut camera) = cameras.get_single_mut() else {
        return;
    };

    let width = window.physical_width();
    let height = window.physical_height();
    let rect = compute_letterbox_viewport(width, height, TARGET_ASPECT_RATIO);

    camera.viewport = Some(Viewport {
        physical_position: UVec2::new(rect.x, rect.y),
        physical_size: UVec2::new(rect.width, rect.height),
        depth: 0.0..1.0,
    });
}

/// Interpolates Car visual transform and follow Camera between fixed steps.
fn interpolate_car_and_camera(
    shell: Res<ShellSimulation>,
    fixed_time: Res<Time<Fixed>>,
    mut cars: Query<(&CarVisual, &mut Transform), Without<FollowCamera>>,
    mut cameras: Query<&mut Transform, (With<FollowCamera>, Without<CarVisual>)>,
) {
    let alpha = fixed_time.overstep_fraction();

    for (visual, mut car_tf) in cars.iter_mut() {
        if let (Some(prev), Some(curr)) = (
            shell.prev_snapshots.get(visual.car_index),
            shell.curr_snapshots.get(visual.car_index),
        ) {
            let interp_pose = interpolate_pose(prev.pose, curr.pose, alpha);
            let interp_heading = interpolate_heading(prev.heading, curr.heading, alpha);

            car_tf.translation.x = interp_pose.x;
            car_tf.translation.y = interp_pose.y;
            car_tf.rotation = Quat::from_rotation_z(interp_heading);

            // Follow camera tracks player Car (index 0)
            if visual.car_index == 0 {
                for mut cam_tf in cameras.iter_mut() {
                    cam_tf.translation.x = interp_pose.x;
                    cam_tf.translation.y = interp_pose.y;
                    cam_tf.rotation = Quat::IDENTITY;
                }
            }
        }
    }
}

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

    let current_lap_time = format!("TIME {}", format_time(player_snap.current_lap_time));

    let best_lap_time = format!("BEST {}", format_opt_lap_time(player_snap.best_lap_time));

    let speed_val = (player_snap.forward_speed.max(0.0).round()) as u32;
    let speed = format!("SPEED {}", speed_val);

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
fn setup_hud(mut commands: Commands) {
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
                            TextBundle::from_section("TIME 00:00.00", text_style.clone()),
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
                    TextBundle::from_section("SPEED 0", text_style.clone()),
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

fn update_hud(shell: Res<ShellSimulation>, mut hud_query: Query<(&HudElement, &mut Text)>) {
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

/// Hides the HUD and countdown overlay while the menu is shown.
pub fn hide_hud(
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
pub fn show_hud(
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

fn spawn_overlay_root<'a>(
    commands: &'a mut Commands,
    row_gap: f32,
    alpha: f32,
) -> bevy::ecs::system::EntityCommands<'a> {
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(row_gap),
            ..default()
        },
        background_color: BackgroundColor(Color::srgba(0.05, 0.05, 0.08, alpha)),
        ..default()
    })
}
/// Query matching menu and results screen roots.
type ScreensQuery<'w, 's> = Query<'w, 's, Entity, Or<(With<MenuUi>, With<ResultsUi>)>>;

/// Despawns menu and results screens when leaving them.
pub fn despawn_screens(mut commands: Commands, screens_q: ScreensQuery) {
    for entity in screens_q.iter() {
        commands.entity(entity).despawn_recursive();
    }
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

/// Updates the countdown overlay text from the simulation phase.
pub fn update_countdown_overlay(
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

/// Best lap loaded from local storage, shown on the menu as the target to beat.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct SavedBestLap(pub Option<f32>);

/// File name holding the persisted best lap inside the app config directory.
pub const BEST_LAP_FILE_NAME: &str = "best_lap.txt";

/// Platform config base directory for local-only storage.
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
        if let Ok(stored) = maybe_save_best_lap(&default_best_lap_path(), best) {
            saved.0 = stored;
        }
    }
}

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

/// Formats a 1-indexed position with its ordinal suffix.
fn ordinal(position: usize) -> String {
    match position {
        1 => "1st".to_owned(),
        2 => "2nd".to_owned(),
        3 => "3rd".to_owned(),
        n => format!("{n}th"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TRACK_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/data/tracks/sample-circuit.json"
    );

    /// The sample circuit ships as a real data file next to the crate, not
    /// just as an embedded string.
    #[test]
    fn sample_track_data_ships_on_disk_and_parses() {
        let text = std::fs::read_to_string(SAMPLE_TRACK_PATH).unwrap();
        topdown_racer_core::track::Track::parse(&text).unwrap();
    }

    #[test]
    fn letterbox_viewport_exact_aspect_ratio_fills_window() {
        let vp = compute_letterbox_viewport(1920, 1080, 16.0 / 9.0);
        assert_eq!(vp.x, 0);
        assert_eq!(vp.y, 0);
        assert_eq!(vp.width, 1920);
        assert_eq!(vp.height, 1080);
    }

    #[test]
    fn letterbox_viewport_wider_window_adds_pillarbox() {
        let vp = compute_letterbox_viewport(2560, 1080, 16.0 / 9.0);
        assert_eq!(vp.width, 1920);
        assert_eq!(vp.height, 1080);
        assert_eq!(vp.x, 320); // (2560 - 1920) / 2
        assert_eq!(vp.y, 0);
    }

    #[test]
    fn letterbox_viewport_taller_window_adds_letterbox() {
        let vp = compute_letterbox_viewport(1080, 1920, 16.0 / 9.0);
        assert_eq!(vp.width, 1080);
        assert_eq!(vp.height, 608);
        assert_eq!(vp.x, 0);
        assert_eq!(vp.y, (1920 - 608) / 2);
    }

    #[test]
    fn wrap_angle_normalizes_to_pi_range() {
        assert_eq!(wrap_angle(0.0), 0.0);
        let pi = std::f32::consts::PI;
        assert!((wrap_angle(pi + 0.1) - (-pi + 0.1)).abs() < 1e-5);
        assert!((wrap_angle(-pi - 0.1) - (pi - 0.1)).abs() < 1e-5);
    }

    #[test]
    fn interpolate_pose_endpoints_and_midpoint() {
        let p0 = Vec2::new(0.0, 10.0);
        let p1 = Vec2::new(20.0, 30.0);
        assert_eq!(interpolate_pose(p0, p1, 0.0), p0);
        assert_eq!(interpolate_pose(p0, p1, 1.0), p1);
        assert_eq!(interpolate_pose(p0, p1, 0.5), Vec2::new(10.0, 20.0));
    }

    #[test]
    fn interpolate_heading_crosses_pi_boundary_smoothly() {
        let prev = 3.10; // close to +pi
        let curr = -3.10; // close to -pi
        let mid = interpolate_heading(prev, curr, 0.5);
        assert!(
            mid.abs() > 3.0,
            "midpoint across boundary must stay near +/- pi: got {}",
            mid
        );
    }

    #[test]
    fn follow_camera_transform_tracks_car_pose_with_identity_rotation() {
        let p0 = Vec2::new(10.0, 20.0);
        let p1 = Vec2::new(30.0, 40.0);
        let alpha = 0.5;
        let interp = interpolate_pose(p0, p1, alpha);

        let mut cam_tf = Transform::from_xyz(0.0, 0.0, 999.0);
        cam_tf.translation.x = interp.x;
        cam_tf.translation.y = interp.y;
        cam_tf.rotation = Quat::IDENTITY;

        assert_eq!(cam_tf.translation.x, 20.0);
        assert_eq!(cam_tf.translation.y, 30.0);
        assert_eq!(cam_tf.rotation, Quat::IDENTITY);
    }

    #[test]
    fn map_keyboard_input_neutral_is_zero() {
        let input = map_keyboard_input(false, false, false, false, false);
        assert_eq!(input, CarInput::default());
    }

    #[test]
    fn map_keyboard_input_all_five_controls() {
        // 1. Throttle
        let throttle = map_keyboard_input(true, false, false, false, false);
        assert_eq!(throttle.throttle, 1.0);
        assert_eq!(throttle.brake, 0.0);

        // 2. Brake
        let brake = map_keyboard_input(false, true, false, false, false);
        assert_eq!(brake.brake, 1.0);
        assert_eq!(brake.throttle, 0.0);

        // 3. Steer left
        let left = map_keyboard_input(false, false, true, false, false);
        assert_eq!(left.steer, 1.0);

        // 4. Steer right
        let right = map_keyboard_input(false, false, false, true, false);
        assert_eq!(right.steer, -1.0);

        // 5. Handbrake
        let hb = map_keyboard_input(false, false, false, false, true);
        assert!(hb.handbrake);
    }

    #[test]
    fn map_keyboard_input_opposing_steer_cancels() {
        let input = map_keyboard_input(false, false, true, true, false);
        assert_eq!(input.steer, 0.0);
    }

    #[test]
    fn reverse_engages_when_stopped_and_holding_brake() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut sim = Sim::new(track, 1);

        // Car is at standstill. Holding down/brake for 24 ticks (> 16 ticks threshold)
        let brake_input = map_keyboard_input(false, true, false, false, false);
        let mut snaps = Vec::new();
        for _ in 0..24 {
            snaps.extend(sim.tick(&[brake_input]));
        }

        let last = snaps.last().unwrap();
        assert!(
            last.forward_speed < 0.0,
            "reverse must engage when braking at a standstill: speed was {}",
            last.forward_speed
        );
    }

    #[test]
    fn read_keyboard_input_system_maps_wasd_and_arrows() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<PlayerInput>()
            .add_systems(Update, read_keyboard_input);

        // Test WASD + Space
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.press(KeyCode::KeyW);
            keyboard.press(KeyCode::KeyA);
            keyboard.press(KeyCode::Space);
        }
        app.update();
        {
            let player_input = app.world().resource::<PlayerInput>();
            assert_eq!(player_input.0.throttle, 1.0);
            assert_eq!(player_input.0.steer, 1.0);
            assert!(player_input.0.handbrake);
        }

        // Test Arrows
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.release(KeyCode::KeyW);
            keyboard.release(KeyCode::KeyA);
            keyboard.release(KeyCode::Space);
            keyboard.press(KeyCode::ArrowDown);
            keyboard.press(KeyCode::ArrowRight);
        }
        app.update();
        {
            let player_input = app.world().resource::<PlayerInput>();
            assert_eq!(player_input.0.brake, 1.0);
            assert_eq!(player_input.0.steer, -1.0);
            assert!(!player_input.0.handbrake);
        }
    }

    fn sample_snapshot() -> CarSnapshot {
        use topdown_racer_core::simulation::RacePhase;
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
        assert_eq!(hud.current_lap_time, "TIME 00:12.34");
        assert_eq!(hud.best_lap_time, "BEST 00:18.45");
        assert_eq!(hud.speed, "SPEED 24");
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
        assert_eq!(hud_initial.speed, "SPEED 0");

        // Mutate snapshot directly and ensure 1:1 reflection in HUD
        snap.completed_laps = 2;
        snap.position = 2;
        snap.current_lap_time = 65.25;
        snap.best_lap_time = Some(61.80);
        snap.forward_speed = 31.7;

        let hud_updated = format_hud_data(&snap, 4);
        assert_eq!(hud_updated.lap, "LAP 3/3");
        assert_eq!(hud_updated.position, "POS 2nd/4");
        assert_eq!(hud_updated.current_lap_time, "TIME 01:05.25");
        assert_eq!(hud_updated.best_lap_time, "BEST 01:01.80");
        assert_eq!(hud_updated.speed, "SPEED 32");
    }

    #[test]
    fn menu_start_leads_into_countdown_and_control_unlocks_at_green() {
        use topdown_racer_core::simulation::{RacePhase, DEFAULT_COUNTDOWN_TICKS};
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut sim = Sim::new_race(track, 4);

        // Menu start produces a countdown with locked controls.
        assert_eq!(
            sim.phase(),
            RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS
            }
        );
        for _ in 0..DEFAULT_COUNTDOWN_TICKS - 1 {
            let snap = sim.tick(&[CarInput {
                throttle: 1.0,
                ..CarInput::default()
            }])[0];
            assert_eq!(
                snap.forward_speed, 0.0,
                "controls must stay locked during countdown"
            );
        }

        // Green: racing phase, throttle now moves the car.
        let snap = sim.tick(&[CarInput {
            throttle: 1.0,
            ..CarInput::default()
        }])[0];
        assert_eq!(snap.phase, RacePhase::Racing);
        let snap2 = sim.tick(&[CarInput {
            throttle: 1.0,
            ..CarInput::default()
        }])[0];
        assert!(
            snap2.forward_speed > 0.0,
            "control must unlock when the countdown ends"
        );
    }

    #[test]
    fn reset_races_replay_identically_across_restarts() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut shell_a = ShellSimulation::new(track.clone());
        let shell_b = ShellSimulation::new(track);

        // Drive shell_a deep into the race, then restart it mid-race.
        for _ in 0..500 {
            shell_a.sim.tick(&[CarInput {
                throttle: 1.0,
                ..CarInput::default()
            }]);
        }
        shell_a.reset_to_fresh_race();

        // The restarted race must replay identically to a fresh race.
        let mut shell_b = shell_b;
        for _ in 0..300 {
            let snaps_a = shell_a.sim.tick(&[CarInput::default()]);
            let snaps_b = shell_b.sim.tick(&[CarInput::default()]);
            assert_eq!(snaps_a, snaps_b, "restarted races must replay identically");
        }
    }

    #[test]
    fn esc_to_menu_and_restart_runs_a_fresh_countdown() {
        use topdown_racer_core::simulation::{RacePhase, DEFAULT_COUNTDOWN_TICKS};
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut shell = ShellSimulation::new(track);

        // Drive the race forward past the countdown.
        for _ in 0..DEFAULT_COUNTDOWN_TICKS + 100 {
            shell.sim.tick(&[CarInput {
                throttle: 1.0,
                ..CarInput::default()
            }]);
        }
        shell.curr_snapshots = shell.sim.tick(&[CarInput::default()]);
        assert_eq!(shell.curr_snapshots[0].phase, RacePhase::Racing);

        // ESC returns to the menu; starting again rebuilds a fresh countdown race.
        shell.reset_to_fresh_race();
        assert_eq!(
            shell.sim.phase(),
            RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS - 1
            }
        );
        assert_eq!(shell.curr_snapshots[0].completed_laps, 0);
        assert_eq!(shell.curr_snapshots[0].forward_speed, 0.0);
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
            let angle_diff = wrap_angle(target_angle - last_heading);
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

    #[test]
    fn restart_produces_a_clean_fresh_race() {
        use topdown_racer_core::simulation::{RacePhase, DEFAULT_COUNTDOWN_TICKS};
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut shell = ShellSimulation::new(track);

        // Run deep into a race, then instant-restart without the menu.
        for _ in 0..DEFAULT_COUNTDOWN_TICKS + 200 {
            shell.sim.tick(&[CarInput {
                throttle: 1.0,
                ..CarInput::default()
            }]);
            shell.racing_ticks += 1;
        }
        assert!(shell.racing_ticks > 0);

        shell.reset_to_fresh_race();
        assert_eq!(
            shell.sim.phase(),
            RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS - 1
            }
        );
        assert_eq!(shell.racing_ticks, 0);
        for snap in &shell.curr_snapshots {
            assert_eq!(snap.completed_laps, 0);
            assert_eq!(snap.lap_times, [None; 3]);
            assert_eq!(snap.best_lap_time, None);
            assert_eq!(snap.forward_speed, 0.0);
        }
    }

    fn temp_best_lap_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "topdown-racer-test-{}-{}-best_lap.txt",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn best_lap_survives_app_restarts() {
        let path = temp_best_lap_path("restart");
        assert_eq!(load_best_lap(&path), None);

        // First session saves a best lap; a fresh load (new process) reads it back.
        let stored = maybe_save_best_lap(&path, 18.455).unwrap();
        assert_eq!(stored, Some(18.455));
        assert_eq!(load_best_lap(&path), Some(18.455));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn menu_shows_the_saved_best_lap_as_the_target() {
        assert_eq!(
            format_best_target(Some(18.455)),
            format!("TARGET TO BEAT — BEST {}", format_time(18.455))
        );
        assert_eq!(format_best_target(None), "TARGET TO BEAT — no best lap yet");
    }

    #[test]
    fn only_a_genuine_improvement_overwrites_the_saved_best() {
        let path = temp_best_lap_path("improvement");

        // First ever lap always saves.
        assert_eq!(maybe_save_best_lap(&path, 20.0).unwrap(), Some(20.0));

        // A slower lap never overwrites the saved best.
        assert_eq!(maybe_save_best_lap(&path, 25.0).unwrap(), Some(20.0));
        assert_eq!(load_best_lap(&path), Some(20.0));

        // A faster lap overwrites the saved best.
        assert_eq!(maybe_save_best_lap(&path, 18.25).unwrap(), Some(18.25));
        assert_eq!(load_best_lap(&path), Some(18.25));

        // Malformed saves read as missing and get overwritten.
        std::fs::write(&path, "not-a-time\n").unwrap();
        assert_eq!(load_best_lap(&path), None);
        assert_eq!(maybe_save_best_lap(&path, 19.0).unwrap(), Some(19.0));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn default_best_lap_path_lives_in_the_app_config_dir() {
        let path = default_best_lap_path();
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some(BEST_LAP_FILE_NAME)
        );
        assert!(
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some("topdown-racer")
        );
    }
}
