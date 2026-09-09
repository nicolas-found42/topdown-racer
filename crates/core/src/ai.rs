//! The AI driver seam: a per-Car brain that turns perception into [`CarInput`]
//! — the exact same controls a human player has. The driver never touches
//! physics or Car state directly: it plans a speed profile from the car's
//! real acceleration and grip limits, picks its own line through upcoming
//! geometry, follows and overtakes other Cars, and recovers from off-track
//! excursions. There is no scripted racing line and no rubber-banding.

use glam::Vec2;

use crate::simulation::{
    wrap_angle, CarInput, BRAKE_ACCEL, LATERAL_GRIP_RATE, MAX_ANGULAR_VEL, STEER_SENSITIVITY,
    TOP_SPEED,
};
use crate::track::{Surface, Track};

/// Perception handed to a driver once per tick: everything a cockpit view
/// would show — own state, the Track, and every Car in the field.
pub struct AiView<'a> {
    /// Index of the driven Car in `field`.
    pub car_index: usize,
    /// World position of the driven Car.
    pub pose: Vec2,
    /// Heading of the driven Car in radians.
    pub heading: f32,
    /// World velocity of the driven Car.
    pub velocity: Vec2,
    /// The Track being raced.
    pub track: &'a Track,
    /// (pose, velocity) of every Car in the field, indexed like the Sim.
    pub field: &'a [(Vec2, Vec2)],
}

/// Distance between speed-plan samples along the centerline, in world units.
const SAMPLE_STEP: f32 = 2.0;
/// Number of plan samples: a 72-unit horizon covers full-speed braking with margin.
const PLAN_SAMPLES: usize = 36;
/// Lateral-slip allowance the speed plan holds (world units/s), set just
/// under the physics drift threshold so planned corners stay gripped.
const SLIP_ALLOWANCE: f32 = 1.0;
/// Tightest turn radius the plan assumes the Car can use by cutting inside a
/// corner (bounded by road width). Centerline samples at polyline vertices
/// report far higher curvature than any drivable line, so clamp there.
const PATH_MIN_RADIUS: f32 = 8.0;
/// Fraction of full braking the plan assumes, leaving a margin for error.
const PLAN_BRAKE_FRACTION: f32 = 0.7;
/// Aim-point lookahead scales with speed, clamped to a sane window.
const AIM_SPEED_SCALE: f32 = 0.6;
const AIM_MIN: f32 = 6.0;
const AIM_MAX: f32 = 26.0;
/// How strongly upcoming curvature pulls the chosen line toward the inside.
const APEX_BIAS_GAIN: f32 = 24.0;
/// Lateral margin the line keeps from the road edges.
const EDGE_MARGIN: f32 = 2.0;
/// Steering proportional gain on the angle to the aim point.
const STEER_GAIN: f32 = 3.0;
/// Countersteer gain against current lateral slip (catches slides).
const COUNTERSTEER_GAIN: f32 = 0.15;
/// Speed error above target that maps to full brake.
const BRAKE_ERROR_FULL: f32 = 6.0;
/// Speed error below target that maps to full throttle.
const THROTTLE_ERROR_FULL: f32 = 4.0;
/// Fraction of throttle cut when steering hard at speed (avoids understeer).
const TRACTION_CUT: f32 = 0.35;
/// Rival perception window ahead and to the sides.
const AVOID_RANGE: f32 = 28.0;
const AVOID_LATERAL: f32 = 4.0;
/// Lateral shift committed when dodging a slower Car ahead.
const AVOID_SHIFT: f32 = 4.0;
/// Gap below which the driver matches a slower Car's speed instead of passing through it.
const FOLLOW_GAP: f32 = 9.0;
/// Sideways separation off the driver's nose below which a rival ahead
/// counts as blocking the same corridor.
const SAME_CORRIDOR_LATERAL: f32 = 2.0;
/// Per-tick blending of the chosen line toward its new target (~0.2 s commitment).
const LINE_SMOOTHING: f32 = 0.08;
/// Reverse recovery tuning.
const STUCK_SPEED: f32 = 0.5;
const STUCK_TICKS_LIMIT: u32 = 48;
const REVERSE_TICKS: u32 = 64;
/// Heading error vs the Track direction that counts as facing the wrong way.
const FACING_AWAY_ANGLE: f32 = 2.2;
/// Speed below which the facing-away trigger may fire.
const REVERSE_SPEED_CAP: f32 = 6.0;
/// Speed-plan scale while off the authored surface.
const OFF_TRACK_SPEED_FACTOR: f32 = 0.45;
/// Handbrake rotation assist: large heading error at speed.
const HANDBRAKE_ANGLE: f32 = 1.1;
const HANDBRAKE_SPEED: f32 = 13.0;

/// A decision-making driver producing the same [`CarInput`] controls as the
/// player. Persistent state is limited to a smoothed line choice, a
/// stuck/reverse recovery mode, and a deterministic personality.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AiDriver {
    /// Scales the planned speed envelope (0.88..1.0 of physics limits).
    speed_scale: f32,
    /// Shortens the following gap and commits harder to overtakes (0.5..1.0).
    aggression: f32,
    /// Smoothed lateral offset the driver has chosen for its line.
    line_offset: f32,
    /// Consecutive ticks below the stuck speed while demanding drive.
    stuck_ticks: u32,
    /// Remaining ticks in reverse-and-recover mode (0 = normal driving).
    reverse_ticks: u32,
}

impl AiDriver {
    /// Builds the driver for a grid slot. Personality is a deterministic hash
    /// of the slot: distinct rivals, stable across runs, no rubber-banding.
    pub fn new(car_index: usize) -> Self {
        let h = (car_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        Self {
            speed_scale: 0.88 + (h >> 60) as f32 / 15.0 * 0.12,
            aggression: 0.5 + ((h >> 44) & 0xFFFF) as f32 / 65535.0 * 0.5,
            line_offset: 0.0,
            stuck_ticks: 0,
            reverse_ticks: 0,
        }
    }

    /// Turns one tick of perception into the player-identical [`CarInput`].
    pub fn compute_input(&mut self, view: AiView<'_>) -> CarInput {
        let fwd = Vec2::new(view.heading.cos(), view.heading.sin());
        let left = Vec2::new(-fwd.y, fwd.x);
        let vf = view.velocity.dot(fwd);
        let vl = view.velocity.dot(left);

        let frame = view.track.centerline_frame(view.pose);
        let track_angle = frame.direction.y.atan2(frame.direction.x);

        // Recovery mode: reverse out of walls and dead ends while rotating the
        // nose back toward the Track direction. In reverse the steering sign
        // flips (negative forward speed), so steer against the heading error.
        if self.reverse_ticks > 0 {
            self.reverse_ticks -= 1;
            if self.reverse_ticks == 0 {
                self.stuck_ticks = 0;
            }
            let error = wrap_angle(track_angle - view.heading);
            let side = if error >= 0.0 { 1.0 } else { -1.0 };
            return CarInput {
                throttle: 0.0,
                brake: 1.0,
                steer: -0.8 * side,
                handbrake: false,
            };
        }

        let surface = view.track.sample_surface(view.pose);
        let off_track = surface == Surface::Grass;

        // Stuck detection runs only while racing (the Sim never consults the
        // driver during countdown, so the counter cannot fill at the start).
        if vf.abs() < STUCK_SPEED {
            self.stuck_ticks += 1;
        } else {
            self.stuck_ticks = 0;
        }
        let facing_away = wrap_angle(track_angle - view.heading).abs() > FACING_AWAY_ANGLE;
        if self.stuck_ticks > STUCK_TICKS_LIMIT
            || (off_track && facing_away && vf.abs() < REVERSE_SPEED_CAP)
        {
            self.reverse_ticks = REVERSE_TICKS;
            let error = wrap_angle(track_angle - view.heading);
            let side = if error >= 0.0 { 1.0 } else { -1.0 };
            return CarInput {
                throttle: 0.0,
                brake: 1.0,
                steer: -0.8 * side,
                handbrake: false,
            };
        }

        // Speed plan: fastest feasible speed at each sample ahead, then a
        // backward pass so the Car arrives at every sample already slow
        // enough for the next one, assuming physics braking with margin.
        let plan = self.speed_plan(view.track, frame.arc);
        let mut target_speed = plan[0];
        if off_track {
            target_speed *= OFF_TRACK_SPEED_FACTOR;
        }

        // Line choice: bias toward the inside of the dominant upcoming
        // corner, then dodge slower Cars ahead. Off-track, aim back at the
        // centerline instead of an apex.
        let aim_dist = (vf * AIM_SPEED_SCALE).clamp(AIM_MIN, AIM_MAX);
        let aim_arc = frame.arc + aim_dist;
        let max_offset = (view.track.road_half_width_at_arc(aim_arc) - EDGE_MARGIN).max(0.5);
        let mut offset = if off_track {
            0.0
        } else {
            (centerline_curvature(view.track, aim_arc) * APEX_BIAS_GAIN)
                .clamp(-max_offset, max_offset)
        };

        // Traffic decisions: dodge Cars being gained on, and match speed only
        // while physically blocked on the same corridor. A rival on a
        // different line is passed, not queued behind.
        let mut follow_speed = f32::INFINITY;
        for (j, &(other_pose, other_vel)) in view.field.iter().enumerate() {
            if j == view.car_index {
                continue;
            }
            let rel = other_pose - view.pose;
            let ahead = rel.dot(fwd);
            let lateral = rel.dot(left);
            if ahead <= 0.0 || ahead > AVOID_RANGE || lateral.abs() > AVOID_LATERAL {
                continue;
            }
            let closing = vf - other_vel.dot(fwd);
            if closing <= 0.2 {
                continue;
            }
            // Pass on the side the rival is not occupying; a Car dead ahead
            // or to the left is passed on the right.
            let dodge_dir = if lateral < -0.5 { 1.0 } else { -1.0 };
            let urgency = (1.0 - ahead / AVOID_RANGE) * (0.5 + self.aggression * 0.5);
            offset += dodge_dir * urgency * AVOID_SHIFT;
            // Lift only while on a collision course with the rival: once
            // there is sideways separation off the nose, the corridor is open
            // and the driver passes instead of queuing.
            let same_corridor = lateral.abs() < SAME_CORRIDOR_LATERAL;
            if same_corridor && ahead < FOLLOW_GAP {
                follow_speed = follow_speed.min(other_vel.dot(fwd).max(2.0));
            }
        }
        offset = offset.clamp(-max_offset, max_offset);
        // Commit smoothly: the line eases toward its target instead of twitching.
        self.line_offset += (offset - self.line_offset) * LINE_SMOOTHING;

        target_speed = target_speed.min(follow_speed);

        // Steering: proportional to the aim-point angle, with countersteer
        // against current lateral slip. Same stick the player holds.
        let aim_center = view.track.point_at_arc(aim_arc);
        let dir_at_aim = (view.track.point_at_arc(aim_arc + 1.0) - aim_center).normalize_or_zero();
        let normal_at_aim = Vec2::new(-dir_at_aim.y, dir_at_aim.x);
        let aim = aim_center + normal_at_aim * self.line_offset;
        let to_aim = aim - view.pose;
        let angle_diff = wrap_angle(to_aim.y.atan2(to_aim.x) - view.heading);
        let mut steer = (angle_diff * STEER_GAIN - vl * COUNTERSTEER_GAIN).clamp(-1.0, 1.0);
        if off_track {
            steer = (steer * 1.5).clamp(-1.0, 1.0);
        }

        // Pedals: proportional to the speed error against the plan. The
        // driver lifts off through hard steering at speed and reaches for the
        // handbrake only to rotate through a large heading error at speed.
        let handbrake = angle_diff.abs() > HANDBRAKE_ANGLE && vf > HANDBRAKE_SPEED;
        let (mut throttle, brake) = if vf > target_speed + 0.5 {
            (
                0.0,
                ((vf - target_speed) / BRAKE_ERROR_FULL).clamp(0.0, 1.0),
            )
        } else {
            let mut throttle = ((target_speed - vf) / THROTTLE_ERROR_FULL).clamp(0.0, 1.0);
            throttle *= 1.0 - TRACTION_CUT * steer.abs() * (vf / TOP_SPEED).clamp(0.0, 1.0);
            (throttle, 0.0)
        };
        if handbrake {
            throttle = 0.0;
        }

        CarInput {
            throttle,
            brake,
            steer,
            handbrake,
        }
    }

    /// Fastest feasible speed at each sample ahead of `arc`, from the car's
    /// own steering and grip limits, with a braking backward pass.
    fn speed_plan(&self, track: &Track, arc: f32) -> [f32; PLAN_SAMPLES] {
        let mut plan = [0.0f32; PLAN_SAMPLES];
        for (i, slot) in plan.iter_mut().enumerate() {
            // Holding curvature k at speed v needs yaw rate k*v. Steering
            // caps the rate at MAX_ANGULAR_VEL, and the grip model settles at
            // lateral slip v*(k*v)/LATERAL_GRIP_RATE which must stay gripped.
            // Planned curvature is also capped at what full steering lock can
            // deliver, so the plan never asks for an undrivable line.
            let k = centerline_curvature(track, arc + i as f32 * SAMPLE_STEP)
                .abs()
                .clamp(1e-4, (1.0 / PATH_MIN_RADIUS).min(STEER_SENSITIVITY));
            let grip_limit = (LATERAL_GRIP_RATE * SLIP_ALLOWANCE / k).sqrt();
            let steer_limit = MAX_ANGULAR_VEL / k;
            *slot = grip_limit.min(steer_limit).min(TOP_SPEED) * self.speed_scale;
        }
        for i in (0..PLAN_SAMPLES - 1).rev() {
            let reachable = (plan[i + 1] * plan[i + 1]
                + 2.0 * BRAKE_ACCEL * PLAN_BRAKE_FRACTION * SAMPLE_STEP)
                .sqrt();
            plan[i] = plan[i].min(reachable);
        }
        plan
    }
}

/// Signed curvature of the centerline just ahead of `arc`, from the
/// circumcircle of three forward arc samples (positive = left turn). Looking
/// only ahead keeps a vertex behind the Car (such as the start-line corner
/// at launch) out of the speed plan. Wraps around the loop.
fn centerline_curvature(track: &Track, arc: f32) -> f32 {
    let p0 = track.point_at_arc(arc);
    let p1 = track.point_at_arc(arc + SAMPLE_STEP);
    let p2 = track.point_at_arc(arc + 2.0 * SAMPLE_STEP);
    let a = p1 - p0;
    let b = p2 - p1;
    let denom = a.length() * b.length() * (p2 - p0).length();
    if denom < 1e-6 {
        return 0.0;
    }
    2.0 * (a.x * b.y - a.y * b.x) / denom
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track::SAMPLE_CIRCUIT;

    fn view<'a>(
        track: &'a Track,
        pose: Vec2,
        heading: f32,
        velocity: Vec2,
        field: &'a [(Vec2, Vec2)],
    ) -> AiView<'a> {
        AiView {
            car_index: 0,
            pose,
            heading,
            velocity,
            track,
            field,
        }
    }

    #[test]
    fn driver_brakes_for_a_corner_before_steering_for_it() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let me = (Vec2::new(108.0, 0.0), Vec2::new(28.0, 0.0));
        let field = [me];
        let mut driver = AiDriver::new(1);
        // Twelve units before the vertex at x = 120, flat out at 28 u/s: the
        // plan already sees the corner and is on the brakes, turning in
        // rather than at full lock.
        let input = driver.compute_input(view(&track, me.0, 0.0, me.1, &field));
        assert!(
            input.brake > 0.15,
            "must brake into the corner, got {input:?}"
        );
        assert_eq!(
            input.throttle, 0.0,
            "must be off the throttle, got {input:?}"
        );
        assert!(
            input.steer.abs() < 0.9,
            "must turn in rather than panic, got {input:?}"
        );
        assert!(!input.handbrake);
    }

    #[test]
    fn driver_dodges_a_slower_car_and_matches_speed_when_too_close() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut driver = AiDriver::new(2);

        // Rival slightly to the left, being gained on: commit to the right.
        let me = (Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0));
        let field = [me, (Vec2::new(12.0, 1.0), Vec2::new(10.0, 0.0))];
        for _ in 0..30 {
            driver.compute_input(view(&track, me.0, 0.0, me.1, &field));
        }
        let input = driver.compute_input(view(&track, me.0, 0.0, me.1, &field));
        assert!(
            input.steer < -0.05,
            "must steer away from the rival's side, got {input:?}"
        );

        // Nose-to-tail on the same corridor with a much slower Car: stand on
        // the brakes to their speed.
        let close = [me, (Vec2::new(6.0, 0.0), Vec2::new(5.0, 0.0))];
        let mut chaser = AiDriver::new(3);
        let input = chaser.compute_input(view(&track, me.0, 0.0, me.1, &close));
        assert!(
            input.brake > 0.5,
            "must brake for the slower car, got {input:?}"
        );
    }

    #[test]
    fn driver_line_clamp_follows_local_width_at_the_aim_point() {
        // Two tracks identical except widths, uniform 8 vs ramping 8 -> 60.
        // The uniform cap (half 4 - EDGE_MARGIN 2 = 2) clamps the dodge
        // line harder than the local cap at the aim point (~17.6): the
        // driver on the wide-at-aim track commits to a wider line for the
        // same traffic.
        let text = |widths: &str| {
            format!(
                r#"{{"name": "Ramp", "width": 8.0, "widths": {widths}, "points": [[0,0],[120,0],[120,60],[0,0]], "surfaces": []}}"#
            )
        };
        let uniform = Track::parse(&text("[8.0, 8.0, 8.0, 8.0]")).unwrap();
        let wide_ahead = Track::parse(&text("[8.0, 60.0, 60.0, 8.0]")).unwrap();

        // Three slower rivals ahead on the right: the driver commits to a
        // left line worth more than the uniform cap.
        let me = (Vec2::new(60.0, 0.0), Vec2::new(20.0, 0.0));
        let field = [
            me,
            (Vec2::new(70.0, -3.5), Vec2::new(5.0, 0.0)),
            (Vec2::new(76.0, -3.5), Vec2::new(5.0, 0.0)),
            (Vec2::new(82.0, -3.5), Vec2::new(5.0, 0.0)),
        ];

        let mut uniform_driver = AiDriver::new(1);
        let mut wide_driver = AiDriver::new(1);
        let mut uniform_steer = 0.0;
        let mut wide_steer = 0.0;
        for _ in 0..40 {
            uniform_steer = uniform_driver
                .compute_input(view(&uniform, me.0, 0.0, me.1, &field))
                .steer;
            wide_steer = wide_driver
                .compute_input(view(&wide_ahead, me.0, 0.0, me.1, &field))
                .steer;
        }
        assert!(
            wide_steer.abs() > uniform_steer.abs() + 0.05,
            "wide-at-aim driver must commit wider than the uniform cap: \
             wide {wide_steer} vs uniform {uniform_steer}"
        );
    }

    #[test]
    fn driver_output_is_always_a_legal_player_input() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let field = [
            (Vec2::new(30.0, 2.0), Vec2::new(12.0, 1.0)),
            (Vec2::new(-10.0, -20.0), Vec2::new(0.0, 0.0)),
        ];
        let mut driver = AiDriver::new(1);
        // Sweep poses, headings, and speeds across the circuit: every output
        // must stay inside the control ranges a gamepad could produce.
        for i in 0..200 {
            let arc = i as f32 * 2.7;
            let center = track.point_at_arc(arc);
            for &(heading, speed) in &[(0.0, 0.0), (1.3, 15.0), (-2.1, 28.0)] {
                let input = driver.compute_input(view(
                    &track,
                    center + Vec2::new(3.0, -4.0),
                    heading,
                    Vec2::new(heading.cos(), heading.sin()) * speed,
                    &field,
                ));
                assert!((0.0..=1.0).contains(&input.throttle), "{input:?}");
                assert!((0.0..=1.0).contains(&input.brake), "{input:?}");
                assert!((-1.0..=1.0).contains(&input.steer), "{input:?}");
            }
        }
    }
}
