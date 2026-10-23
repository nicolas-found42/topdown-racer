//! Deterministic native captures of real physics and per-wheel rendering.
//! Run with `ordinary` or `handbrake`; output is retained as review evidence.
use bevy::{
    prelude::*, render::view::window::screenshot::ScreenshotManager, window::PrimaryWindow,
};
use topdown_racer::{world_z, AppState, PlayerInput, RacerGamePlugin, ShellSimulation};
use topdown_racer_core::{
    simulation::{CarInput, GridCar, Sim, FIXED_DT},
    track::{Track, HILLSIDE_CIRCUIT},
};
#[derive(Resource)]
struct Probe {
    kind: String,
    ticks: u32,
    captured: bool,
    path: String,
    slipping: [u32; 4],
}
fn main() {
    let kind = std::env::args().nth(1).expect("ordinary or handbrake");
    assert!(matches!(kind.as_str(), "ordinary" | "handbrake"));
    let path = format!("docs/reviews/issue-62/{kind}.png");
    std::fs::create_dir_all("docs/reviews/issue-62").unwrap();
    let _ = std::fs::remove_file(&path);
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: format!("Issue 62: {kind}"),
                        resolution: (1280_f32, 720_f32).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(RacerGamePlugin)
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(FIXED_DT as f64),
        ))
        .insert_resource(bevy::winit::WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::Continuous,
        })
        .insert_resource(Probe {
            kind,
            ticks: 0,
            captured: false,
            path,
            slipping: [0; 4],
        })
        .add_systems(Startup, topdown_racer::auto_start_race)
        .add_systems(
            First,
            |mut events: ResMut<Events<bevy::window::WindowFocused>>| events.clear(),
        )
        .add_systems(FixedFirst, drive.run_if(in_state(AppState::Race)))
        .add_systems(PostUpdate, capture.run_if(in_state(AppState::Race)))
        .run();
}
fn drive(
    mut probe: ResMut<Probe>,
    mut shell: ResMut<ShellSimulation>,
    mut input: ResMut<PlayerInput>,
) {
    if probe.ticks == 0 {
        shell.sim = Sim::from_grid(
            Track::parse(HILLSIDE_CIRCUIT).unwrap(),
            &[GridCar {
                pose: Vec2::new(65., 0.),
                heading: 0.,
                velocity: Vec2::new(15., 0.),
            }],
        );
        shell.curr_snapshots = shell.sim.snapshots();
        shell.prev_snapshots = shell.curr_snapshots.clone();
        shell.generation += 1;
    }
    probe.ticks += 1;
    input.0 = CarInput {
        steer: if probe.kind == "handbrake" { 0.0 } else { 0.1 },
        handbrake: probe.kind == "handbrake" && probe.ticks <= 90,
        ..default()
    };
}
fn capture(
    mut probe: ResMut<Probe>,
    shell: Res<ShellSimulation>,
    mut manager: ResMut<ScreenshotManager>,
    windows: Query<Entity, With<PrimaryWindow>>,
    decals: Query<(&Visibility, &Transform)>,
    mut exit: EventWriter<bevy::app::AppExit>,
    mut texts: Query<&mut Text>,
) {
    for mut text in &mut texts {
        for section in &mut text.sections {
            if section.value == "GO!" {
                section.value.clear();
            }
        }
    }
    let s = shell.curr_snapshots[0];
    for i in 0..4 {
        probe.slipping[i] += s.wheels[i].beyond_limit as u32;
    }
    if probe.ticks >= 120 && !probe.captured {
        let marks = decals
            .iter()
            .filter(|(visible, tf)| {
                **visible == Visibility::Visible && (tf.translation.z - world_z::SKID).abs() < 0.001
            })
            .count();
        if probe.kind == "ordinary" {
            assert_eq!(marks, 0);
            assert_eq!(probe.slipping, [0; 4]);
        } else {
            assert!(marks > 0);
            assert_eq!(&probe.slipping[..2], &[0, 0]);
            assert!(probe.slipping[2] > 0 && probe.slipping[3] > 0);
        }
        println!("{} ticks={} marks={marks} wheel_slip_ticks={:?} speed={} steering_angle={} rpm={} gear={}",probe.kind,probe.ticks,probe.slipping,s.forward_speed,s.steering_angle,s.engine_rpm,s.gear);
        manager
            .save_screenshot_to_disk(windows.single(), &probe.path)
            .unwrap();
        probe.captured = true;
    }
    if probe.captured
        && probe.ticks >= 180
        && std::fs::metadata(&probe.path).is_ok_and(|file| file.len() > 0)
    {
        exit.send(bevy::app::AppExit::Success);
    }
    assert!(probe.ticks < 600, "capture was not written");
}
