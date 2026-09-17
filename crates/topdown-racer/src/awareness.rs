//! Fixed-scale camera lead and a compact Track overview.
use crate::{palette, AppState, FollowCamera, ShellSimulation};
use bevy::prelude::*;

/// At most 0.4 seconds of preview, capped at ten world units in any direction.
pub const MAX_CAMERA_LEAD: f32 = 10.0;
pub const NEARBY_RIVAL_DISTANCE: f32 = 60.0;

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
    previous: Vec2,
    generation: u64,
}
impl CameraLead {
    pub fn step(&mut self, velocity: Vec2, mode: CameraMode) -> Vec2 {
        self.previous = self.offset;
        if mode == CameraMode::Centered {
            self.offset = Vec2::ZERO;
            return self.offset;
        }
        let target = if velocity.length() < 2.0 {
            Vec2::ZERO
        } else {
            (velocity * 0.4).clamp_length_max(MAX_CAMERA_LEAD)
        };
        self.offset = self.offset.lerp(target, 0.08);
        self.offset
    }
    pub fn offset(&self) -> Vec2 {
        self.offset
    }
    /// Reject history from another Race even before its first fixed tick.
    pub fn interpolated_offset(&self, generation: u64, alpha: f32) -> Vec2 {
        if generation != self.generation {
            Vec2::ZERO
        } else {
            self.previous.lerp(self.offset, alpha.clamp(0.0, 1.0))
        }
    }
}

/// Nearby means at most 60 world units; markers remain inside the actual view.
pub fn rival_indicator(player: Vec2, rival: Vec2, camera: Vec2) -> Option<Vec2> {
    let relative = rival - camera;
    let half_view = crate::CAMERA_VIEW_SIZE * 0.5;
    if rival.distance(player) > NEARBY_RIVAL_DISTANCE
        || (relative.x.abs() <= half_view.x && relative.y.abs() <= half_view.y)
    {
        return None;
    }
    let inset = half_view - Vec2::splat(2.0);
    let scale = (inset.x / relative.x.abs()).min(inset.y / relative.y.abs());
    Some(camera + relative * scale)
}

/// Affine projection of world positions, not lap progress: no jump at the finish seam.
#[derive(Resource)]
pub struct OverviewProjection {
    min: Vec2,
    scale: f32,
}
impl OverviewProjection {
    pub fn new(points: &[Vec2]) -> Self {
        let min = points
            .iter()
            .copied()
            .fold(Vec2::splat(f32::INFINITY), Vec2::min);
        let max = points
            .iter()
            .copied()
            .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max);
        Self {
            min,
            scale: (16.0 / (max.x - min.x).max(1.0)).min(10.0 / (max.y - min.y).max(1.0)),
        }
    }
    pub fn project(&self, point: Vec2) -> Vec2 {
        Vec2::new(21.0, -16.0) + (point - self.min) * self.scale
    }
}

/// Use the play viewport, not window dimensions that may mostly be letterbox bars.
pub fn optional_awareness_visible(viewport: Vec2) -> bool {
    viewport.x >= 800.0 && viewport.y >= 450.0
}

#[derive(Component)]
struct OverviewPanel;
#[derive(Component)]
struct OverviewNumber(usize);

const LIVERIES: [u32; 4] = [
    palette::PLAYER_BLUE,
    palette::RIVAL_WHITE,
    palette::RIVAL_ORANGE,
    palette::RIVAL_PLUM,
];

fn setup_overview(mut commands: Commands, shell: Res<ShellSimulation>) {
    commands.insert_resource(OverviewProjection::new(&shell.sim.track().points));
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: palette::color(palette::ASPHALT_DARKEST),
                custom_size: Some(Vec2::new(20.0, 14.0)),
                ..default()
            },
            visibility: Visibility::Hidden,
            ..default()
        },
        OverviewPanel,
    ));
    for (i, color) in LIVERIES.into_iter().enumerate() {
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    (i + 1).to_string(),
                    TextStyle {
                        font_size: 16.0,
                        color: palette::color(color),
                        ..default()
                    },
                ),
                transform: Transform::from_scale(Vec3::splat(1.0 / 8.0)),
                visibility: Visibility::Hidden,
                ..default()
            },
            OverviewNumber(i),
        ));
    }
}

pub(crate) struct AwarenessPlugin;
impl Plugin for AwarenessPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraLead>()
            .add_systems(Startup, setup_overview)
            .add_systems(
                FixedUpdate,
                advance_lead
                    .after(crate::step_simulation)
                    .run_if(in_state(AppState::Race)),
            )
            .add_systems(
                Update,
                draw_awareness.after(crate::camera::interpolate_car_and_camera),
            );
    }
}
fn advance_lead(shell: Res<ShellSimulation>, mut lead: ResMut<CameraLead>) {
    if lead.generation != shell.generation {
        lead.offset = Vec2::ZERO;
        lead.previous = Vec2::ZERO;
        lead.generation = shell.generation;
    }
    if shell.sim.is_paused() || shell.sim.preparation_ticks() > 0 {
        return;
    }
    if let Some(player) = shell.curr_snapshots.first() {
        lead.step(player.velocity, shell.camera_mode);
    }
}

type PanelQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static mut Visibility),
    (
        With<OverviewPanel>,
        Without<FollowCamera>,
        Without<OverviewNumber>,
    ),
>;
type NumberQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static OverviewNumber,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (Without<OverviewPanel>, Without<FollowCamera>),
>;
fn draw_awareness(
    shell: Res<ShellSimulation>,
    overview: Res<OverviewProjection>,
    cameras: Query<(&Transform, &Camera), With<FollowCamera>>,
    mut panels: PanelQuery,
    mut numbers: NumberQuery,
    mut gizmos: Gizmos,
    state: Res<State<AppState>>,
) {
    let Ok((camera, view)) = cameras.get_single() else {
        return;
    };
    let show = *state.get() == AppState::Race
        && view
            .logical_viewport_size()
            .is_some_and(optional_awareness_visible);
    let center = camera.translation.truncate();
    for (mut transform, mut visibility) in &mut panels {
        *visibility = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        transform.translation = (center + Vec2::new(29.0, -11.0)).extend(100.0);
    }
    for (number, mut transform, mut visibility) in &mut numbers {
        let car = shell.curr_snapshots.get(number.0);
        *visibility = if show && car.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(car) = car {
            transform.translation = (center + overview.project(car.pose)).extend(101.0);
        }
    }
    if !show {
        return;
    }
    let Some(player) = shell.curr_snapshots.first() else {
        return;
    };
    let project = |point| center + overview.project(point);
    for pair in shell.sim.track().points.windows(2) {
        gizmos.line_2d(
            project(pair[0]),
            project(pair[1]),
            palette::color(palette::CREAM_HIGHLIGHT),
        );
    }
    // The cyan ring distinguishes the player's numbered Livery in a tight pack.
    gizmos.circle_2d(
        project(player.pose),
        0.8,
        palette::color(palette::SIGNAL_CYAN),
    );
    for (i, car) in shell.curr_snapshots.iter().enumerate().skip(1) {
        if car.finish_status != topdown_racer_core::simulation::FinishStatus::Racing {
            continue;
        }
        if let Some(at) = rival_indicator(player.pose, car.pose, center) {
            let direction = (car.pose - center).normalize_or_zero();
            let left = Vec2::new(-direction.y, direction.x);
            let color = palette::color(LIVERIES[i % LIVERIES.len()]);
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
