//! Shell library for Topdown Racer: camera, rendering, and simulation glue.

mod audio;
mod best_lap;
mod camera;
mod capture;
mod fmt;
mod hud;
mod menu;
mod overlay;
mod race;
mod results;
mod track_geometry;

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
    compute_letterbox_viewport, interpolate_heading, interpolate_pose, FollowCamera, ViewportRect,
};
pub use capture::{frame_capture_system, FrameCapture};
pub use fmt::{format_opt_lap_time, format_time};
pub use hud::{countdown_display, format_hud_data, CountdownText, HudData, HudElement, HudRoot};
pub use menu::{spawn_menu_ui, MenuUi, StartButton};
pub use overlay::despawn_screens;
pub use race::{
    auto_start_race, detect_race_finish, esc_to_menu_system, menu_action_system,
    reset_race_on_enter, results_action_system, AppState, RaceLifecyclePlugin,
};
pub use results::{format_results, should_show_results, spawn_results_ui, ResultRow, ResultsUi};

use bevy::prelude::*;
use glam::Vec2;
use topdown_racer_core::{
    ai::AiDriver,
    simulation::{CarInput, CarSnapshot, Sim, FIXED_HZ},
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
}

impl ShellSimulation {
    pub fn new(track: Track) -> Self {
        Self::from_sim(Sim::new_race(track, TOTAL_RACE_CARS))
    }

    fn from_sim(mut sim: Sim) -> Self {
        sim.set_ai(0, Some(AiDriver::new(0)));
        let initial = sim.snapshots();
        Self {
            sim,
            prev_snapshots: initial.clone(),
            curr_snapshots: initial,
        }
    }

    /// Rebuilds a fresh race starting from the countdown phase, preserving the
    /// player's autopilot toggle across the reset.
    pub fn reset_to_fresh_race(&mut self) {
        let track = self.sim.track().clone();
        let autopilot = self.sim.ai_enabled(0);
        *self = Self::from_sim(Sim::new_race(track, TOTAL_RACE_CARS));
        if !autopilot {
            self.sim.set_ai(0, None);
        }
    }
}

/// Main game plugin wiring camera, track rendering, simulation fixed step, and motion interpolation.
pub struct RacerGamePlugin;

impl Plugin for RacerGamePlugin {
    fn build(&self, app: &mut App) {
        let track = Track::parse(SAMPLE_CIRCUIT).expect("sample circuit must parse");
        let auto_start = std::env::var("TOPDOWN_AUTO_START").as_deref() == Ok("1");
        let capture_frames = std::env::var("TOPDOWN_CAPTURE").as_deref() == Ok("1");
        app.add_plugins(RaceLifecyclePlugin)
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
            .add_systems(OnEnter(AppState::Race), (show_hud, unmute_audio))
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

fn setup_track(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sim: Res<ShellSimulation>,
) {
    let track = sim.sim.track();

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

    // All quad math comes from the pure geometry seam; this system only
    // chooses materials and z-ordering.
    let geo = track_geometry::build_track_geometry(track);

    for road_quad in &geo.road {
        let mat = match road_quad.surface {
            Surface::Road => road_mat.clone(),
            Surface::Gravel => gravel_mat.clone(),
            Surface::Grass => continue,
        };
        spawn_quad(&mut commands, &mut meshes, road_quad.quad, mat, 1.0);
    }
    for quad in &geo.edge_lines {
        spawn_quad(&mut commands, &mut meshes, *quad, white_mat.clone(), 1.2);
    }
    for quad in &geo.dashes {
        spawn_quad(&mut commands, &mut meshes, *quad, white_mat.clone(), 1.1);
    }
    for (i, quad) in geo.kerbs.iter().enumerate() {
        let mat = if i % 2 == 0 {
            kerb_red.clone()
        } else {
            kerb_white.clone()
        };
        spawn_quad(&mut commands, &mut meshes, *quad, mat, 1.3);
    }
    for quad in &geo.guardrails {
        spawn_quad(
            &mut commands,
            &mut meshes,
            *quad,
            guardrail_mat.clone(),
            2.0,
        );
    }
    for (i, quad) in geo.start_line.iter().enumerate() {
        let mat = if (i / 10 + i % 10) % 2 == 0 {
            white_mat.clone()
        } else {
            black_mat.clone()
        };
        spawn_quad(&mut commands, &mut meshes, *quad, mat, 1.4);
    }
}

/// Spawns one quad as a colored 2D mesh at the given z height.
fn spawn_quad(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    quad: track_geometry::Quad,
    material: Handle<ColorMaterial>,
    z: f32,
) {
    commands.spawn(ColorMesh2dBundle {
        mesh: meshes
            .add(create_quad_mesh(
                quad.verts[0],
                quad.verts[1],
                quad.verts[2],
                quad.verts[3],
            ))
            .into(),
        material,
        transform: Transform::from_xyz(0.0, 0.0, z),
        ..default()
    });
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

/// Fixed step system: advances the simulation at 64 Hz using mapped player
/// inputs. Cars with an AI driver compute their own input unless the player
/// overrides it for that tick.
fn step_simulation(mut shell: ResMut<ShellSimulation>, player_input: Res<PlayerInput>) {
    shell.prev_snapshots = shell.curr_snapshots.clone();
    shell.curr_snapshots = shell.sim.tick(&[player_input.0]);
}

/// Toggles player car autopilot with the T key.
pub fn toggle_autopilot_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut shell: ResMut<ShellSimulation>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        let on = shell.sim.ai_enabled(0);
        shell
            .sim
            .set_ai(0, if on { None } else { Some(AiDriver::new(0)) });
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
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS
            }
        );
        assert_eq!(shell.curr_snapshots[0].completed_laps, 0);
        assert_eq!(shell.curr_snapshots[0].forward_speed, 0.0);
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
        }
        assert!(shell.sim.racing_ticks() > 0);

        shell.reset_to_fresh_race();
        assert_eq!(
            shell.sim.phase(),
            RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS
            }
        );
        assert_eq!(shell.sim.racing_ticks(), 0);
        for snap in &shell.curr_snapshots {
            assert_eq!(snap.completed_laps, 0);
            assert_eq!(snap.lap_times, [None; 3]);
            assert_eq!(snap.best_lap_time, None);
            assert_eq!(snap.forward_speed, 0.0);
        }
    }

    #[test]
    fn reset_preserves_the_autopilot_toggle() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut shell = ShellSimulation::new(track);
        assert!(shell.sim.ai_enabled(0), "autopilot starts enabled");

        shell.sim.set_ai(0, None);
        shell.reset_to_fresh_race();
        assert!(
            !shell.sim.ai_enabled(0),
            "a disabled autopilot toggle must survive a race reset"
        );

        shell.sim.set_ai(0, Some(AiDriver::new(0)));
        shell.reset_to_fresh_race();
        assert!(
            shell.sim.ai_enabled(0),
            "an enabled autopilot toggle must survive a race reset"
        );
    }
}
