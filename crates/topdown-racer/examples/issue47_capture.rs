use bevy::{
    prelude::*, render::view::window::screenshot::ScreenshotManager, window::PrimaryWindow,
};
use topdown_racer::{AppState, RacerGamePlugin, ShellSimulation};
use topdown_racer_core::{
    simulation::{GridCar, Sim},
    track::{Surface, Track, HILLSIDE_CIRCUIT},
};

#[derive(Resource)]
struct Probe {
    kind: String,
    initialized: bool,
    requested: bool,
    frames: u32,
}

fn main() {
    let kind = std::env::args().nth(1).unwrap();
    let mut track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
    if kind == "gravel" {
        track.surfaces.fill(Surface::Gravel);
    }
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: format!("Issue 47 — {kind}"),
                        resolution: (1280_f32, 720_f32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(RacerGamePlugin)
        .insert_resource(bevy::winit::WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::Continuous,
        })
        .insert_resource(ShellSimulation::new(track))
        .insert_resource(Probe {
            kind,
            initialized: false,
            requested: false,
            frames: 0,
        })
        .add_systems(Startup, |mut state: ResMut<NextState<AppState>>| {
            state.set(AppState::Race)
        })
        .add_systems(Update, probe.run_if(in_state(AppState::Race)))
        .add_systems(
            First,
            |mut events: ResMut<Events<bevy::window::WindowFocused>>| events.clear(),
        )
        .run();
}

fn probe(
    mut probe: ResMut<Probe>,
    mut shell: ResMut<ShellSimulation>,
    mut manager: ResMut<ScreenshotManager>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut exit: EventWriter<bevy::app::AppExit>,
) {
    if !probe.initialized {
        let y = if probe.kind == "grass" { 15.0 } else { 0.0 };
        let speed = match probe.kind.as_str() {
            "road-fast" => 40.0,
            "road-slow" => 12.0,
            _ => 12.0,
        };
        shell.sim = Sim::from_grid(
            shell.sim.track().clone(),
            &[
                GridCar {
                    pose: Vec2::new(85.0, y),
                    heading: 0.0,
                    velocity: Vec2::new(speed, 0.0),
                },
                GridCar {
                    pose: Vec2::new(75.0, y + 5.0),
                    heading: 0.0,
                    velocity: Vec2::new(speed, 0.0),
                },
            ],
        );
        shell.curr_snapshots = shell.sim.snapshots();
        shell.prev_snapshots = shell.curr_snapshots.clone();
        probe.initialized = true;
    }
    if !probe.requested && shell.sim.racing_ticks() >= 16 {
        println!(
            "CAPTURE {} tick={} cars={:?}",
            probe.kind,
            shell.sim.racing_ticks(),
            shell
                .curr_snapshots
                .iter()
                .map(|c| (c.pose, c.surface, c.velocity.length()))
                .collect::<Vec<_>>()
        );
        // Let the camera, particles, and assets settle for a rendered frame.
        probe.frames += 1;
        if probe.frames == 10 {
            manager
                .save_screenshot_to_disk(
                    windows.single(),
                    format!(".scratch/issue-47-captures/{}.png", probe.kind),
                )
                .unwrap();
            probe.requested = true;
            probe.frames = 0;
        }
    }
    if probe.requested {
        probe.frames += 1;
        if probe.frames > 600 {
            exit.send(bevy::app::AppExit::Success);
        }
    }
}
