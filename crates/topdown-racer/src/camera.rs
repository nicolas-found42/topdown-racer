//! Camera projection: letterbox viewport math, pose interpolation, and the
//! follow-camera systems.

use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};
use glam::Vec2;
use topdown_racer_core::simulation::{wrap_angle, CarSnapshot};

use crate::{CarSprite, ShellSimulation, CAMERA_ZOOM, TARGET_ASPECT_RATIO};

/// Component tagging the follow camera.
#[derive(Component)]
pub struct FollowCamera;

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
    prev + wrap_angle(curr - prev) * alpha
}

/// Interpolates 2D world position.
pub fn interpolate_pose(prev: Vec2, curr: Vec2, alpha: f32) -> Vec2 {
    prev.lerp(curr, alpha)
}

/// Projects two car snapshots into an interpolated pose and heading.
pub(crate) fn interpolate_snapshot_pose(
    prev: &CarSnapshot,
    curr: &CarSnapshot,
    alpha: f32,
) -> (Vec2, f32) {
    (
        interpolate_pose(prev.pose, curr.pose, alpha),
        interpolate_heading(prev.heading, curr.heading, alpha),
    )
}

/// Spawns the 2D camera with the fixed zoom and follow-camera marker.
pub(crate) fn setup_camera(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.projection.scale = CAMERA_ZOOM;
    commands.spawn((camera, FollowCamera));
}

/// Updates camera viewport letterboxing when the window size changes.
pub(crate) fn update_letterbox(
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

/// Interpolates Car Sprite transform and follow Camera between fixed steps.
pub(crate) fn interpolate_car_and_camera(
    shell: Res<ShellSimulation>,
    fixed_time: Res<Time<Fixed>>,
    mut cars: Query<(&CarSprite, &mut Transform), Without<FollowCamera>>,
    mut cameras: Query<&mut Transform, (With<FollowCamera>, Without<CarSprite>)>,
) {
    let alpha = fixed_time.overstep_fraction();

    for (sprite, mut car_tf) in cars.iter_mut() {
        if let (Some(prev), Some(curr)) = (
            shell.prev_snapshots.get(sprite.car_index),
            shell.curr_snapshots.get(sprite.car_index),
        ) {
            let (interp_pose, interp_heading) = interpolate_snapshot_pose(prev, curr, alpha);

            car_tf.translation.x = interp_pose.x;
            car_tf.translation.y = interp_pose.y;
            car_tf.rotation = Quat::from_rotation_z(interp_heading);

            // Follow camera tracks player Car (index 0)
            if sprite.car_index == 0 {
                for mut cam_tf in cameras.iter_mut() {
                    cam_tf.translation.x = interp_pose.x;
                    cam_tf.translation.y = interp_pose.y;
                    cam_tf.rotation = Quat::IDENTITY;
                }
            }
        }
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
        let vp = compute_letterbox_viewport(2560, 1080, 16.0 / 9.0);
        assert_eq!(vp.width, 1920);
        assert_eq!(vp.height, 1080);
        assert_eq!(vp.x, 320); // (2560 - 1920) / 2
        assert_eq!(vp.y, 0);
    }

    #[test]
    fn letterbox_viewport_taller_window_adds_letterbox() {
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
        let mid = interpolate_heading(prev, curr, 0.5);
        assert!(
            mid.abs() > 3.0,
            "midpoint across boundary must stay near +/- pi: got {}",
            mid
        );
    }

    #[test]
    fn follow_camera_transform_tracks_car_pose_with_identity_rotation() {
        let p0 = Vec2::new(10.0, 20.0);
        let p1 = Vec2::new(30.0, 40.0);
        let alpha = 0.5;
        let interp = interpolate_pose(p0, p1, alpha);

        let mut cam_tf = Transform::from_xyz(0.0, 0.0, 999.0);
        cam_tf.translation.x = interp.x;
        cam_tf.translation.y = interp.y;
        cam_tf.rotation = Quat::IDENTITY;

        assert_eq!(cam_tf.translation.x, 20.0);
        assert_eq!(cam_tf.translation.y, 30.0);
        assert_eq!(cam_tf.rotation, Quat::IDENTITY);
    }
}
