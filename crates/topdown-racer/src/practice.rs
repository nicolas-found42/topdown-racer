//! Presentation of the single authored Corner Practice challenge.
use crate::{AppState, ShellSimulation};
use bevy::prelude::*;
use topdown_racer_core::practice::{CornerChallenge, PracticeStatus};
#[derive(Component)]
pub struct PracticeButton;
pub(crate) struct PracticePlugin;
impl Plugin for PracticePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_gates.run_if(in_state(AppState::Race)));
    }
}
pub(crate) fn record_key(shell: &ShellSimulation) -> String {
    format!(
        "practice:{}:{:016x}:handling1:manual-section1:{:?}",
        CornerChallenge::ID,
        crate::records::track_identity(shell.sim.track()),
        shell.steering_response
    )
}
fn draw_gates(shell: Res<ShellSimulation>, mut gizmos: Gizmos) {
    let Some(attempt) = &shell.practice else {
        return;
    };
    for (i, gate) in attempt.challenge.gates.iter().enumerate() {
        let left = Vec2::new(-gate.direction.y, gate.direction.x) * gate.half_width;
        let color = if i == 0 {
            crate::palette::SIGNAL_CYAN
        } else if i + 1 == attempt.challenge.gates.len() {
            crate::palette::CREAM_HIGHLIGHT
        } else {
            crate::palette::RIVAL_ORANGE
        };
        gizmos.line_2d(
            gate.center - left,
            gate.center + left,
            crate::palette::color(color),
        );
    }
}
pub(crate) fn spawn_results(commands: &mut Commands, shell: &ShellSimulation, notice: &str) {
    let attempt = shell.practice.as_ref().unwrap();
    let text = match attempt.status() {
        PracticeStatus::Finished {
            seconds,
            exit_speed,
        } => format!(
            "SECTION {:.3}s | EXIT {:.1} u/s\n{}",
            seconds,
            exit_speed,
            shell
                .practice_baseline
                .map(|best| format!("{:+.3}s against previous best ({best:.3}s)", seconds - best))
                .unwrap_or_else(|| "First eligible attempt".into())
        ),
        PracticeStatus::Invalid(reason) => format!("ATTEMPT INELIGIBLE: {reason}"),
        _ => "ATTEMPT ENDED".into(),
    };
    crate::overlay::spawn_overlay_root(commands, 12.0, 0.94)
        .insert(crate::ResultsUi)
        .with_children(|root| {
            if !notice.is_empty() {
                root.spawn(TextBundle::from_section(
                    notice,
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
            }
            for (label, size) in [
                ("CORNER PRACTICE".to_owned(), 36.0),
                (text, 24.0),
                (CornerChallenge::HINT.to_owned(), 16.0),
                (
                    "R / Enter: retry identical start | Esc: menu".to_owned(),
                    20.0,
                ),
            ] {
                root.spawn(TextBundle::from_section(
                    label,
                    TextStyle {
                        font_size: size,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
            }
        });
}
