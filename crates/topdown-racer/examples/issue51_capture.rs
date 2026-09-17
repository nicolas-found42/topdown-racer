//! Scripted native surface verification, not a human preference test.
use bevy::{
    prelude::*, render::view::window::screenshot::ScreenshotManager, time::TimeUpdateStrategy,
    window::PrimaryWindow,
};
use topdown_racer::{controls::SteeringResponse, AppState, RacerGamePlugin, ShellSimulation};

#[derive(Resource, Default)]
struct Frame(u32);

fn main() {
    std::fs::create_dir_all(".scratch/issue-51").unwrap();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Issue 51 steering verification".into(),
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
        .add_systems(
            First,
            |mut focus: ResMut<Events<bevy::window::WindowFocused>>| focus.clear(),
        )
        .add_systems(
            PreUpdate,
            inject
                .after(bevy::input::InputSystem)
                .before(topdown_racer::toggle_autopilot_system),
        )
        .add_systems(Last, observe)
        .run();
}

fn inject(mut frame: ResMut<Frame>, mut keys: ResMut<ButtonInput<KeyCode>>) {
    frame.0 += 1;
    keys.reset_all();
    let held: &[KeyCode] = match frame.0 {
        100 => &[KeyCode::KeyF],
        160 => &[KeyCode::Enter],
        370..=450 => &[KeyCode::KeyW],
        451..=458 => &[KeyCode::KeyW, KeyCode::KeyA],
        459..=480 => &[KeyCode::KeyW],
        490 => &[KeyCode::Escape],
        520 => &[KeyCode::KeyR],
        _ => &[],
    };
    for &key in held {
        keys.press(key);
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
    let capture = match frame.0 {
        80 => Some("raw-menu"),
        140 => {
            assert_eq!(shell.steering_response, SteeringResponse::Smooth);
            Some("smooth-menu")
        }
        458 => {
            assert_eq!(shell.curr_snapshots[0].steer, 1.0);
            Some("steering")
        }
        463 => {
            assert_eq!(shell.curr_snapshots[0].steer, 0.0);
            Some("release")
        }
        540 => {
            assert_eq!(*state.get(), AppState::Race);
            assert_eq!(shell.steering_response, SteeringResponse::Smooth);
            assert_eq!(shell.sim.racing_ticks(), 0);
            assert_eq!(shell.curr_snapshots[0].steer, 0.0);
            Some("restart")
        }
        _ => None,
    };
    if (451..=463).contains(&frame.0) {
        println!(
            "STEERING frame={} applied={}",
            frame.0, shell.curr_snapshots[0].steer
        );
    }
    if let Some(name) = capture {
        screenshots
            .save_screenshot_to_disk(windows.single(), format!(".scratch/issue-51/{name}.png"))
            .unwrap();
        println!(
            "CAPTURE {name} policy={:?} applied={}",
            shell.steering_response, shell.curr_snapshots[0].steer
        );
    }
    if frame.0 == 620 {
        exit.send(bevy::app::AppExit::Success);
    }
}
