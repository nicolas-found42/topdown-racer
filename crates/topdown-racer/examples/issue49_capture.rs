use bevy::{
    prelude::*, render::view::window::screenshot::ScreenshotManager, window::PrimaryWindow,
};
use topdown_racer::{AppState, RacerGamePlugin, ShellSimulation};
use topdown_racer_core::track::{Track, HILLSIDE_CIRCUIT};

#[derive(Resource)]
struct Probe {
    ticks: u32,
    captured: bool,
}

fn main() {
    let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Issue 49 — Asset Root Verification".into(),
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
            ticks: 0,
            captured: false,
        })
        .add_systems(Startup, |mut state: ResMut<NextState<AppState>>| {
            state.set(AppState::Race)
        })
        .add_systems(Update, probe.run_if(in_state(AppState::Race)))
        .run();
}

fn probe(
    mut probe: ResMut<Probe>,
    main_window: Query<Entity, With<PrimaryWindow>>,
    mut screenshot_manager: ResMut<ScreenshotManager>,
    mut app_exit: EventWriter<AppExit>,
) {
    probe.ticks += 1;
    if probe.ticks == 30 && !probe.captured {
        let window = main_window.single();
        let path = ".scratch/issue-49-computer-use/31-car-sprite-verified.png";
        screenshot_manager
            .save_screenshot_to_disk(window, path)
            .unwrap();
        probe.captured = true;
        probe.ticks = 0;
    }

    if probe.captured {
        probe.ticks += 1;
        if probe.ticks > 300 {
            app_exit.send(AppExit::Success);
        }
    }
}
