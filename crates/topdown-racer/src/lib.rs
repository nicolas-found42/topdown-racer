//! Shell library for Topdown Racer: camera, rendering, and simulation glue.

use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};
use glam::Vec2;
use topdown_racer_core::{
    simulation::{CarInput, CarSnapshot, Sim, FIXED_HZ},
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
}

impl ShellSimulation {
    pub fn new(track: Track) -> Self {
        let mut sim = Sim::new(track, TOTAL_RACE_CARS);
        sim.enable_ai_opponents();
        let initial = sim.tick(&[CarInput::default()]);
        Self {
            sim,
            prev_snapshots: initial.clone(),
            curr_snapshots: initial,
        }
    }
}

/// Main game plugin wiring camera, track rendering, simulation fixed step, and motion interpolation.
pub struct RacerGamePlugin;

impl Plugin for RacerGamePlugin {
    fn build(&self, app: &mut App) {
        let track = Track::parse(SAMPLE_CIRCUIT).expect("sample circuit must parse");
        app.insert_resource(Time::<Fixed>::from_hz(FIXED_HZ as f64))
            .insert_resource(ShellSimulation::new(track))
            .init_resource::<PlayerInput>()
            .add_systems(Startup, (setup_camera, setup_track, setup_car, setup_hud))
            .add_systems(PreUpdate, read_keyboard_input)
            .add_systems(FixedUpdate, step_simulation)
            .add_systems(
                Update,
                (update_letterbox, interpolate_car_and_camera, update_hud),
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

    let pos_suffix = match player_snap.position {
        1 => "1st".to_owned(),
        2 => "2nd".to_owned(),
        3 => "3rd".to_owned(),
        n => format!("{n}th"),
    };
    let position = format!("POS {}/{}", pos_suffix, total_cars);

    let current_lap_time = format!("TIME {}", format_time(player_snap.current_lap_time));

    let best_lap_time = match player_snap.best_lap_time {
        Some(best) => format!("BEST {}", format_time(best)),
        None => "BEST --:--.--".to_owned(),
    };

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
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::SpaceBetween,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            ..default()
        })
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
}
