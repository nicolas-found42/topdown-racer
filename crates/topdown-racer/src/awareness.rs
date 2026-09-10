//! Fixed-scale camera lead and a compact Track overview.
use crate::{palette, AppState, FollowCamera, ShellSimulation};
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraMode {
    #[default]
    Centered,
    LookAhead,
}
impl CameraMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Centered => "CENTERED",
            Self::LookAhead => "LOOK AHEAD",
        }
    }
}

#[derive(Default, Resource)]
pub struct CameraLead {
    offset: Vec2,
    generation: u64,
}
impl CameraLead {
    pub fn step(&mut self, velocity: Vec2, mode: CameraMode) -> Vec2 {
        if mode == CameraMode::Centered {
            self.offset = Vec2::ZERO;
            return self.offset;
        }
        let target = if velocity.length() < 2.0 {
            Vec2::ZERO
        } else {
            (velocity * 0.4).clamp_length_max(10.0)
        };
        self.offset = self.offset.lerp(target, 0.08);
        self.offset
    }
    pub fn offset(&self) -> Vec2 {
        self.offset
    }
}

/// Nearby means at most 60 world units; markers remain inside the actual view.
pub fn rival_indicator(player: Vec2, rival: Vec2, camera: Vec2) -> Option<Vec2> {
    let relative = rival - camera;
    if rival.distance(player) > 60.0 || (relative.x.abs() <= 40.0 && relative.y.abs() <= 22.5) {
        return None;
    }
    let scale = (38.0 / relative.x.abs()).min(20.5 / relative.y.abs());
    Some(camera + relative * scale)
}

pub(crate) struct AwarenessPlugin;
impl Plugin for AwarenessPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraLead>()
            .add_systems(
                FixedUpdate,
                advance_lead
                    .after(crate::step_simulation)
                    .run_if(in_state(AppState::Race)),
            )
            .add_systems(
                Update,
                draw_awareness
                    .after(crate::camera::interpolate_car_and_camera)
                    .run_if(in_state(AppState::Race)),
            );
    }
}
fn advance_lead(shell: Res<ShellSimulation>, mut lead: ResMut<CameraLead>) {
    if lead.generation != shell.generation {
        lead.offset = Vec2::ZERO;
        lead.generation = shell.generation;
    }
    if shell.sim.is_paused() || shell.sim.preparation_ticks() > 0 {
        return;
    }
    if let Some(player) = shell.curr_snapshots.first() {
        lead.step(player.velocity, shell.camera_mode);
    }
}
fn draw_awareness(
    shell: Res<ShellSimulation>,
    cameras: Query<&Transform, With<FollowCamera>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut gizmos: Gizmos,
) {
    if windows
        .get_single()
        .is_ok_and(|w| w.width() < 800.0 || w.height() < 450.0)
    {
        return;
    }
    let Ok(camera) = cameras.get_single() else {
        return;
    };
    let center = camera.translation.truncate();
    let Some(player) = shell.curr_snapshots.first() else {
        return;
    };
    let track = shell.sim.track();
    let min = track
        .points
        .iter()
        .copied()
        .fold(Vec2::splat(f32::INFINITY), Vec2::min);
    let max = track
        .points
        .iter()
        .copied()
        .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max);
    let scale = (16.0 / (max.x - min.x).max(1.0)).min(10.0 / (max.y - min.y).max(1.0));
    let project = |point: Vec2| center + Vec2::new(21.0, -19.0) + (point - min) * scale;
    for pair in track.points.windows(2) {
        gizmos.line_2d(
            project(pair[0]),
            project(pair[1]),
            palette::color(palette::CREAM_HIGHLIGHT),
        );
    }
    let colors = [
        palette::PLAYER_BLUE,
        palette::RIVAL_WHITE,
        palette::RIVAL_ORANGE,
        palette::RIVAL_PLUM,
    ];
    for (i, car) in shell.curr_snapshots.iter().enumerate() {
        let color = palette::color(colors[i % colors.len()]);
        gizmos.circle_2d(project(car.pose), if i == 0 { 0.45 } else { 0.3 }, color);
        if i == 0 {
            gizmos.circle_2d(project(car.pose), 0.6, palette::color(palette::SIGNAL_CYAN));
        } else if car.finish_status == topdown_racer_core::simulation::FinishStatus::Racing {
            if let Some(at) = rival_indicator(player.pose, car.pose, center) {
                let direction = (car.pose - center).normalize_or_zero();
                let left = Vec2::new(-direction.y, direction.x);
                gizmos.line_2d(
                    at + direction * 0.65,
                    at - direction * 0.4 + left * 0.4,
                    color,
                );
                gizmos.line_2d(
                    at + direction * 0.65,
                    at - direction * 0.4 - left * 0.4,
                    color,
                );
            }
        }
    }
}
