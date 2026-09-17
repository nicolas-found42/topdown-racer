//! Native lifecycle exercise: real input events, fixed ticks, rendering and audio.
//! The stranded fixture is explicitly installed after the moving Race exercise.
use bevy::{
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
    render::view::window::screenshot::ScreenshotManager,
    time::TimeUpdateStrategy,
    window::{PrimaryWindow, WindowFocused},
};
use topdown_racer::{AppState, RacerGamePlugin, ShellSimulation};
use topdown_racer_core::simulation::{DrivingMode, GridCar, Sim};

#[derive(Resource, Default)]
struct Frame(u32);

fn main() {
    std::fs::create_dir_all("docs/reviews/issue-56").unwrap();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Issue 56 pause and recovery verification".into(),
                        resolution: (1280.0_f32, 720.0_f32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(RacerGamePlugin)
        .init_resource::<Frame>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 64.0),
        ))
        .insert_resource(bevy::winit::WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::Continuous,
        })
        .add_systems(First, inject)
        .add_systems(Last, observe)
        .run();
}

fn inject(
    mut frame: ResMut<Frame>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut keyboard: EventWriter<KeyboardInput>,
    mut focus: EventWriter<WindowFocused>,
    mut shell: ResMut<ShellSimulation>,
) {
    frame.0 += 1;
    let window = windows.single();
    let code = match frame.0 {
        80 => Some(KeyCode::KeyT),
        100 | 200 | 880 | 1040 | 1190 => Some(KeyCode::Enter),
        140 | 830 | 1140 | 1290 | 1380 => Some(KeyCode::Escape),
        1170 => Some(KeyCode::KeyC),
        1320 => Some(KeyCode::KeyR),
        1410 => Some(KeyCode::KeyM),
        _ => None,
    };
    if let Some(key_code) = code {
        for state in [
            bevy::input::ButtonState::Pressed,
            bevy::input::ButtonState::Released,
        ] {
            keyboard.send(KeyboardInput {
                key_code,
                logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                state,
                window,
            });
        }
    }
    if frame.0 == 970 || frame.0 == 1000 {
        focus.send(WindowFocused {
            window,
            focused: frame.0 == 1000,
        });
    }
    if frame.0 == 1120 {
        // Repeatable stranded Manual Car; rivals are clear of the recovery destination.
        let track = shell.sim.track().clone();
        let mut grid: Vec<_> = Sim::new(track.clone(), 4)
            .snapshots()
            .iter()
            .map(|car| GridCar {
                pose: car.pose,
                heading: car.heading,
                velocity: Vec2::ZERO,
            })
            .collect();
        grid[0] = GridCar {
            pose: Vec2::new(50.0, 30.0),
            heading: 2.0,
            velocity: Vec2::ZERO,
        };
        for (index, rival) in grid.iter_mut().enumerate().skip(1) {
            rival.pose = track.point_at_arc(200.0 + index as f32 * 10.0);
            let direction = track.centerline_frame(rival.pose).direction;
            rival.heading = direction.y.atan2(direction.x);
        }
        shell.sim = Sim::from_grid(track, &grid);
        shell.curr_snapshots = shell.sim.snapshots();
        shell.prev_snapshots = shell.curr_snapshots.clone();
        shell.generation += 1;
    }
}

fn observe(
    frame: Res<Frame>,
    shell: Res<ShellSimulation>,
    state: Res<State<AppState>>,
    mut screenshots: ResMut<ScreenshotManager>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut exit: EventWriter<bevy::app::AppExit>,
) {
    let name = match frame.0 {
        60 => Some("menu-controls"),
        170 => {
            assert!(shell.sim.is_paused());
            assert_eq!(shell.sim.racing_ticks(), 0);
            Some("countdown-pause")
        }
        220 => {
            assert!(shell.sim.preparation_ticks() > 0);
            assert_eq!(shell.sim.racing_ticks(), 0);
            Some("resume-preparation")
        }
        860 => {
            assert!(shell.sim.is_paused());
            assert!(shell.curr_snapshots[0].velocity.length() > 5.0);
            assert!(
                shell.curr_snapshots[0].pose.y > 1.0,
                "Car must be inside the bend"
            );
            Some("mid-corner-pause")
        }
        1020 => {
            assert!(shell.sim.is_paused());
            Some("focus-restored-paused")
        }
        1180 => {
            assert!(shell.sim.is_paused());
            assert!(shell.curr_snapshots[0].current_lap_invalidated);
            assert_eq!(shell.curr_snapshots[0].velocity, Vec2::ZERO);
            Some("recovered")
        }
        1350 => {
            assert_eq!(shell.sim.racing_ticks(), 0);
            assert!(!shell.curr_snapshots[0].current_lap_invalidated);
            assert_eq!(shell.sim.player_mode(), DrivingMode::Manual);
            Some("restart")
        }
        1440 => {
            assert_eq!(*state.get(), AppState::Menu);
            Some("returned-menu")
        }
        _ => None,
    };
    if frame.0 == 950 {
        assert!(
            !shell.sim.is_paused(),
            "mid-corner Resume must be deliberate"
        );
        assert_eq!(shell.sim.preparation_ticks(), 0);
    }
    if let Some(name) = name {
        screenshots
            .save_screenshot_to_disk(
                windows.single(),
                format!("docs/reviews/issue-56/{name}.png"),
            )
            .unwrap();
        println!(
            "CAPTURE {name}: phase={:?} ticks={} pose={:?} paused={} preparation={}",
            shell.sim.phase(),
            shell.sim.racing_ticks(),
            shell.curr_snapshots[0].pose,
            shell.sim.is_paused(),
            shell.sim.preparation_ticks()
        );
    }
    if frame.0 == 1850 {
        exit.send(bevy::app::AppExit::Success);
    }
}
