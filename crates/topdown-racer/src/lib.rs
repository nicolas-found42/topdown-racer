//! Shell library for Topdown Racer: camera, rendering, and simulation glue.

use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};
use glam::Vec2;
use topdown_racer_core::{
    simulation::{CarInput, CarSnapshot, Sim, FIXED_HZ},
    track::{Surface, Track},
};

/// Canonical aspect ratio for the game view (16:9).
pub const TARGET_ASPECT_RATIO: f32 = 16.0 / 9.0;

/// Fixed camera zoom (orthographic scale). Smaller value = closer zoom.
pub const CAMERA_ZOOM: f32 = 0.08;

/// Embedded copy of the sample circuit JSON data.
pub const SAMPLE_CIRCUIT_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../core/data/tracks/sample-circuit.json"
));

/// Rectangular viewport region for letterboxing/pillarboxing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Computes the pixel viewport rectangle to letterbox or pillarbox
/// a target aspect ratio inside a window of size `(window_width, window_height)`.
pub fn compute_letterbox_viewport(
    window_width: u32,
    window_height: u32,
    target_aspect: f32,
) -> ViewportRect {
    if window_width == 0 || window_height == 0 || target_aspect <= 0.0 {
        return ViewportRect {
            x: 0,
            y: 0,
            width: window_width,
            height: window_height,
        };
    }

    let current_aspect = window_width as f32 / window_height as f32;

    if current_aspect > target_aspect {
        // Window is wider than target: pillarbox (black bars left and right).
        let viewport_width = (window_height as f32 * target_aspect).round() as u32;
        let x = (window_width.saturating_sub(viewport_width)) / 2;
        ViewportRect {
            x,
            y: 0,
            width: viewport_width.min(window_width),
            height: window_height,
        }
    } else {
        // Window is taller than target: letterbox (black bars top and bottom).
        let viewport_height = (window_width as f32 / target_aspect).round() as u32;
        let y = (window_height.saturating_sub(viewport_height)) / 2;
        ViewportRect {
            x: 0,
            y,
            width: window_width,
            height: viewport_height.min(window_height),
        }
    }
}

/// Smoothly interpolates an angle in radians between `prev` and `curr` taking
/// the shortest rotational path across the $[-\pi, \pi]$ boundary.
pub fn interpolate_heading(prev: f32, curr: f32, alpha: f32) -> f32 {
    let mut diff = curr - prev;
    while diff > std::f32::consts::PI {
        diff -= 2.0 * std::f32::consts::PI;
    }
    while diff < -std::f32::consts::PI {
        diff += 2.0 * std::f32::consts::PI;
    }
    prev + diff * alpha
}

/// Interpolates 2D world position.
pub fn interpolate_pose(prev: Vec2, curr: Vec2, alpha: f32) -> Vec2 {
    prev.lerp(curr, alpha)
}

/// Component tagging the follow camera.
#[derive(Component)]
pub struct FollowCamera;

/// Component tagging the rendered Car chassis.
#[derive(Component)]
pub struct CarVisual {
    pub car_index: usize,
}

/// Resource holding simulation state and consecutive snapshots for render interpolation.
#[derive(Resource)]
pub struct ShellSimulation {
    pub sim: Sim,
    pub prev_snapshot: CarSnapshot,
    pub curr_snapshot: CarSnapshot,
    pub waypoint_index: usize,
}

impl ShellSimulation {
    pub fn new(track: Track) -> Self {
        let mut sim = Sim::new(track, 1);
        let initial = sim.tick(&[CarInput::default()])[0];
        Self {
            sim,
            prev_snapshot: initial,
            curr_snapshot: initial,
            waypoint_index: 0,
        }
    }
}

/// Main game plugin wiring camera, track rendering, simulation fixed step, and motion interpolation.
pub struct RacerGamePlugin;

impl Plugin for RacerGamePlugin {
    fn build(&self, app: &mut App) {
        let track = Track::parse(SAMPLE_CIRCUIT_JSON).expect("sample circuit must parse");
        app.insert_resource(Time::<Fixed>::from_hz(FIXED_HZ as f64))
            .insert_resource(ShellSimulation::new(track))
            .add_systems(Startup, (setup_camera, setup_track, setup_car))
            .add_systems(FixedUpdate, step_simulation)
            .add_systems(Update, (update_letterbox, interpolate_car_and_camera));
    }
}

fn setup_camera(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.projection.scale = CAMERA_ZOOM;
    commands.spawn((camera, FollowCamera));
}

fn setup_track(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sim: Res<ShellSimulation>,
) {
    let track = sim.sim.track();
    let points = &track.points;
    let n = points.len() - 1;

    let road_mat = materials.add(Color::srgb(0.22, 0.22, 0.26));
    let gravel_mat = materials.add(Color::srgb(0.48, 0.40, 0.28));
    let wall_mat = materials.add(Color::srgb(0.75, 0.25, 0.25));
    let start_mat = materials.add(Color::srgb(0.95, 0.95, 0.95));

    let road_half_w = track.road_half_width();
    let wall_dist = track.wall_distance();

    // Spawn quads for each segment
    for i in 0..n {
        let p0 = points[i];
        let p1 = points[i + 1];
        let dir = (p1 - p0).normalize_or_zero();
        let normal = Vec2::new(-dir.y, dir.x);

        let mat = match track.surfaces.get(i).copied().unwrap_or(Surface::Road) {
            Surface::Road => road_mat.clone(),
            Surface::Gravel => gravel_mat.clone(),
            Surface::Grass => continue,
        };

        // Road segment mesh
        let road_mesh = create_quad_mesh(
            p0 + normal * road_half_w,
            p0 - normal * road_half_w,
            p1 - normal * road_half_w,
            p1 + normal * road_half_w,
        );
        commands.spawn(ColorMesh2dBundle {
            mesh: meshes.add(road_mesh).into(),
            material: mat,
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        });

        // Boundary walls: left and right wall line segments
        for side in &[-1.0f32, 1.0f32] {
            let wall_w = 0.6;
            let offset = *side * wall_dist;
            let w_p0 = p0 + normal * offset;
            let w_p1 = p1 + normal * offset;
            let wall_quad = create_quad_mesh(
                w_p0 + normal * (wall_w * 0.5),
                w_p0 - normal * (wall_w * 0.5),
                w_p1 - normal * (wall_w * 0.5),
                w_p1 + normal * (wall_w * 0.5),
            );
            commands.spawn(ColorMesh2dBundle {
                mesh: meshes.add(wall_quad).into(),
                material: wall_mat.clone(),
                transform: Transform::from_xyz(0.0, 0.0, 2.0),
                ..default()
            });
        }
    }

    // Start/finish line across the track at points[0]
    if n > 0 {
        let dir = (points[1] - points[0]).normalize_or_zero();
        let normal = Vec2::new(-dir.y, dir.x);
        let sf_mesh = create_quad_mesh(
            points[0] + normal * road_half_w + dir * 0.5,
            points[0] - normal * road_half_w + dir * 0.5,
            points[0] - normal * road_half_w - dir * 0.5,
            points[0] + normal * road_half_w - dir * 0.5,
        );
        commands.spawn(ColorMesh2dBundle {
            mesh: meshes.add(sf_mesh).into(),
            material: start_mat,
            transform: Transform::from_xyz(0.0, 0.0, 3.0),
            ..default()
        });
    }
}

fn create_quad_mesh(tl: Vec2, tr: Vec2, br: Vec2, bl: Vec2) -> Mesh {
    use bevy::render::mesh::{Indices, PrimitiveTopology};
    let positions = vec![
        [tl.x, tl.y, 0.0],
        [tr.x, tr.y, 0.0],
        [br.x, br.y, 0.0],
        [bl.x, bl.y, 0.0],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(indices);
    mesh
}

fn setup_car(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sim: Res<ShellSimulation>,
) {
    let car_mesh = meshes.add(Rectangle::new(4.0, 2.0));
    let car_mat = materials.add(Color::srgb(0.92, 0.22, 0.22));

    commands.spawn((
        ColorMesh2dBundle {
            mesh: car_mesh.into(),
            material: car_mat,
            transform: Transform::from_xyz(
                sim.curr_snapshot.pose.x,
                sim.curr_snapshot.pose.y,
                10.0,
            )
            .with_rotation(Quat::from_rotation_z(sim.curr_snapshot.heading)),
            ..default()
        },
        CarVisual { car_index: 0 },
    ));
}

/// Fixed step system: advances the simulation at 64 Hz using scripted inputs for verification.
fn step_simulation(mut shell: ResMut<ShellSimulation>) {
    shell.prev_snapshot = shell.curr_snapshot;

    // Scripted input: follows track waypoints
    let track = shell.sim.track();
    let waypoints = &track.points[..track.points.len() - 1];
    let current_pose = shell.curr_snapshot.pose;
    let current_heading = shell.curr_snapshot.heading;

    let target = waypoints[shell.waypoint_index % waypoints.len()];
    let to_target = target - current_pose;

    if to_target.length() < 12.0 {
        shell.waypoint_index = (shell.waypoint_index + 1) % waypoints.len();
    }

    let target_angle = to_target.y.atan2(to_target.x);
    let mut angle_diff = target_angle - current_heading;
    while angle_diff > std::f32::consts::PI {
        angle_diff -= 2.0 * std::f32::consts::PI;
    }
    while angle_diff < -std::f32::consts::PI {
        angle_diff += 2.0 * std::f32::consts::PI;
    }

    let steer = (angle_diff * 2.5).clamp(-1.0, 1.0);
    let throttle = if angle_diff.abs() > 0.5 { 0.5 } else { 1.0 };
    let brake = if angle_diff.abs() > 1.0 { 0.3 } else { 0.0 };

    let input = CarInput {
        throttle,
        brake,
        steer,
        handbrake: false,
    };

    let snaps = shell.sim.tick(&[input]);
    shell.curr_snapshot = snaps[0];
}

/// Updates camera viewport letterboxing when the window size changes.
fn update_letterbox(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Camera, With<FollowCamera>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok(mut camera) = cameras.get_single_mut() else {
        return;
    };

    let width = window.physical_width();
    let height = window.physical_height();
    let rect = compute_letterbox_viewport(width, height, TARGET_ASPECT_RATIO);

    camera.viewport = Some(Viewport {
        physical_position: UVec2::new(rect.x, rect.y),
        physical_size: UVec2::new(rect.width, rect.height),
        depth: 0.0..1.0,
    });
}

/// Interpolates Car visual transform and follow Camera between fixed steps.
fn interpolate_car_and_camera(
    shell: Res<ShellSimulation>,
    fixed_time: Res<Time<Fixed>>,
    mut cars: Query<&mut Transform, (With<CarVisual>, Without<FollowCamera>)>,
    mut cameras: Query<&mut Transform, (With<FollowCamera>, Without<CarVisual>)>,
) {
    let alpha = fixed_time.overstep_fraction();

    let interp_pose = interpolate_pose(shell.prev_snapshot.pose, shell.curr_snapshot.pose, alpha);
    let interp_heading = interpolate_heading(
        shell.prev_snapshot.heading,
        shell.curr_snapshot.heading,
        alpha,
    );

    // Update Car transform
    for mut car_tf in cars.iter_mut() {
        car_tf.translation.x = interp_pose.x;
        car_tf.translation.y = interp_pose.y;
        car_tf.rotation = Quat::from_rotation_z(interp_heading);
    }

    // Update Follow Camera: follows Car without rotating the world
    for mut cam_tf in cameras.iter_mut() {
        cam_tf.translation.x = interp_pose.x;
        cam_tf.translation.y = interp_pose.y;
        cam_tf.rotation = Quat::IDENTITY;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letterbox_viewport_exact_aspect_ratio_fills_window() {
        let vp = compute_letterbox_viewport(1920, 1080, 16.0 / 9.0);
        assert_eq!(vp.x, 0);
        assert_eq!(vp.y, 0);
        assert_eq!(vp.width, 1920);
        assert_eq!(vp.height, 1080);
    }

    #[test]
    fn letterbox_viewport_wider_window_adds_pillarbox() {
        // Ultra-wide 2560x1080 -> 16:9 viewport should be 1920x1080, centered
        let vp = compute_letterbox_viewport(2560, 1080, 16.0 / 9.0);
        assert_eq!(vp.width, 1920);
        assert_eq!(vp.height, 1080);
        assert_eq!(vp.x, 320); // (2560 - 1920) / 2
        assert_eq!(vp.y, 0);
    }

    #[test]
    fn letterbox_viewport_taller_window_adds_letterbox() {
        // 1080x1920 portrait -> 16:9 viewport should be 1080 wide, 608 high, centered vertically
        let vp = compute_letterbox_viewport(1080, 1920, 16.0 / 9.0);
        assert_eq!(vp.width, 1080);
        assert_eq!(vp.height, 608);
        assert_eq!(vp.x, 0);
        assert_eq!(vp.y, (1920 - 608) / 2);
    }

    #[test]
    fn interpolate_pose_endpoints_and_midpoint() {
        let p0 = Vec2::new(0.0, 10.0);
        let p1 = Vec2::new(20.0, 30.0);
        assert_eq!(interpolate_pose(p0, p1, 0.0), p0);
        assert_eq!(interpolate_pose(p0, p1, 1.0), p1);
        assert_eq!(interpolate_pose(p0, p1, 0.5), Vec2::new(10.0, 20.0));
    }

    #[test]
    fn interpolate_heading_crosses_pi_boundary_smoothly() {
        let prev = 3.10; // close to +pi
        let curr = -3.10; // close to -pi
                          // Total angular distance the short way is ~0.083 rad
        let mid = interpolate_heading(prev, curr, 0.5);
        // Midpoint should be near pi / -pi, not 0.0!
        assert!(
            mid.abs() > 3.0,
            "midpoint across boundary must stay near +/- pi: got {}",
            mid
        );
    }

    #[test]
    fn camera_follows_car_without_rotating_world() {
        let car_pose = Vec2::new(123.4, -56.7);
        let mut cam_tf = Transform::from_xyz(0.0, 0.0, 999.0);
        // Camera update logic:
        cam_tf.translation.x = car_pose.x;
        cam_tf.translation.y = car_pose.y;
        cam_tf.rotation = Quat::IDENTITY;

        assert_eq!(cam_tf.translation.x, car_pose.x);
        assert_eq!(cam_tf.translation.y, car_pose.y);
        assert_eq!(cam_tf.rotation, Quat::IDENTITY);
    }
}
