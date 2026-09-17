//! Scripted native presentation snapshots; not a human driving/listening session.
use bevy::{
    prelude::*, render::view::window::screenshot::ScreenshotManager, window::PrimaryWindow,
};
use topdown_racer::{AppState, PlayerInput, RacerGamePlugin, ShellSimulation};
use topdown_racer_core::{
    simulation::{CarInput, GridCar, Sim},
    track::{Track, HILLSIDE_CIRCUIT},
};

#[derive(Resource)]
struct Probe {
    kind: String,
    ticks: u32,
    captured: bool,
    sustained: bool,
}
fn main() {
    let kind = std::env::args()
        .nth(1)
        .expect("brake, drift, surface, impact, or combined");
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: format!("Issue 55 {kind}"),
                        resolution: (1280.0_f32, 720.0_f32).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(RacerGamePlugin)
        .insert_resource(bevy::winit::WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::Continuous,
        })
        .insert_resource(Probe {
            kind,
            ticks: 0,
            captured: false,
            sustained: false,
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
        let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
        let y = if probe.kind == "impact" {
            track.wall_distance_at(Vec2::new(82.0, 0.0)) - 0.1
        } else if probe.kind == "surface" || probe.kind == "combined" {
            14.0
        } else {
            0.0
        };
        let velocity = if probe.kind == "combined" {
            Vec2::new(30.0, 3.0)
        } else {
            Vec2::new(22.0, 0.0)
        };
        shell.sim = Sim::from_grid(
            track,
            &[
                GridCar {
                    pose: Vec2::new(82.0, y),
                    heading: if probe.kind == "impact" {
                        std::f32::consts::FRAC_PI_2
                    } else {
                        0.0
                    },
                    velocity: if probe.kind == "impact" {
                        Vec2::new(0.0, 22.0)
                    } else {
                        velocity
                    },
                },
                GridCar {
                    pose: Vec2::new(76.0, y - 3.0),
                    heading: 0.0,
                    velocity,
                },
                GridCar {
                    pose: Vec2::new(70.0, y + 1.0),
                    heading: 0.0,
                    velocity,
                },
                GridCar {
                    pose: Vec2::new(64.0, y - 2.0),
                    heading: 0.0,
                    velocity,
                },
            ],
        );
        shell.curr_snapshots = shell.sim.snapshots();
        shell.prev_snapshots = shell.curr_snapshots.clone();
    }
    probe.ticks += 1;
    input.0 = match probe.kind.as_str() {
        "brake" => CarInput {
            brake: 1.0,
            ..default()
        },
        "drift" | "combined" => CarInput {
            handbrake: true,
            steer: 0.5,
            throttle: 1.0,
            ..default()
        },
        "impact" => CarInput {
            steer: 1.0,
            throttle: 1.0,
            ..default()
        },
        _ => CarInput {
            throttle: 1.0,
            ..default()
        },
    };
}
fn capture(
    mut probe: ResMut<Probe>,
    shell: Res<ShellSimulation>,
    mut manager: ResMut<ScreenshotManager>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut exit: EventWriter<bevy::app::AppExit>,
    mut texts: Query<&mut Text>,
) {
    // The rolling fixture has no countdown; remove its otherwise stale GO label.
    for mut text in &mut texts {
        for section in &mut text.sections {
            if section.value == "GO!" {
                section.value.clear();
            }
        }
    }
    let car = shell.curr_snapshots[0];
    let ready = if probe.kind == "impact" {
        car.wall_contact
    } else {
        probe.ticks >= 24
    };
    if ready && !probe.captured {
        let directory = "docs/reviews/issue-55";
        std::fs::create_dir_all(directory).unwrap();
        println!(
            "CAPTURE {} tick={} brake={} handbrake={} drift={} surface={:?} wall={} speed={}",
            probe.kind,
            probe.ticks,
            car.brake,
            car.handbrake,
            car.drifting,
            car.surface,
            car.wall_contact,
            car.velocity.length()
        );
        manager
            .save_screenshot_to_disk(windows.single(), format!("{directory}/{}.png", probe.kind))
            .unwrap();
        probe.captured = true;
    }
    if probe.kind == "combined" && probe.ticks >= 80 && !probe.sustained {
        manager
            .save_screenshot_to_disk(
                windows.single(),
                "docs/reviews/issue-55/combined-sustained.png",
            )
            .unwrap();
        println!(
            "CAPTURE sustained tick={} handbrake={} surface={:?}",
            probe.ticks, car.handbrake, car.surface
        );
        probe.sustained = true;
    }
    if probe.ticks > 384 {
        exit.send(bevy::app::AppExit::Success);
    }
}
