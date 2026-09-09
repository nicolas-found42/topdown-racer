//! Race lifecycle: the shell's screen state machine (Menu → Race → Results),
//! the transition systems that write `NextState<AppState>`, and the restart
//! semantics. Screens consume the state and never write it; every transition
//! lives here and is tested through the state machine itself.

use bevy::prelude::*;

use crate::menu::StartButton;
use crate::results::should_show_results;
use crate::ShellSimulation;

/// Installs the screen state machine: state init and every system that
/// writes `NextState<AppState>` or rebuilds a race on entry.
///
/// Interface requirement: the app must have Bevy's `StatesPlugin` installed
/// (any App built with `DefaultPlugins` has it). Headless test harnesses add
/// it explicitly.
pub struct RaceLifecyclePlugin;

/// Shell-level screen state: menu, an active race, or the results screen.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Menu,
    Race,
    Results,
}

impl Plugin for RaceLifecyclePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .init_resource::<crate::ControlGate>()
            .init_resource::<crate::PlayerInput>()
            .add_systems(OnEnter(AppState::Race), reset_race_on_enter)
            .add_systems(
                Update,
                (
                    (crate::menu::select_pace_system, menu_action_system).chain().run_if(in_state(AppState::Menu)),
                    esc_to_menu_system.run_if(in_state(AppState::Race)),
                    detect_race_finish.run_if(in_state(AppState::Race)),
                    results_action_system.run_if(in_state(AppState::Results)),
                ),
            );
    }
}

/// Skips the menu when TOPDOWN_AUTO_START=1 by transitioning into the Race
/// state on the first frame, so OnEnter(Race) systems (HUD show, race reset)
/// still run.
pub fn auto_start_race(mut next_state: ResMut<NextState<AppState>>) {
    next_state.set(AppState::Race);
}

/// Rebuilds a fresh countdown race whenever entering the Race state.
pub fn reset_race_on_enter(mut shell: ResMut<ShellSimulation>, mut gate: ResMut<crate::ControlGate>, mut input: ResMut<crate::PlayerInput>) {
    shell.reset_to_fresh_race();
    gate.0 = true;
    input.0 = Default::default();
}

/// Starts the race from the menu via the start button or the Enter key.
pub fn menu_action_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    button_q: Query<&Interaction, (Changed<Interaction>, With<StartButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let button_clicked = button_q
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    if button_clicked || keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(AppState::Race);
    }
}

/// Returns cleanly to the menu when ESC is pressed during a race.
pub fn esc_to_menu_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Menu);
    }
}

/// Transitions to the results screen exactly when the race finishes.
pub fn detect_race_finish(
    shell: Res<ShellSimulation>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if should_show_results(shell.sim.phase()) {
        next_state.set(AppState::Results);
    }
}

/// Instant-restarts a fresh race (R / Enter) or returns to the menu (ESC).
pub fn results_action_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) || keyboard.just_pressed(KeyCode::Enter) {
        // OnEnter(Race) rebuilds the fresh countdown race; no menu pass-through.
        next_state.set(AppState::Race);
    } else if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Menu);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use topdown_racer_core::simulation::{RacePhase, Sim, DEFAULT_COUNTDOWN_TICKS};
    use topdown_racer_core::track::{Track, SAMPLE_CIRCUIT};

    /// A minimal app running just the lifecycle state machine against a real
    /// shell simulation, the same harness pattern the keyboard and headless
    /// audio tests use.
    fn lifecycle_app(track: Track) -> App {
        let mut app = App::new();
        app.add_plugins((bevy::state::app::StatesPlugin, RaceLifecyclePlugin))
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(ShellSimulation::new(track));
        app
    }

    /// An app in the Race state whose shell simulation has already finished,
    /// entered through the real Menu → Race transition (which runs the
    /// OnEnter reset) before substituting a Finished race.
    fn finished_shell_app() -> App {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut app = App::new();
        app.add_plugins((bevy::state::app::StatesPlugin, RaceLifecyclePlugin))
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(ShellSimulation::new(track.clone()));
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Race);
        app.update();

        let sim = Sim::new_with_phase(track, 4, RacePhase::Finished);
        app.insert_resource(ShellSimulation {
            selected_pace: Default::default(),
            curr_snapshots: sim.snapshots(),
            prev_snapshots: sim.snapshots(),
            sim,
        });
        app
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }

    fn state(app: &mut App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    /// Settles one transition: one update lets a system write NextState, the
    /// next applies it through StateTransition (running OnEnter systems).
    fn settle(app: &mut App) {
        app.update();
        app.update();
    }

    #[test]
    fn menu_pace_selection_is_fixed_after_start_and_survives_restart() {
        use topdown_racer_core::ai::OpponentPace;
        let mut app = lifecycle_app(Track::parse(SAMPLE_CIRCUIT).unwrap());
        press(&mut app, KeyCode::Digit1);
        settle(&mut app);
        press(&mut app, KeyCode::Enter);
        settle(&mut app);
        assert_eq!(app.world().resource::<ShellSimulation>().sim.opponent_pace(), OpponentPace::Touring);
        press(&mut app, KeyCode::Digit3);
        settle(&mut app);
        assert_eq!(app.world().resource::<ShellSimulation>().sim.opponent_pace(), OpponentPace::Touring);
        app.world_mut().resource_mut::<ShellSimulation>().reset_to_fresh_race();
        assert_eq!(app.world().resource::<ShellSimulation>().sim.opponent_pace(), OpponentPace::Touring);
    }

    #[test]
    fn menu_enter_transitions_to_race_and_rebuilds_fresh_countdown() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut app = lifecycle_app(track);

        // Drive the shell sim deep into an in-flight race so the reset on
        // entry is observable.
        for _ in 0..DEFAULT_COUNTDOWN_TICKS + 50 {
            app.world_mut()
                .resource_mut::<ShellSimulation>()
                .sim
                .tick(&[
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    Default::default(),
                ]);
        }
        let before = app.world().resource::<ShellSimulation>();
        assert_eq!(before.sim.phase(), RacePhase::Racing);
        assert!(before.sim.racing_ticks() > 0);

        press(&mut app, KeyCode::Enter);
        settle(&mut app);

        assert_eq!(state(&mut app), AppState::Race);
        let shell = app.world().resource::<ShellSimulation>();
        assert_eq!(
            shell.sim.phase(),
            RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS
            },
            "entering the Race state must rebuild a fresh countdown race"
        );
        assert_eq!(shell.sim.racing_ticks(), 0);
    }

    #[test]
    fn esc_during_race_returns_to_menu() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut app = lifecycle_app(track);

        // Enter the Race state first.
        press(&mut app, KeyCode::Enter);
        settle(&mut app);
        assert_eq!(state(&mut app), AppState::Race);

        press(&mut app, KeyCode::Escape);
        settle(&mut app);
        assert_eq!(state(&mut app), AppState::Menu);
    }

    #[test]
    fn finished_race_transitions_to_results_exactly_when_finished() {
        let mut app = finished_shell_app();

        // A finished race trips the finish detector without any key press.
        app.update();
        app.update();
        assert_eq!(state(&mut app), AppState::Results);
    }

    #[test]
    fn results_r_instant_restarts_into_a_fresh_countdown_race() {
        let mut app = finished_shell_app();

        // Land on the results screen first.
        settle(&mut app);
        assert_eq!(state(&mut app), AppState::Results);

        press(&mut app, KeyCode::KeyR);
        settle(&mut app);
        assert_eq!(state(&mut app), AppState::Race);
        let shell = app.world().resource::<ShellSimulation>();
        assert_eq!(
            shell.sim.phase(),
            RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS
            }
        );
    }

    #[test]
    fn results_esc_returns_to_menu_without_resetting_the_race() {
        let mut app = finished_shell_app();
        settle(&mut app);
        assert_eq!(state(&mut app), AppState::Results);

        press(&mut app, KeyCode::Escape);
        settle(&mut app);
        assert_eq!(state(&mut app), AppState::Menu);
        let shell = app.world().resource::<ShellSimulation>();
        assert_eq!(
            shell.sim.phase(),
            RacePhase::Finished,
            "returning to the menu must not rebuild the race"
        );
    }

    #[test]
    fn auto_start_skips_the_menu_but_still_resets_on_entry() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut app = lifecycle_app(track);
        app.add_systems(Update, auto_start_race);
        settle(&mut app);

        assert_eq!(state(&mut app), AppState::Race);
        let shell = app.world().resource::<ShellSimulation>();
        assert_eq!(
            shell.sim.phase(),
            RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS
            }
        );
    }
}
