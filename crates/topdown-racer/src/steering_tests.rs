//! Exercise the real keyboard → fixed tick → snapshot boundary without rendering.
use super::*;
use bevy::{state::app::StatesPlugin, time::TimeUpdateStrategy};
use std::time::Duration;
use topdown_racer_core::{simulation::DrivingMode, track::SAMPLE_CIRCUIT};

fn controls_app() -> App {
    let mut app = App::new();
    let mut shell = ShellSimulation::from_sim(Sim::new(Track::parse(SAMPLE_CIRCUIT).unwrap(), 1));
    shell.steering_response = controls::SteeringResponse::Smooth;
    app.insert_resource(shell)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<PlayerInput>()
        .init_resource::<ControlGate>()
        .init_resource::<controls::SteeringFilter>()
        .add_systems(
            PreUpdate,
            (toggle_autopilot_system, read_keyboard_input).chain(),
        )
        .add_systems(FixedUpdate, step_simulation);
    app
}

fn keys(app: &mut App, pressed: &[KeyCode]) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    for &key in pressed {
        keys.press(key);
    }
}

fn tick(app: &mut App) -> CarSnapshot {
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world().resource::<ShellSimulation>().curr_snapshots[0]
}

#[test]
fn steering_echo_releases_reverses_and_opposing_keys_cancel_immediately() {
    let mut app = controls_app();
    keys(&mut app, &[KeyCode::KeyA]);
    for expected in [0.15625, 0.3125, 0.46875, 0.625, 0.78125, 0.9375, 1.0] {
        assert_eq!(tick(&mut app).steer, expected);
    }
    keys(&mut app, &[]);
    for expected in [0.6875, 0.375, 0.0625, 0.0] {
        assert_eq!(tick(&mut app).steer, expected);
    }
    keys(&mut app, &[KeyCode::KeyA]);
    for _ in 0..7 {
        tick(&mut app);
    }
    keys(&mut app, &[KeyCode::KeyD]);
    for expected in [0.6875, 0.375, 0.0625, -0.25, -0.40625] {
        assert_eq!(tick(&mut app).steer, expected);
    }
    // Mixed aliases must cancel too, rather than choosing the last key or
    // continuing to return slowly from the previous applied steering.
    keys(&mut app, &[KeyCode::KeyD, KeyCode::ArrowLeft]);
    assert_eq!(tick(&mut app).steer, 0.0);
    keys(&mut app, &[KeyCode::ArrowLeft]);
    assert_eq!(tick(&mut app).steer, 0.15625);
}

#[derive(Resource, Default)]
struct Trace(Vec<CarSnapshot>);

fn record_tick(shell: Res<ShellSimulation>, mut trace: ResMut<Trace>) {
    trace.0.push(shell.curr_snapshots[0]);
}

fn cadence_trace(frame: Duration, frames_per_input: usize) -> Vec<CarSnapshot> {
    let mut app = controls_app();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(64.0))
        .insert_resource(TimeUpdateStrategy::ManualDuration(frame))
        .init_resource::<Trace>()
        .add_systems(FixedUpdate, record_tick.after(step_simulation));
    app.update(); // Initialize the clock before supplying the fixed-tick stream.
    for pressed in [
        vec![KeyCode::KeyW, KeyCode::KeyA],
        vec![KeyCode::KeyW],
        vec![KeyCode::KeyW, KeyCode::KeyD],
        vec![KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyD],
    ] {
        keys(&mut app, &pressed);
        for _ in 0..frames_per_input {
            app.update();
        }
    }
    app.world_mut().remove_resource::<Trace>().unwrap().0
}

#[test]
fn effective_commands_replay_identically_across_render_cadences() {
    // Four quarter-second input intervals: both zero-tick and multi-tick
    // rendered frames exercise the actual Bevy fixed-loop accumulator.
    let fast = cadence_trace(Duration::from_secs_f64(1.0 / 256.0), 64);
    let slow = cadence_trace(Duration::from_secs_f64(1.0 / 16.0), 4);
    assert_eq!(fast.len(), 64);
    assert_eq!(fast, slow);
    let mut replay = Sim::new(Track::parse(SAMPLE_CIRCUIT).unwrap(), 1);
    for expected in fast {
        let actual = replay.tick(&[CarInput {
            throttle: expected.throttle,
            brake: expected.brake,
            steer: expected.steer,
            handbrake: expected.handbrake,
        }])[0];
        assert_eq!(actual, expected);
    }
}

#[test]
fn autopilot_and_ai_opponents_bypass_human_steering_response() {
    let run = |response| {
        let mut app = controls_app();
        let mut shell =
            ShellSimulation::from_sim(Sim::new_race(Track::parse(SAMPLE_CIRCUIT).unwrap(), 4));
        shell.steering_response = response;
        shell.sim.request_player_mode(DrivingMode::Autopilot);
        app.insert_resource(shell);
        keys(&mut app, &[KeyCode::KeyA]);
        let mut trace = Vec::new();
        for _ in 0..600 {
            tick(&mut app);
            trace.push(
                app.world()
                    .resource::<ShellSimulation>()
                    .curr_snapshots
                    .clone(),
            );
        }
        trace
    };
    assert_eq!(
        run(controls::SteeringResponse::Raw),
        run(controls::SteeringResponse::Smooth)
    );
}

#[test]
fn ownership_switch_discards_smoothing_and_requires_physical_release() {
    let mut app = controls_app();
    keys(&mut app, &[KeyCode::KeyA]);
    for _ in 0..7 {
        tick(&mut app);
    }
    keys(&mut app, &[KeyCode::KeyA, KeyCode::KeyT]);
    tick(&mut app);
    keys(&mut app, &[KeyCode::KeyA, KeyCode::KeyT]);
    assert_eq!(tick(&mut app).steer, 0.0);
    assert_eq!(tick(&mut app).steer, 0.0);
    keys(&mut app, &[]);
    assert_eq!(tick(&mut app).steer, 0.0);
    keys(&mut app, &[KeyCode::KeyA]);
    assert_eq!(tick(&mut app).steer, 0.15625);
}

#[test]
fn menu_selection_focus_loss_and_restart_preserve_policy_not_filter_history() {
    let mut app = controls_app();
    app.add_plugins((StatesPlugin, RaceLifecyclePlugin));
    app.update();
    app.world_mut()
        .resource_mut::<ShellSimulation>()
        .steering_response = controls::SteeringResponse::Raw;
    keys(&mut app, &[KeyCode::KeyF, KeyCode::Enter]);
    app.update();
    keys(&mut app, &[]);
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Race
    );
    assert_eq!(
        app.world().resource::<ShellSimulation>().steering_response,
        controls::SteeringResponse::Smooth
    );
    for _ in 0..200 {
        tick(&mut app);
    }
    keys(&mut app, &[KeyCode::KeyA]);
    for _ in 0..7 {
        tick(&mut app);
    }
    assert_eq!(
        app.world().resource::<ShellSimulation>().curr_snapshots[0].steer,
        1.0
    );
    let before = app.world().resource::<ShellSimulation>().sim.racing_ticks();
    app.world_mut().send_event(bevy::window::WindowFocused {
        window: Entity::PLACEHOLDER,
        focused: false,
    });
    tick(&mut app);
    assert!(app.world().resource::<ShellSimulation>().sim.is_paused());
    assert_eq!(
        app.world().resource::<ShellSimulation>().sim.racing_ticks(),
        before
    );
    keys(&mut app, &[KeyCode::KeyR]);
    tick(&mut app);
    assert_eq!(
        app.world().resource::<ShellSimulation>().sim.racing_ticks(),
        0
    );
    assert_eq!(
        app.world().resource::<ShellSimulation>().steering_response,
        controls::SteeringResponse::Smooth
    );
    keys(&mut app, &[]);
    for _ in 0..200 {
        tick(&mut app);
    }
    keys(&mut app, &[KeyCode::KeyA]);
    assert_eq!(tick(&mut app).steer, 0.15625);
}
