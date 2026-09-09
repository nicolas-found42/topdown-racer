//! Shell library for Topdown Racer: camera, rendering, and simulation glue.

mod audio;
mod best_lap;
mod camera;
mod capture;
mod fmt;
mod hud;
mod menu;
mod overlay;
pub mod palette;
mod race;
mod results;
mod track_geometry;
pub mod world_bake;

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
pub use track_geometry::{build_track_geometry, Quad, RoadQuad, TrackRenderGeometry, ZoneQuad};
pub use world_bake::{bake_world, WorldCanvas, TEXELS_PER_UNIT};

use bevy::prelude::*;
use glam::Vec2;
use topdown_racer_core::{
    simulation::{CarInput, CarSnapshot, Sim, FIXED_HZ},
    track::{PropKind, Track, SAMPLE_CIRCUIT},
};

use audio::setup_audio;
use camera::{interpolate_car_and_camera, setup_camera, update_letterbox};
use hud::{hide_hud, setup_hud, show_hud, update_countdown_overlay, update_hud};

/// Canonical aspect ratio for the game view (16:9).
pub const TARGET_ASPECT_RATIO: f32 = 16.0 / 9.0;

/// Fixed camera view in world units, independent of window size.
pub const CAMERA_VIEW_SIZE: Vec2 = Vec2::new(80.0, 45.0);

/// World z-stack. Skid decals lie on the bake; scenery and Cars sit above
/// them, with airborne driving FX above the Cars. Car children use local
/// offsets below 0.1; grid slots reserve another 0.1 each.
pub mod world_z {
    pub const BAKE: f32 = 0.0;
    pub const SKID: f32 = 1.0;
    pub const SCENERY: f32 = 2.0;
    pub const CAR: f32 = 3.0;
    pub const FX: f32 = 4.0;
}

/// Component tagging a rendered Car Sprite and identifying its car index.
#[derive(Component)]
pub struct CarSprite {
    pub car_index: usize,
}

/// Component tagging a front-wheel visual so steering can rotate it. The
/// wheel entity is a child of the Car body; its local rotation is the visual
/// steer angle.
#[derive(Component)]
pub struct FrontWheel {
    pub car_index: usize,
}

/// Maximum visual yaw of a front wheel at full steering lock, in radians.
pub const MAX_FRONT_WHEEL_ANGLE: f32 = 0.5;

/// Maps a steering demand (-1..1, positive left) to a front-wheel visual
/// angle (counter-clockwise positive, matching Bevy rotation_z).
pub fn front_wheel_angle(steer: f32) -> f32 {
    steer.clamp(-1.0, 1.0) * MAX_FRONT_WHEEL_ANGLE
}

/// Total cars in the race: 1 player car + 3 AI opponents.
pub const TOTAL_RACE_CARS: usize = 4;

/// Resource holding simulation state and consecutive snapshots for render interpolation.
#[derive(Resource)]
pub struct ShellSimulation {
    pub selected_pace: topdown_racer_core::ai::OpponentPace,
    pub sim: Sim,
    pub prev_snapshots: Vec<CarSnapshot>,
    pub curr_snapshots: Vec<CarSnapshot>,
}

impl ShellSimulation {
    pub fn new(track: Track) -> Self {
        Self::from_sim(Sim::new_race(track, TOTAL_RACE_CARS))
    }

    fn from_sim(sim: Sim) -> Self {
        let initial = sim.snapshots();
        Self {
            selected_pace: sim.opponent_pace(),
            sim,
            prev_snapshots: initial.clone(),
            curr_snapshots: initial,
        }
    }

    /// Rebuilds a fresh race starting from the countdown phase, preserving the
    /// player's autopilot toggle across the reset.
    pub fn reset_to_fresh_race(&mut self) {
        let track = self.sim.track().clone();
        let mode = self.sim.player_mode();
        *self = Self::from_sim(Sim::new_race_with_pace(
            track,
            TOTAL_RACE_CARS,
            self.selected_pace,
        ));
        self.sim.request_player_mode(mode);
    }
}

/// Main game plugin wiring camera, track rendering, simulation fixed step, and motion interpolation.
pub struct RacerGamePlugin;

impl Plugin for RacerGamePlugin {
    fn build(&self, app: &mut App) {
        let track = Track::parse(SAMPLE_CIRCUIT).expect("sample circuit must parse");
        let auto_start = std::env::var("TOPDOWN_AUTO_START").as_deref() == Ok("1");
        let capture_frames = std::env::var("TOPDOWN_CAPTURE").as_deref() == Ok("1");
        app.insert_resource(Msaa::Off)
            .add_plugins(RaceLifecyclePlugin)
            .insert_resource(Time::<Fixed>::from_hz(FIXED_HZ as f64))
            .insert_resource(ShellSimulation::new(track))
            .insert_resource(SavedBestLap(load_best_lap(&default_best_lap_path())))
            .init_resource::<PlayerInput>()
            .init_resource::<ControlGate>()
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
                (
                    toggle_autopilot_system
                        .run_if(in_state(AppState::Menu).or_else(in_state(AppState::Race))),
                    read_keyboard_input.run_if(in_state(AppState::Race)),
                )
                    .chain()
                    .after(bevy::input::InputSystem),
            )
            .add_systems(
                FixedUpdate,
                step_simulation.run_if(in_state(AppState::Race)),
            )
            .add_systems(
                Update,
                (
                    update_letterbox,
                    scale_ui_to_window,
                    menu::update_driving_summary,
                    interpolate_car_and_camera.run_if(in_state(AppState::Race)),
                    update_front_wheel_steer.run_if(in_state(AppState::Race)),
                    update_hud.run_if(in_state(AppState::Race)),
                    update_countdown_overlay.run_if(in_state(AppState::Race)),
                    update_audio.run_if(in_state(AppState::Race)),
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
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
    sim: Res<ShellSimulation>,
) {
    use bevy::render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::ImageSampler,
    };

    let track = sim.sim.track();
    let canvas = bake_world(&build_track_geometry(track), track.theme);
    let size = Vec2::new(canvas.width as f32, canvas.height as f32) / TEXELS_PER_UNIT;
    let center = canvas.world_origin() + size * 0.5;
    let mut image = Image::new(
        Extent3d {
            width: canvas.width,
            height: canvas.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        canvas.pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    commands.spawn(SpriteBundle {
        texture: images.add(image),
        sprite: Sprite {
            custom_size: Some(size),
            ..default()
        },
        transform: Transform::from_xyz(center.x, center.y, world_z::BAKE),
        ..default()
    });

    // Scenery reads Track placement and renders at native sprite density.
    for prop in &track.props {
        let (path, size) = match prop.kind {
            PropKind::Tree => ("sprites/scenery/tree.png", Vec2::new(3.0, 4.0)),
            PropKind::TireStack => ("sprites/scenery/tire_stack.png", Vec2::new(2.0, 2.0)),
            PropKind::BrakeBoard => ("sprites/scenery/brake_board.png", Vec2::new(1.5, 2.0)),
        };
        commands.spawn(SpriteBundle {
            texture: assets.load(path),
            transform: Transform::from_xyz(prop.position.x, prop.position.y, world_z::SCENERY)
                .with_rotation(Quat::from_rotation_z(prop.rotation)),
            sprite: Sprite {
                custom_size: Some(size),
                ..default()
            },
            ..default()
        });
    }
}

/// Livery textures, one Car Sprite per grid slot; slot 0 is the player car.
const LIVERY_TEXTURES: [&str; TOTAL_RACE_CARS] = [
    "sprites/car/body_blue.png",
    "sprites/car/body_white.png",
    "sprites/car/body_orange.png",
    "sprites/car/body_plum.png",
];

/// Front-wheel sprite shared by every car.
const WHEEL_TEXTURE: &str = "sprites/car/wheel.png";

fn setup_car(mut commands: Commands, asset_server: Res<AssetServer>, sim: Res<ShellSimulation>) {
    let wheel_texture: Handle<Image> = asset_server.load(WHEEL_TEXTURE);
    for (i, snap) in sim.curr_snapshots.iter().enumerate() {
        let body_texture: Handle<Image> =
            asset_server.load(LIVERY_TEXTURES[i % LIVERY_TEXTURES.len()]);

        commands
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(
                        snap.pose.x,
                        snap.pose.y,
                        world_z::CAR + i as f32 * 0.1,
                    )
                    .with_rotation(Quat::from_rotation_z(snap.heading)),
                    ..default()
                },
                CarSprite { car_index: i },
            ))
            .with_children(|car| {
                // Body Car Sprite, authored at native density (4.2 x 2.0
                // world units); rear wheels and wing are baked into the art.
                car.spawn(SpriteBundle {
                    texture: body_texture.clone(),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(34.0 / TEXELS_PER_UNIT, 2.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 0.0, 0.02),
                    ..default()
                });
                // Separate front-wheel sprites at the front axle so the steer
                // system keeps yawing them; the body art leaves wheel-well
                // arches where they sit.
                for &(x, y) in &[(1.3, 1.05), (1.3, -1.05)] {
                    car.spawn((
                        SpriteBundle {
                            texture: wheel_texture.clone(),
                            sprite: Sprite {
                                custom_size: Some(Vec2::new(
                                    5.0 / TEXELS_PER_UNIT,
                                    4.0 / TEXELS_PER_UNIT,
                                )),
                                ..default()
                            },
                            transform: Transform::from_xyz(x, y, 0.03),
                            ..default()
                        },
                        FrontWheel { car_index: i },
                    ));
                }
            });
    }
}

/// Player input mapped from keyboard devices.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerInput(pub CarInput);

/// Held movement keys must return to neutral after ownership or screen changes.
#[derive(Resource, Default)]
pub struct ControlGate(bool);

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
    mut gate: ResMut<ControlGate>,
) {
    let up = keyboard.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]);
    let down = keyboard.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]);
    let left = keyboard.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]);
    let right = keyboard.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]);
    let handbrake = keyboard.pressed(KeyCode::Space);
    let input = map_keyboard_input(up, down, left, right, handbrake);
    if gate.0 && !(up || down || left || right || handbrake) {
        gate.0 = false;
    }
    player_input.0 = if gate.0 { CarInput::default() } else { input };
}

/// Fixed step system: advances the simulation at 64 Hz using mapped player
/// inputs. Cars with an AI driver compute their own input unless the player
/// overrides it for that tick.
fn step_simulation(mut shell: ResMut<ShellSimulation>, player_input: Res<PlayerInput>) {
    shell.prev_snapshots = shell.curr_snapshots.clone();
    shell.curr_snapshots = shell.sim.tick(&[player_input.0]);
}

/// Yaws each front-wheel visual to the interpolated steering demand. The
/// angle lerps between the previous and current snapshots with the same
/// fixed-step overstep fraction the body interpolation uses, so the wheels
/// lead the chassis through a turn instead of sitting straight.
pub(crate) fn update_front_wheel_steer(
    shell: Res<ShellSimulation>,
    fixed_time: Res<Time<Fixed>>,
    mut wheels: Query<(&FrontWheel, &mut Transform)>,
) {
    let alpha = fixed_time.overstep_fraction();
    for (wheel, mut tf) in wheels.iter_mut() {
        if let (Some(prev), Some(curr)) = (
            shell.prev_snapshots.get(wheel.car_index),
            shell.curr_snapshots.get(wheel.car_index),
        ) {
            let steer = prev.steer + (curr.steer - prev.steer) * alpha;
            tf.rotation = Quat::from_rotation_z(front_wheel_angle(steer));
        }
    }
}

/// Toggles player car autopilot with the T key.
pub fn toggle_autopilot_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut shell: ResMut<ShellSimulation>,
    mut gate: ResMut<ControlGate>,
    mut input: ResMut<PlayerInput>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        let mode = shell.sim.player_mode().toggled();
        shell.sim.request_player_mode(mode);
        gate.0 = true;
        input.0 = CarInput::default();
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
    fn new_shell_defaults_to_manual() {
        let mut shell = ShellSimulation::new(Track::parse(SAMPLE_CIRCUIT).unwrap());
        assert!(!shell.sim.ai_enabled(0));
        for _ in 0..240 {
            assert_eq!(shell.sim.tick(&[])[0].throttle, 0.0);
        }
    }

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
            .init_resource::<ControlGate>()
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
    fn toggling_to_manual_requires_held_controls_to_be_released() {
        let mut app = App::new();
        let mut shell = ShellSimulation::new(Track::parse(SAMPLE_CIRCUIT).unwrap());
        shell
            .sim
            .request_player_mode(topdown_racer_core::simulation::DrivingMode::Autopilot);
        shell.sim.tick(&[]);
        app.insert_resource(shell)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<PlayerInput>()
            .init_resource::<ControlGate>()
            .add_systems(
                Update,
                (
                    toggle_autopilot_system,
                    read_keyboard_input,
                    step_simulation,
                )
                    .chain(),
            );
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyW);
            keys.press(KeyCode::KeyA);
            keys.press(KeyCode::KeyT);
        }
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        for _ in 0..5 {
            app.update();
            let snap = app.world().resource::<ShellSimulation>().curr_snapshots[0];
            assert_eq!((snap.throttle, snap.steer), (0.0, 0.0));
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.update();
        assert_eq!(app.world().resource::<PlayerInput>().0.throttle, 1.0);
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
        assert!(!shell.sim.ai_enabled(0), "Manual starts enabled");

        shell.sim.set_ai(0, None);
        shell.reset_to_fresh_race();
        assert!(
            !shell.sim.ai_enabled(0),
            "a disabled autopilot toggle must survive a race reset"
        );

        shell
            .sim
            .request_player_mode(topdown_racer_core::simulation::DrivingMode::Autopilot);
        shell.reset_to_fresh_race();
        assert!(
            shell.sim.player_mode() == topdown_racer_core::simulation::DrivingMode::Autopilot,
            "an enabled autopilot toggle must survive a race reset"
        );
    }

    #[test]
    fn front_wheel_angle_tracks_steer_demand() {
        assert_eq!(front_wheel_angle(0.0), 0.0);
        assert_eq!(front_wheel_angle(1.0), MAX_FRONT_WHEEL_ANGLE);
        assert_eq!(front_wheel_angle(-1.0), -MAX_FRONT_WHEEL_ANGLE);
        // Out-of-range demands clamp to full lock rather than over-rotating.
        assert_eq!(front_wheel_angle(2.0), MAX_FRONT_WHEEL_ANGLE);
        assert_eq!(front_wheel_angle(-2.0), -MAX_FRONT_WHEEL_ANGLE);
    }

    /// The full render path: the real car scene plus the real steer system
    /// must yaw both front wheels per car to the snapshot steer angle. A
    /// missed marker or missing system registration fails here, not on screen.
    #[test]
    fn front_wheels_yaw_with_snapshot_steer() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut shell = ShellSimulation::new(track);
        for snapshots in [&mut shell.prev_snapshots, &mut shell.curr_snapshots] {
            for (k, snap) in snapshots.iter_mut().enumerate() {
                snap.steer = if k == 0 { 1.0 } else { -1.0 };
            }
        }
        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.insert_resource(Time::<Fixed>::from_hz(FIXED_HZ as f64));
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<ColorMaterial>>();
        app.init_asset::<Image>();
        app.insert_resource(shell);
        app.add_systems(Startup, setup_car);
        app.add_systems(Update, update_front_wheel_steer);
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        app.update();

        let mut wheels = app.world_mut().query::<(&FrontWheel, &Transform)>();
        let world = app.world();
        assert_eq!(wheels.iter(world).len(), 2 * TOTAL_RACE_CARS,);
        for (wheel, tf) in wheels.iter(world) {
            let angle = 2.0 * tf.rotation.z.atan2(tf.rotation.w);
            let expected = if wheel.car_index == 0 {
                MAX_FRONT_WHEEL_ANGLE
            } else {
                -MAX_FRONT_WHEEL_ANGLE
            };
            assert!(
                (angle - expected).abs() < 1e-4,
                "car {} wheel must yaw to {expected}, got {angle}",
                wheel.car_index,
            );
        }
    }

    /// Issue #42: the shell renders the baked world canvas as a single
    /// textured quad. `setup_track` must spawn exactly one bake Sprite
    /// (nearest sampling, canvas-sized, at `world_z::BAKE`), zero flat
    /// color-mesh quads, scenery above the bake, and a strictly ordered
    /// z-stack with room for the skid/FX layers that follow.
    #[test]
    fn setup_track_renders_the_baked_canvas_as_one_quad() {
        use bevy::render::texture::{ImageFilterMode, ImageSampler};

        assert!(world_z::BAKE < world_z::SKID);
        assert!(world_z::SKID < world_z::SCENERY);
        assert!(world_z::SCENERY < world_z::CAR);
        assert!(world_z::CAR < world_z::FX);

        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let canvas = bake_world(&build_track_geometry(&track), track.theme);
        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<Image>();
        app.insert_resource(ShellSimulation::new(track));
        app.add_systems(Startup, setup_track);
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        app.update();

        let mut meshes = app.world_mut().query::<&Handle<Mesh>>();
        let world = app.world();
        // The flat quad renderer is gone: no mesh entities survive setup.
        assert_eq!(
            meshes.iter(world).len(),
            0,
            "flat quad path must be removed"
        );

        // One baked canvas quad (the sample circuit ships no scenery).
        let mut sprites = app
            .world_mut()
            .query::<(&Handle<Image>, &Sprite, &Transform)>();
        let world = app.world();
        let baked: Vec<(&Handle<Image>, &Sprite, &Transform)> = sprites.iter(world).collect();
        assert_eq!(baked.len(), 1, "want exactly the bake quad");
        let (handle, sprite, transform) = baked[0];
        assert_eq!(transform.translation.z, world_z::BAKE, "bake below scenery");
        let size = Vec2::new(canvas.width as f32, canvas.height as f32) / TEXELS_PER_UNIT;
        assert_eq!(sprite.custom_size, Some(size), "bake spans the canvas");
        let center = canvas.world_origin() + size * 0.5;
        assert_eq!(transform.translation.x, center.x);
        assert_eq!(transform.translation.y, center.y);
        let images = world.resource::<Assets<Image>>();
        let image = images.get(handle).expect("bake texture uploaded");
        assert_eq!(image.width(), canvas.width);
        assert_eq!(image.height(), canvas.height);
        let ImageSampler::Descriptor(desc) = &image.sampler else {
            panic!("bake texture must pin its sampler, got {:?}", image.sampler);
        };
        assert!(
            matches!(desc.mag_filter, ImageFilterMode::Nearest),
            "crisp texels, no linear smear"
        );
        assert!(matches!(desc.min_filter, ImageFilterMode::Nearest));
    }

    /// Scenery mounts above the bake so the FX layers have a defined place.
    #[test]
    fn setup_track_mounts_scenery_above_the_bake() {
        let track = Track::parse(
            r#"{"name":"Props","width":12,
                "points":[[0,0],[100,0],[100,60],[0,60],[0,0]],"surfaces":[],
                "props":[{"type":"tree","position":[50,40]}]}"#,
        )
        .unwrap();
        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<Image>();
        app.insert_resource(ShellSimulation::new(track));
        app.add_systems(Startup, setup_track);
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        app.update();

        let mut sprites = app.world_mut().query::<(&Sprite, &Transform)>();
        let world = app.world();
        let mut bake = 0u32;
        let mut scenery = 0u32;
        for (sprite, transform) in sprites.iter(world) {
            if transform.translation.z == world_z::BAKE {
                bake += 1;
            } else if transform.translation.z == world_z::SCENERY {
                assert_eq!(
                    sprite.custom_size,
                    Some(Vec2::new(3.0, 4.0)),
                    "tree at native sprite density"
                );
                scenery += 1;
            } else {
                panic!("unexpected z {}", transform.translation.z);
            }
        }
        assert_eq!((bake, scenery), (1, 1), "one bake quad, one tree");
    }
}

/// Keep the menus and labels inside the supported 640x360 minimum window.
fn scale_ui_to_window(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
) {
    if let Ok(window) = windows.get_single() {
        scale.0 = (window.width() / 960.0)
            .min(window.height() / 600.0)
            .clamp(0.5, 1.0);
    }
}
