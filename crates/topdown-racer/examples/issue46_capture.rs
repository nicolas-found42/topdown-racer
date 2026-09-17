use bevy::{
    prelude::*, render::view::window::screenshot::ScreenshotManager, window::PrimaryWindow,
};
use topdown_racer::{AppState, PlayerInput, RacerGamePlugin, ShellSimulation};
use topdown_racer_core::simulation::{CarInput, RacePhase};

#[derive(Resource, Default)]
struct Script {
    ticks: u32,
    pending: Option<&'static str>,
}

fn input(mut script: ResMut<Script>, shell: Res<ShellSimulation>, mut input: ResMut<PlayerInput>) {
    if shell.sim.phase() != RacePhase::Racing {
        return;
    }
    script.ticks += 1;
    input.0 = if script.ticks < 128 {
        CarInput {
            throttle: 1.0,
            ..default()
        }
    } else if script.ticks < 156 {
        CarInput::default()
    } else if script.ticks < 176 {
        CarInput {
            brake: 1.0,
            ..default()
        }
    } else {
        CarInput::default()
    };
    script.pending = match script.ticks {
        120 => Some("01-before"),
        140 => Some("02-brake-smoke"),
        156 => Some("03-released"),
        172 => Some("04-brake-lights"),
        _ => script.pending,
    };
}
fn capture(
    mut script: ResMut<Script>,
    mut manager: ResMut<ScreenshotManager>,
    windows: Query<Entity, With<PrimaryWindow>>,
    shell: Res<ShellSimulation>,
    mut exit: EventWriter<bevy::app::AppExit>,
) {
    if let Some(name) = script.pending.take() {
        println!(
            "CAPTURE {name} ticks={} echoes={:?}",
            script.ticks,
            shell
                .curr_snapshots
                .iter()
                .map(|c| (c.brake, c.handbrake, c.pose))
                .collect::<Vec<_>>()
        );
        manager
            .save_screenshot_to_disk(windows.single(), format!(".scratch/issue-46/{name}.png"))
            .unwrap();
    }
    if script.ticks > 1000 {
        exit.send(bevy::app::AppExit::Success);
    }
}
fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Issue 46 scripted capture".into(),
                        resolution: (1280.0_f32, 720.0_f32).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(bevy::winit::WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::Continuous,
        })
        .add_plugins(RacerGamePlugin)
        .init_resource::<Script>()
        .add_systems(
            PreUpdate,
            (|mut events: ResMut<Events<bevy::window::WindowFocused>>| events.clear())
                .before(bevy::input::InputSystem),
        )
        .add_systems(Startup, topdown_racer::auto_start_race)
        .add_systems(FixedFirst, input.run_if(in_state(AppState::Race)))
        .add_systems(PostUpdate, capture)
        .run();
}
