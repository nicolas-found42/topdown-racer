//! Native scripted digital Manual approaches, not human driving or automated coaching.
use bevy::{
    prelude::*, render::view::window::screenshot::ScreenshotManager, time::TimeUpdateStrategy,
    window::PrimaryWindow,
};
use topdown_racer::{
    records::{LocalRecords, PracticeRecord},
    AppState, RacerGamePlugin, ShellSimulation,
};
use topdown_racer_core::{
    ai::{AiDriver, AiView},
    practice::PracticeStatus,
};

#[derive(Resource)]
struct Probe {
    frame: u32,
    stage: usize,
    age: u32,
    driver: AiDriver,
    results: Vec<PracticeRecord>,
    captured: bool,
}
fn main() {
    std::fs::create_dir_all("docs/reviews/issue-57").unwrap();
    let path = std::path::PathBuf::from(".scratch/issue-57-records.json");
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Issue 57 Corner Practice".into(),
                        resolution: (1280.0_f32, 720.0_f32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(RacerGamePlugin)
        .insert_resource(LocalRecords::from_path(path))
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 64.0),
        ))
        .insert_resource(Probe {
            frame: 0,
            stage: 0,
            age: 0,
            driver: AiDriver::new(0),
            results: vec![],
            captured: false,
        })
        .insert_resource(bevy::winit::WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::Continuous,
        })
        .add_systems(
            First,
            |mut events: ResMut<Events<bevy::window::WindowFocused>>| events.clear(),
        )
        .add_systems(
            PreUpdate,
            drive
                .after(bevy::input::InputSystem)
                .before(topdown_racer::toggle_autopilot_system),
        )
        .add_systems(Last, observe)
        .run();
}
fn drive(
    mut probe: ResMut<Probe>,
    shell: Res<ShellSimulation>,
    state: Res<State<AppState>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
) {
    probe.frame += 1;
    keys.reset_all();
    if probe.frame == 60 {
        keys.press(KeyCode::KeyP);
    }
    if probe.stage == 4 {
        if probe.age == 5 {
            keys.press(KeyCode::Enter);
        }
        return;
    }
    if *state.get() == AppState::Results {
        probe.age += 1;
        if probe.age == 40 {
            if probe.stage < 3 {
                keys.press(KeyCode::KeyR);
                probe.driver = AiDriver::new(0);
            } else {
                keys.press(KeyCode::Escape);
                probe.stage = 4;
                probe.age = 0;
            }
        }
        return;
    }
    if *state.get() != AppState::Race {
        return;
    }
    probe.age = 0;
    probe.captured = false;
    if shell.sim.preparation_ticks() > 0 {
        return;
    }
    let car = shell.curr_snapshots[0];
    let field = [(car.pose, car.velocity)];
    let mut input = probe.driver.compute_input(AiView {
        active: None,
        car_index: 0,
        pose: car.pose,
        heading: car.heading,
        velocity: car.velocity,
        track: shell.sim.track(),
        field: &field,
    });
    if probe.stage > 0 && car.pose.x < 136.0 && car.pose.y < 8.0 {
        input.throttle = 1.0;
        input.brake = 0.0;
    }
    if input.throttle > 0.0 {
        keys.press(KeyCode::KeyW);
    }
    if input.brake > 0.0 {
        keys.press(KeyCode::KeyS);
    }
    if input.steer > 0.05 {
        keys.press(KeyCode::KeyA);
    }
    if input.steer < -0.05 {
        keys.press(KeyCode::KeyD);
    }
}
fn observe(
    mut probe: ResMut<Probe>,
    shell: Res<ShellSimulation>,
    state: Res<State<AppState>>,
    mut screenshots: ResMut<ScreenshotManager>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut exit: EventWriter<bevy::app::AppExit>,
) {
    let mut capture = None;
    if probe.frame == 40 {
        capture = Some("menu".to_owned());
    }
    if probe.frame == 80 {
        capture = Some("preparation".to_owned());
    }
    if probe.frame == 250 {
        capture = Some("gates".to_owned());
    }
    if *state.get() == AppState::Results && !probe.captured {
        let PracticeStatus::Finished {
            seconds,
            exit_speed,
        } = shell.practice.as_ref().unwrap().status()
        else {
            panic!(
                "attempt failed: {:?}",
                shell.practice.as_ref().unwrap().status()
            );
        };
        let result = PracticeRecord {
            seconds,
            exit_speed,
        };
        println!(
            "RESULT approach={} seconds={seconds:.6} exit_speed={exit_speed:.6} baseline={:?}",
            probe.stage, shell.practice_baseline
        );
        if probe.stage == 2 {
            assert_eq!(
                result, probe.results[1],
                "rapid retry must repeat both measurements"
            );
        }
        probe.results.push(result);
        capture = Some(["tidy-result", "late-result", "retry-result"][probe.stage].to_owned());
        probe.stage += 1;
        probe.captured = true;
    }
    if probe.stage == 4 {
        probe.age += 1;
        if probe.age == 30 {
            assert_eq!(*state.get(), AppState::Race);
            assert_eq!(shell.curr_snapshots.len(), 4);
            assert!(shell.practice.is_none());
            capture = Some("race-restored".to_owned());
        }
        if probe.age == 60 {
            exit.send(bevy::app::AppExit::Success);
        }
    }
    if let Some(name) = capture {
        screenshots
            .save_screenshot_to_disk(
                windows.single(),
                format!("docs/reviews/issue-57/{name}.png"),
            )
            .unwrap();
        println!("CAPTURE {name}");
    }
    assert!(probe.frame < 6000, "scenario timeout");
}
