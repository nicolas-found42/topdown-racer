//! Shell library for Topdown Racer: camera, rendering, and simulation glue.

mod audio;
mod best_lap;
mod camera;
mod hud;
mod menu;
mod overlay;
mod results;

pub use audio::{
    create_pcm_wav, engine_pitch_from_speed, generate_engine_loop_wav, generate_skid_loop_wav,
    mute_audio, skid_volume_from_drift, unmute_audio, update_audio, EngineAudio, SkidAudio,
    DEFAULT_ENGINE_VOLUME,
};
pub use best_lap::{
    default_best_lap_path, format_best_target, load_best_lap, maybe_save_best_lap,
    persist_best_lap_on_finish, SavedBestLap, BEST_LAP_FILE_NAME,
};
pub use camera::{
    compute_letterbox_viewport, interpolate_heading, interpolate_pose, wrap_angle, FollowCamera,
    ViewportRect,
};
pub use hud::{
    countdown_display, format_hud_data, format_time, CountdownText, HudData, HudElement, HudRoot,
};
pub use menu::{
    esc_to_menu_system, menu_action_system, reset_race_on_enter, spawn_menu_ui, MenuUi, StartButton,
};
pub use overlay::despawn_screens;
pub use results::{
    detect_race_finish, format_opt_lap_time, format_results, results_action_system,
    should_show_results, spawn_results_ui, ResultRow, ResultsUi,
};

use bevy::{
    prelude::*, render::view::window::screenshot::ScreenshotManager, window::PrimaryWindow,
};
use glam::Vec2;
use topdown_racer_core::{
    simulation::{AiDriver, CarInput, CarSnapshot, RacePhase, Sim, FIXED_HZ},
    track::{Surface, Track, SAMPLE_CIRCUIT},
};

use audio::setup_audio;
use camera::{interpolate_car_and_camera, setup_camera, update_letterbox};
use hud::{hide_hud, setup_hud, show_hud, update_countdown_overlay, update_hud};

/// Canonical aspect ratio for the game view (16:9).
pub const TARGET_ASPECT_RATIO: f32 = 16.0 / 9.0;

/// Fixed camera zoom (orthographic scale). Smaller value = closer zoom.
pub const CAMERA_ZOOM: f32 = 0.05;

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
    /// Whether player car autopilot is enabled when no manual controls are pressed.
    pub autopilot: bool,
}

impl ShellSimulation {
    pub fn new(track: Track) -> Self {
        Self::from_sim(Sim::new_race(track, TOTAL_RACE_CARS))
    }

    fn from_sim(mut sim: Sim) -> Self {
        sim.set_ai(0, Some(AiDriver::new(0)));
        let initial = sim.tick(&[CarInput::default()]);
        Self {
            sim,
            prev_snapshots: initial.clone(),
            curr_snapshots: initial,
            racing_ticks: 0,
            autopilot: true,
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

/// Main game plugin wiring camera, track rendering, simulation fixed step, and motion interpolation.
pub struct RacerGamePlugin;

impl Plugin for RacerGamePlugin {
    fn build(&self, app: &mut App) {
        let track = Track::parse(SAMPLE_CIRCUIT).expect("sample circuit must parse");
        let auto_start = std::env::var("TOPDOWN_AUTO_START").as_deref() == Ok("1");
        let capture_frames = std::env::var("TOPDOWN_CAPTURE").as_deref() == Ok("1");

        app.init_state::<AppState>()
            .insert_resource(Time::<Fixed>::from_hz(FIXED_HZ as f64))
            .insert_resource(ShellSimulation::new(track))
            .insert_resource(SavedBestLap(load_best_lap(&default_best_lap_path())))
            .init_resource::<PlayerInput>()
            .insert_resource(FrameCapture {
                active: capture_frames,
                ..default()
            })
            .add_systems(
                Startup,
                (setup_camera, setup_track, setup_car, setup_hud, setup_audio),
            )
            .add_systems(
                OnEnter(AppState::Menu),
                (spawn_menu_ui, hide_hud, mute_audio),
            )
            .add_systems(OnExit(AppState::Menu), despawn_screens)
            .add_systems(
                OnEnter(AppState::Race),
                (reset_race_on_enter, show_hud, unmute_audio),
            )
            .add_systems(
                OnEnter(AppState::Results),
                (
                    spawn_results_ui,
                    hide_hud,
                    persist_best_lap_on_finish,
                    mute_audio,
                ),
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
                    update_audio.run_if(in_state(AppState::Race)),
                    toggle_autopilot_system.run_if(in_state(AppState::Race)),
                    frame_capture_system,
                ),
            );

        if auto_start {
            app.add_systems(Startup, auto_start_race);
        }
    }
}

/// Skips the menu when TOPDOWN_AUTO_START=1 by transitioning into the Race state
/// on the first frame, so OnEnter(Race) systems (HUD show, race reset) still run.
fn auto_start_race(mut next_state: ResMut<NextState<AppState>>) {
    next_state.set(AppState::Race);
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
    let grass_mesh = meshes.add(Rectangle::new(2400.0, 1600.0));
    let grass_mat = materials.add(Color::srgb(0.12, 0.36, 0.14));
    commands.spawn(ColorMesh2dBundle {
        mesh: grass_mesh.into(),
        material: grass_mat,
        transform: Transform::from_xyz(80.0, 60.0, 0.0),
        ..default()
    });

    let road_mat = materials.add(Color::srgb(0.20, 0.20, 0.24));
    let gravel_mat = materials.add(Color::srgb(0.55, 0.44, 0.28));
    let white_mat = materials.add(Color::srgb(0.95, 0.95, 0.95));
    let black_mat = materials.add(Color::srgb(0.10, 0.10, 0.12));
    let kerb_red = materials.add(Color::srgb(0.85, 0.18, 0.18));
    let kerb_white = materials.add(Color::srgb(0.95, 0.95, 0.95));
    let guardrail_mat = materials.add(Color::srgb(0.72, 0.75, 0.80));

    let road_half_w = track.road_half_width();
    let wall_dist = track.wall_distance();

    // Pre-calculate segment directions and normals
    let mut seg_dirs = Vec::with_capacity(n);
    let mut seg_normals = Vec::with_capacity(n);
    for i in 0..n {
        let dir = (points[i + 1] - points[i]).normalize_or_zero();
        let normal = Vec2::new(-dir.y, dir.x);
        seg_dirs.push(dir);
        seg_normals.push(normal);
    }

    // Pre-calculate corner miter vectors for each vertex i (0..=n)
    // Vertex i is at the junction between incoming segment (i + n - 1) % n and outgoing segment i % n.
    let mut miter_normals = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let prev_idx = if i == 0 { n - 1 } else { (i - 1) % n };
        let curr_idx = i % n;
        let n_prev = seg_normals[prev_idx];
        let n_curr = seg_normals[curr_idx];
        let dot = n_prev.dot(n_curr);
        let m = if dot > -0.999 {
            (n_prev + n_curr) / (1.0 + dot)
        } else {
            n_curr
        };
        miter_normals.push(m);
    }

    // Mitered road perimeter points (left and right)
    let edge_line_w = 0.35;
    let mut left_outer = Vec::with_capacity(n + 1);
    let mut right_outer = Vec::with_capacity(n + 1);
    let mut left_inner = Vec::with_capacity(n + 1);
    let mut right_inner = Vec::with_capacity(n + 1);

    for i in 0..=n {
        let p = points[i];
        let m = miter_normals[i];
        left_outer.push(p + m * road_half_w);
        right_outer.push(p - m * road_half_w);
        left_inner.push(p + m * (road_half_w - edge_line_w));
        right_inner.push(p - m * (road_half_w - edge_line_w));
    }

    for i in 0..n {
        let p0 = points[i];
        let p1 = points[i + 1];
        let dir = seg_dirs[i];
        let normal = seg_normals[i];
        let seg_len = (p1 - p0).length();

        let mat = match track.surfaces.get(i).copied().unwrap_or(Surface::Road) {
            Surface::Road => road_mat.clone(),
            Surface::Gravel => gravel_mat.clone(),
            Surface::Grass => continue,
        };

        // 1. Continuous mitered road surface quad
        let road_mesh = create_quad_mesh(
            left_outer[i],
            right_outer[i],
            right_outer[i + 1],
            left_outer[i + 1],
        );
        commands.spawn(ColorMesh2dBundle {
            mesh: meshes.add(road_mesh).into(),
            material: mat,
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        });

        // 2. Continuous mitered white edge lines (flush joints, no overlapping boxes)
        let left_edge_mesh = create_quad_mesh(
            left_outer[i],
            left_inner[i],
            left_inner[i + 1],
            left_outer[i + 1],
        );
        commands.spawn(ColorMesh2dBundle {
            mesh: meshes.add(left_edge_mesh).into(),
            material: white_mat.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 1.2),
            ..default()
        });

        let right_edge_mesh = create_quad_mesh(
            right_inner[i],
            right_outer[i],
            right_outer[i + 1],
            right_inner[i + 1],
        );
        commands.spawn(ColorMesh2dBundle {
            mesh: meshes.add(right_edge_mesh).into(),
            material: white_mat.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 1.2),
            ..default()
        });

        // 3. Dashed centerline (stops before corner junctions to avoid intersecting lines)
        let dash_step = 5.0;
        let dash_len = 2.4;
        let start_d = (road_half_w + 1.0).min(seg_len * 0.5);
        let end_d = (seg_len - road_half_w - 1.0).max(start_d);
        let mut d = start_d;
        while d + dash_len <= end_d {
            let d0 = p0 + dir * d;
            let d1 = p0 + dir * (d + dash_len);
            let dash_mesh = create_quad_mesh(
                d0 + normal * 0.18,
                d0 - normal * 0.18,
                d1 - normal * 0.18,
                d1 + normal * 0.18,
            );
            commands.spawn(ColorMesh2dBundle {
                mesh: meshes.add(dash_mesh).into(),
                material: white_mat.clone(),
                transform: Transform::from_xyz(0.0, 0.0, 1.1),
                ..default()
            });
            d += dash_step;
        }

        // 4. Red-and-white rumble strip kerbs along corner apexes
        let prev_idx = if i == 0 { n - 1 } else { i - 1 };
        let norm_prev = seg_normals[prev_idx];
        let dir_prev = seg_dirs[prev_idx];
        if (norm_prev - normal).length() > 0.001 {
            let kerb_w = 0.9;
            let kerb_len = 1.6;
            // Place kerbs on the outer side (-normal side for CCW loop) so they
            // sit in the runoff instead of overlapping the crossing leg's asphalt.
            // Incoming stretch towards corner
            for step in 0..4 {
                let d_end = (step as f32) * kerb_len;
                let d_start = (step as f32 + 1.0) * kerb_len;
                let pt0 = p0 - dir_prev * d_start;
                let pt1 = p0 - dir_prev * d_end;
                let quad = create_quad_mesh(
                    pt0 - norm_prev * (road_half_w + kerb_w),
                    pt0 - norm_prev * road_half_w,
                    pt1 - norm_prev * road_half_w,
                    pt1 - norm_prev * (road_half_w + kerb_w),
                );
                let mat = if step % 2 == 0 {
                    kerb_red.clone()
                } else {
                    kerb_white.clone()
                };
                commands.spawn(ColorMesh2dBundle {
                    mesh: meshes.add(quad).into(),
                    material: mat,
                    transform: Transform::from_xyz(0.0, 0.0, 1.3),
                    ..default()
                });
            }
            // Outgoing stretch from corner
            for step in 0..4 {
                let d_start = (step as f32) * kerb_len;
                let d_end = (step as f32 + 1.0) * kerb_len;
                let pt0 = p0 + dir * d_start;
                let pt1 = p0 + dir * d_end;
                let quad = create_quad_mesh(
                    pt0 - normal * (road_half_w + kerb_w),
                    pt0 - normal * road_half_w,
                    pt1 - normal * road_half_w,
                    pt1 - normal * (road_half_w + kerb_w),
                );
                let mat = if step % 2 == 0 {
                    kerb_red.clone()
                } else {
                    kerb_white.clone()
                };
                commands.spawn(ColorMesh2dBundle {
                    mesh: meshes.add(quad).into(),
                    material: mat,
                    transform: Transform::from_xyz(0.0, 0.0, 1.3),
                    ..default()
                });
            }
        }

        // 5. Outer boundary safety barrier (steel guardrail around outer perimeter only)
        // Outer side is -normal (-miter_normals), preventing walls from crossing infield
        let wall_w = 0.8;
        let w0 = points[i] - miter_normals[i] * wall_dist;
        let w1 = points[i + 1] - miter_normals[i + 1] * wall_dist;
        let w_dir = (w1 - w0).normalize_or_zero();
        let w_norm = Vec2::new(-w_dir.y, w_dir.x);
        let wall_quad = create_quad_mesh(
            w0 + w_norm * (wall_w * 0.5),
            w0 - w_norm * (wall_w * 0.5),
            w1 - w_norm * (wall_w * 0.5),
            w1 + w_norm * (wall_w * 0.5),
        );
        commands.spawn(ColorMesh2dBundle {
            mesh: meshes.add(wall_quad).into(),
            material: guardrail_mat.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 2.0),
            ..default()
        });
    }

    // Checkered start/finish line across the opening straight, just past the
    // corner exit: clear of the crossing leg's asphalt, and the whole grid
    // stages behind it.
    if n > 0 {
        let dir = seg_dirs[0];
        let normal = seg_normals[0];
        let line_center = points[0] + dir * (road_half_w + 3.0);
        let checkers_count = 10;
        let checker_w = (road_half_w * 2.0) / checkers_count as f32;
        for row in 0..2 {
            let row_offset = (row as f32 - 0.5) * 0.8;
            for col in 0..checkers_count {
                let col_offset = -road_half_w + (col as f32 + 0.5) * checker_w;
                let center = line_center + dir * row_offset + normal * col_offset;
                let sq = create_quad_mesh(
                    center + dir * 0.4 + normal * (checker_w * 0.5),
                    center + dir * 0.4 - normal * (checker_w * 0.5),
                    center - dir * 0.4 - normal * (checker_w * 0.5),
                    center - dir * 0.4 + normal * (checker_w * 0.5),
                );
                let mat = if (row + col) % 2 == 0 {
                    white_mat.clone()
                } else {
                    black_mat.clone()
                };
                commands.spawn(ColorMesh2dBundle {
                    mesh: meshes.add(sq).into(),
                    material: mat,
                    transform: Transform::from_xyz(0.0, 0.0, 1.4),
                    ..default()
                });
            }
        }
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
    let colors = [
        Color::srgb(0.88, 0.12, 0.12), // Player: Formula Red
        Color::srgb(0.12, 0.42, 0.88), // AI 1: Cobalt Blue
        Color::srgb(0.92, 0.78, 0.12), // AI 2: Solar Yellow
        Color::srgb(0.12, 0.78, 0.35), // AI 3: Emerald Green
    ];

    let tire_mesh = meshes.add(Rectangle::new(1.0, 0.48));
    let tire_mat = materials.add(Color::srgb(0.06, 0.06, 0.06));

    let chassis_mesh = meshes.add(Rectangle::new(4.2, 2.0));
    let nose_mesh = meshes.add(Rectangle::new(1.2, 1.6));
    let cockpit_mesh = meshes.add(Rectangle::new(1.8, 1.4));
    let cockpit_mat = materials.add(Color::srgb(0.08, 0.12, 0.18));

    let stripe_mesh = meshes.add(Rectangle::new(4.2, 0.35));
    let white_mat = materials.add(Color::srgb(0.95, 0.95, 0.95));

    let wing_mesh = meshes.add(Rectangle::new(0.4, 2.3));
    let carbon_mat = materials.add(Color::srgb(0.12, 0.12, 0.14));

    let headlight_mesh = meshes.add(Rectangle::new(0.3, 0.4));
    let headlight_mat = materials.add(Color::srgb(1.0, 0.96, 0.65));

    let taillight_mesh = meshes.add(Rectangle::new(0.2, 0.4));
    let taillight_mat = materials.add(Color::srgb(1.0, 0.15, 0.15));

    for (i, snap) in sim.curr_snapshots.iter().enumerate() {
        let body_mat = materials.add(colors[i % colors.len()]);

        commands
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(snap.pose.x, snap.pose.y, 10.0 + i as f32 * 0.1)
                        .with_rotation(Quat::from_rotation_z(snap.heading)),
                    ..default()
                },
                CarVisual { car_index: i },
            ))
            .with_children(|car| {
                // 4 Real Rubber Tires
                for &(x, y) in &[(1.3, 1.05), (1.3, -1.05), (-1.3, 1.05), (-1.3, -1.05)] {
                    car.spawn(ColorMesh2dBundle {
                        mesh: tire_mesh.clone().into(),
                        material: tire_mat.clone(),
                        transform: Transform::from_xyz(x, y, -0.01),
                        ..default()
                    });
                }
                // Main Aerodynamic Chassis Body
                car.spawn(ColorMesh2dBundle {
                    mesh: chassis_mesh.clone().into(),
                    material: body_mat.clone(),
                    transform: Transform::from_xyz(0.0, 0.0, 0.02),
                    ..default()
                });
                // Contoured Front Nose
                car.spawn(ColorMesh2dBundle {
                    mesh: nose_mesh.clone().into(),
                    material: body_mat.clone(),
                    transform: Transform::from_xyz(1.5, 0.0, 0.03),
                    ..default()
                });
                // Cockpit Glass
                car.spawn(ColorMesh2dBundle {
                    mesh: cockpit_mesh.clone().into(),
                    material: cockpit_mat.clone(),
                    transform: Transform::from_xyz(0.1, 0.0, 0.04),
                    ..default()
                });
                // White Racing Stripe on Player Car
                if i == 0 {
                    car.spawn(ColorMesh2dBundle {
                        mesh: stripe_mesh.clone().into(),
                        material: white_mat.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.05),
                        ..default()
                    });
                }
                // Rear Spoiler / Wing
                car.spawn(ColorMesh2dBundle {
                    mesh: wing_mesh.clone().into(),
                    material: carbon_mat.clone(),
                    transform: Transform::from_xyz(-2.0, 0.0, 0.06),
                    ..default()
                });
                // Twin Headlights
                for &y in &[0.65, -0.65] {
                    car.spawn(ColorMesh2dBundle {
                        mesh: headlight_mesh.clone().into(),
                        material: headlight_mat.clone(),
                        transform: Transform::from_xyz(2.0, y, 0.05),
                        ..default()
                    });
                }
                // Twin Taillights
                for &y in &[0.65, -0.65] {
                    car.spawn(ColorMesh2dBundle {
                        mesh: taillight_mesh.clone().into(),
                        material: taillight_mat.clone(),
                        transform: Transform::from_xyz(-2.05, y, 0.05),
                        ..default()
                    });
                }
            });
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
/// When autopilot is enabled and no manual controls are pressed, the player's car
/// is driven by the internal AI controller.
fn step_simulation(mut shell: ResMut<ShellSimulation>, player_input: Res<PlayerInput>) {
    shell.prev_snapshots = shell.curr_snapshots.clone();

    let manual_input = player_input.0;
    let snaps = shell.sim.tick(&[manual_input]);
    shell.curr_snapshots = snaps;
    if shell.sim.phase() == RacePhase::Racing {
        shell.racing_ticks += 1;
    }
}

/// Toggles player car autopilot with the T key.
pub fn toggle_autopilot_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut shell: ResMut<ShellSimulation>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        shell.autopilot = !shell.autopilot;
        let autopilot = shell.autopilot;
        shell.sim.set_ai(
            0,
            if autopilot {
                Some(AiDriver::new(0))
            } else {
                None
            },
        );
    }
}

/// Frame capture configuration for saving game window screenshots to disk via FFmpeg.
#[derive(Resource)]
pub struct FrameCapture {
    pub output_dir: std::path::PathBuf,
    pub max_frames: u32,
    pub frame_count: u32,
    pub active: bool,
    pub sender: Option<std::sync::mpsc::Sender<Vec<u8>>>,
}

impl Default for FrameCapture {
    fn default() -> Self {
        let max_frames = std::env::var("TOPDOWN_CAPTURE_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600);
        Self {
            output_dir: std::path::PathBuf::from("/tmp/topdown_frames"),
            max_frames,
            frame_count: 0,
            active: false,
            sender: None,
        }
    }
}

/// Captures game window frames and pipes them to FFmpeg rawvideo on stdin,
/// producing pixel-perfect PNG screenshots of the game window.
pub fn frame_capture_system(
    mut capture: ResMut<FrameCapture>,
    mut screenshot_manager: ResMut<ScreenshotManager>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
) {
    if !capture.active {
        return;
    }
    if capture.frame_count >= capture.max_frames {
        capture.active = false;
        capture.sender = None;
        info!(
            "frame capture complete: {} frames piped to ffmpeg in {}",
            capture.frame_count,
            capture.output_dir.display()
        );
        return;
    }

    let Ok((window_entity, window)) = windows.get_single() else {
        return;
    };

    if std::fs::create_dir_all(&capture.output_dir).is_err() {
        capture.active = false;
        return;
    }

    // Lazy initialization of the FFmpeg rawvideo child process
    if capture.sender.is_none() {
        let width = window.physical_width();
        let height = window.physical_height();
        let out_pattern = capture.output_dir.join("frame_%04d.png");
        let out_str = out_pattern.to_string_lossy().to_string();

        let child = std::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "rawvideo",
                "-pixel_format",
                "bgra",
                "-video_size",
                &format!("{width}x{height}"),
                "-framerate",
                "60",
                "-i",
                "pipe:0",
                "-c:v",
                "png",
                &out_str,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        match child {
            Ok(mut proc) => {
                let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
                std::thread::spawn(move || {
                    use std::io::Write;
                    if let Some(mut stdin) = proc.stdin.take() {
                        while let Ok(frame_data) = rx.recv() {
                            if stdin.write_all(&frame_data).is_err() {
                                break;
                            }
                        }
                    }
                    let _ = proc.wait();
                });
                capture.sender = Some(tx);
            }
            Err(e) => {
                warn!("failed to spawn ffmpeg: {e}; falling back to direct save");
            }
        }
    }

    capture.frame_count += 1;

    if let Some(tx) = capture.sender.as_ref().cloned() {
        if screenshot_manager
            .take_screenshot(window_entity, move |image| {
                let _ = tx.send(image.data);
            })
            .is_err()
        {
            capture.frame_count -= 1;
        }
    } else {
        let path = capture
            .output_dir
            .join(format!("frame_{:04}.png", capture.frame_count));
        if screenshot_manager
            .save_screenshot_to_disk(window_entity, &path)
            .is_err()
        {
            capture.frame_count -= 1;
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

    #[test]
    fn engine_pitch_strictly_increases_with_forward_speed() {
        let p0 = engine_pitch_from_speed(0.0);
        let p10 = engine_pitch_from_speed(10.0);
        let p20 = engine_pitch_from_speed(20.0);
        let p30 = engine_pitch_from_speed(30.0);

        assert!(
            p0 > 0.7 && p0 < 0.9,
            "idle pitch should be around 0.8: got {p0}"
        );
        assert!(
            p10 > p0,
            "pitch must increase with speed: p10={p10} > p0={p0}"
        );
        assert!(
            p20 > p10,
            "pitch must increase with speed: p20={p20} > p10={p10}"
        );
        assert!(
            p30 > p20,
            "pitch must increase with speed: p30={p30} > p20={p20}"
        );
    }

    #[test]
    fn skid_volume_is_active_during_drift_and_silent_when_grip_recovers() {
        // 1. Not drifting: silent
        assert_eq!(skid_volume_from_drift(false, 15.0), 0.0);
        assert_eq!(skid_volume_from_drift(false, 0.0), 0.0);

        // 2. Drifting at speed: audible skid cue
        let vol_drift = skid_volume_from_drift(true, 15.0);
        assert!(
            vol_drift > 0.5,
            "skid volume must be audible while drifting: got {vol_drift}"
        );

        // 3. Grip recovers (drifting becomes false): silent
        assert_eq!(skid_volume_from_drift(false, 15.0), 0.0);

        // 4. Standstill / stopped drift: silent
        assert_eq!(skid_volume_from_drift(true, 0.5), 0.0);
    }

    #[test]
    fn audio_systems_run_without_error_when_no_audio_device_is_present() {
        let mut app = App::new();
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        app.insert_resource(ShellSimulation::new(track))
            .add_systems(Update, (update_audio, mute_audio, unmute_audio));

        // Spawn audio entities without AudioSink (simulating headless runner without audio stream)
        app.world_mut().spawn(EngineAudio);
        app.world_mut().spawn(SkidAudio);

        // Must execute cleanly without error or panic
        app.update();
    }

    #[test]
    fn pcm_wav_generator_produces_valid_wave_header() {
        let engine_wav = generate_engine_loop_wav();
        assert!(engine_wav.len() > 44);
        assert_eq!(&engine_wav[0..4], b"RIFF");
        assert_eq!(&engine_wav[8..12], b"WAVE");
        assert_eq!(&engine_wav[12..16], b"fmt ");
        assert_eq!(&engine_wav[36..40], b"data");

        let skid_wav = generate_skid_loop_wav();
        assert!(skid_wav.len() > 44);
        assert_eq!(&skid_wav[0..4], b"RIFF");
        assert_eq!(&skid_wav[8..12], b"WAVE");
    }
}
