//! The simulation seam: Track + per-Car inputs in, snapshots out, at a fixed
//! 64 Hz step. Plain Rust, no engine types — the Bevy shell renders snapshots
//! and maps keys to [`CarInput`]; all game rules live here.

use glam::Vec2;

use crate::track::Track;

/// Fixed simulation rate in steps per second.
pub const FIXED_HZ: f32 = 64.0;

/// Length of one fixed simulation step in seconds.
pub const FIXED_DT: f32 = 1.0 / FIXED_HZ;

/// Per-Car control for one tick, produced by the player's key mapping or an
/// AI driver. The simulation never reads devices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarInput {
    /// Forward acceleration demand, 0..1.
    pub throttle: f32,
    /// Braking demand, 0..1. Held at a standstill, engages Reverse.
    pub brake: f32,
    /// Steering demand, -1..1. Positive steers left (counter-clockwise).
    pub steer: f32,
}

impl Default for CarInput {
    fn default() -> Self {
        CarInput {
            throttle: 0.0,
            brake: 0.0,
            steer: 0.0,
        }
    }
}

/// One Car's externally observable state after a tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarSnapshot {
    /// World position.
    pub pose: Vec2,
    /// Heading in radians; 0 faces +x, positive is counter-clockwise.
    pub heading: f32,
    /// World velocity.
    pub velocity: Vec2,
    /// Signed speed along the Car's heading; negative while reversing.
    pub forward_speed: f32,
}

/// The headless world: Cars advancing over a Track at the fixed step.
pub struct Sim {
    track: Track,
    cars: Vec<CarState>,
}

struct CarState {
    pose: Vec2,
    heading: f32,
    velocity: Vec2,
    reverse: bool,
    standstill_ticks: u32,
}

impl Sim {
    /// Builds a sim with `car_count` Cars staged behind the Track's start
    /// line, all facing along the first polyline segment.
    pub fn new(track: Track, car_count: usize) -> Sim {
        let heading = track.points[1] - track.points[0];
        let heading = heading.y.atan2(heading.x);
        let cars = (0..car_count)
            .map(|i| CarState {
                pose: track.spawn_pose(i as f32 * CAR_SPACING),
                heading,
                velocity: Vec2::ZERO,
                reverse: false,
                standstill_ticks: 0,
            })
            .collect();
        Sim { track, cars }
    }

    /// The parsed Track this sim runs on.
    pub fn track(&self) -> &Track {
        &self.track
    }

    /// Advances the world one fixed step from the given per-Car inputs (in
    /// spawn order; missing inputs fall back to neutral) and returns one
    /// snapshot per Car.
    pub fn tick(&mut self, inputs: &[CarInput]) -> Vec<CarSnapshot> {
        self.cars
            .iter_mut()
            .enumerate()
            .map(|(i, car)| {
                let input = inputs.get(i).copied().unwrap_or_default();
                step_car(car, input);
                CarSnapshot {
                    pose: car.pose,
                    heading: car.heading,
                    velocity: car.velocity,
                    forward_speed: car.velocity.dot(forward(car.heading)),
                }
            })
            .collect()
    }
}

fn step_car(car: &mut CarState, input: CarInput) {
    let fwd = forward(car.heading);
    let left = Vec2::new(-fwd.y, fwd.x);
    let mut vf = car.velocity.dot(fwd);
    let vl = car.velocity.dot(left);

    let drag = DRAG_COEFF * vf * vf.abs() + ROLLING_RESIST * vf;

    if !car.reverse {
        if vf.abs() < STANDSTILL_THRESHOLD && input.throttle <= 0.0 {
            vf = 0.0;
            if input.brake > 0.0 {
                car.standstill_ticks += 1;
                if car.standstill_ticks >= STANDSTILL_REVERSE_TICKS {
                    car.reverse = true;
                    car.standstill_ticks = 0;
                }
            } else {
                car.standstill_ticks = 0;
            }
        } else {
            car.standstill_ticks = 0;
            let drive = input.throttle * ENGINE_ACCEL;
            let brake = input.brake * BRAKE_ACCEL;

            let accel = drive - drag - brake;
            vf += accel * FIXED_DT;

            if input.brake > 0.0 && vf < 0.0 {
                vf = 0.0;
            }
        }
    } else {
        if vf.abs() < STANDSTILL_THRESHOLD && input.brake <= 0.0 {
            vf = 0.0;
            car.reverse = false;
            car.standstill_ticks = 0;
        } else {
            let reverse_drive = -input.brake * REVERSE_ACCEL;
            let throttle_brake = input.throttle * BRAKE_ACCEL;

            let accel = reverse_drive - drag + throttle_brake;
            vf += accel * FIXED_DT;

            if vf < -REVERSE_MAX_SPEED {
                vf = -REVERSE_MAX_SPEED;
            }
            if input.throttle > 0.0 && vf > 0.0 {
                vf = 0.0;
                car.reverse = false;
                car.standstill_ticks = 0;
            }
        }
    }

    // Steering: angular velocity scales with forward speed and steering input.
    let angular_vel =
        (vf * STEER_SENSITIVITY * input.steer).clamp(-MAX_ANGULAR_VEL, MAX_ANGULAR_VEL);
    car.heading += angular_vel * FIXED_DT;

    let fwd = forward(car.heading);
    let left = Vec2::new(-fwd.y, fwd.x);
    car.velocity = fwd * vf + left * vl;
    car.pose += car.velocity * FIXED_DT;
}

/// Unit vector along a heading (0 faces +x, positive is counter-clockwise).
fn forward(heading: f32) -> Vec2 {
    Vec2::new(heading.cos(), heading.sin())
}

const CAR_SPACING: f32 = 4.0;

/// Engine acceleration at full throttle, in world units per second squared.
const ENGINE_ACCEL: f32 = 24.0;
/// Quadratic air drag coefficient, per (unit/s)^2.
const DRAG_COEFF: f32 = 0.0075;
/// Linear rolling resistance, per unit/s.
const ROLLING_RESIST: f32 = 0.6;
/// Braking deceleration at full brake, in world units per second squared.
const BRAKE_ACCEL: f32 = 36.0;
/// Reverse acceleration at full demand, in world units per second squared.
const REVERSE_ACCEL: f32 = 12.0;
/// Top speed while reversing, in world units per second.
const REVERSE_MAX_SPEED: f32 = 10.0;
/// Speed threshold below which the Car is considered at a standstill.
const STANDSTILL_THRESHOLD: f32 = 0.05;
/// Number of fixed ticks holding brake at a standstill to engage reverse (0.25 s at 64 Hz).
const STANDSTILL_REVERSE_TICKS: u32 = 16;
/// Maximum angular velocity when steering, in radians per second (~180 deg/s).
const MAX_ANGULAR_VEL: f32 = 3.2;
/// Steering sensitivity factor translating forward speed and steer input into angular velocity.
const STEER_SENSITIVITY: f32 = 0.15;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track::Surface;

    /// A long rectangular loop so a test Car has kilometers of open Road in
    /// front of it; spawn faces +x along the first segment.
    fn straight_track() -> Track {
        Track {
            name: "test loop".to_owned(),
            width: 20.0,
            points: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(2000.0, 0.0),
                Vec2::new(2000.0, 200.0),
                Vec2::new(0.0, 200.0),
                Vec2::new(0.0, 0.0),
            ],
            surfaces: vec![Surface::Road; 4],
        }
    }

    fn hold(throttle: f32, brake: f32, steer: f32) -> CarInput {
        CarInput {
            throttle,
            brake,
            steer,
        }
    }

    fn run(track: Track, inputs: &[CarInput]) -> Vec<CarSnapshot> {
        let mut sim = Sim::new(track, 1);
        let mut snaps = Vec::with_capacity(inputs.len());
        for input in inputs {
            snaps.extend(sim.tick(&[*input]));
        }
        snaps
    }
    #[test]
    fn held_throttle_accelerates_the_car_from_rest() {
        let inputs = vec![hold(1.0, 0.0, 0.0); FIXED_HZ as usize];
        let snaps = run(straight_track(), &inputs);

        // After the first tick (1/64 s), the Car has just started rolling.
        assert!(
            snaps[0].forward_speed > 0.0 && snaps[0].forward_speed < 1.0,
            "starts rolling: {}",
            snaps[0].forward_speed
        );
        let speeds: Vec<f32> = snaps.iter().map(|s| s.forward_speed).collect();
        // Speed builds monotonically under sustained throttle.
        for pair in speeds.windows(2) {
            assert!(pair[1] > pair[0], "speed must grow while accelerating");
        }
        // A full second of throttle carries the Car well down the straight.
        let last = *snaps.last().unwrap();
        assert!(
            last.forward_speed > 15.0,
            "top of first second, got {}",
            last.forward_speed
        );
        assert!(
            last.pose.x > 8.0,
            "must have moved forward, at {:?}",
            last.pose
        );
        assert!(
            (last.heading - snaps[0].heading).abs() < 1e-4,
            "no steer, no turn"
        );
    }

    #[test]
    fn held_brake_brings_moving_car_to_a_complete_stop() {
        let mut inputs = vec![hold(1.0, 0.0, 0.0); FIXED_HZ as usize];
        inputs.extend(vec![hold(0.0, 1.0, 0.0); 35]);
        let snaps = run(straight_track(), &inputs);

        let peak = snaps[FIXED_HZ as usize - 1];
        assert!(peak.forward_speed > 15.0);

        let last = *snaps.last().unwrap();
        assert_eq!(last.forward_speed, 0.0, "car must come to a complete stop");
        assert_eq!(last.velocity, Vec2::ZERO);
    }

    #[test]
    fn held_brake_at_standstill_engages_reverse_and_throttle_recovers() {
        // Hold brake for 64 ticks (1 s).
        let mut inputs = vec![hold(0.0, 1.0, 0.0); FIXED_HZ as usize];
        // Then hold throttle for 32 ticks (0.5 s) to brake out of reverse and start moving forward.
        inputs.extend(vec![hold(1.0, 0.0, 0.0); 32]);
        let snaps = run(straight_track(), &inputs);

        // At tick 16 (0.25 s), reverse engages; by tick 63 (end of 1 s brake), car has reversed.
        let reverse_peak = snaps[FIXED_HZ as usize - 1];
        assert!(
            reverse_peak.forward_speed < -3.0,
            "must be moving in reverse: {}",
            reverse_peak.forward_speed
        );
        assert!(
            reverse_peak.forward_speed >= -REVERSE_MAX_SPEED,
            "reverse speed must be capped"
        );
        assert!(
            reverse_peak.pose.x < 0.0,
            "must have moved backward, at {:?}",
            reverse_peak.pose
        );

        // After applying throttle, the car recovers from reverse and accelerates forward.
        let last = *snaps.last().unwrap();
        assert!(
            last.forward_speed > 0.0,
            "throttle must recover to forward motion: {}",
            last.forward_speed
        );
    }

    #[test]
    fn steering_turns_moving_car_and_does_not_turn_at_rest() {
        // 1. Steering at rest produces zero angular rotation.
        let steer_rest_inputs = vec![hold(0.0, 0.0, 1.0); FIXED_HZ as usize];
        let snaps_rest = run(straight_track(), &steer_rest_inputs);
        let initial_heading = snaps_rest[0].heading;
        let last_rest = snaps_rest.last().unwrap();
        assert_eq!(
            last_rest.heading, initial_heading,
            "car must not turn at rest"
        );
        assert_eq!(
            last_rest.pose, snaps_rest[0].pose,
            "car must not move at rest"
        );

        // 2. Steering left while moving forward turns heading counter-clockwise and curves path.
        let steer_left_inputs = vec![hold(1.0, 0.0, 1.0); FIXED_HZ as usize];
        let snaps_left = run(straight_track(), &steer_left_inputs);
        let last_left = snaps_left.last().unwrap();
        assert!(
            last_left.heading > initial_heading + 0.3,
            "heading must rotate counter-clockwise (got {})",
            last_left.heading
        );
        assert!(
            last_left.pose.y > 1.0,
            "path must curve left in y (got {:?}",
            last_left.pose
        );

        // 3. Steering right while moving forward turns heading clockwise.
        let steer_right_inputs = vec![hold(1.0, 0.0, -1.0); FIXED_HZ as usize];
        let snaps_right = run(straight_track(), &steer_right_inputs);
        let last_right = snaps_right.last().unwrap();
        assert!(
            last_right.heading < initial_heading - 0.3,
            "heading must rotate clockwise (got {})",
            last_right.heading
        );
        assert!(
            last_right.pose.y < -1.0,
            "path must curve right in -y (got {:?}",
            last_right.pose
        );
    }

    #[test]
    fn multiple_cars_stage_in_order_behind_start_line() {
        let track = straight_track();
        let mut sim = Sim::new(track, 4);
        let snaps = sim.tick(&[]);
        assert_eq!(snaps.len(), 4);

        // Lead car at index 0 spawns exactly at the start line (0 distance back).
        assert_eq!(snaps[0].pose, Vec2::new(0.0, 0.0));
        // Trailing cars staged backwards along the loop (segment 3 goes (0, 200) -> (0, 0), so -y in that segment).
        // In our loop: segment 0: (0,0)->(2000,0); segment 3: (0,200)->(0,0).
        // Spawning distance back from (0,0) along segment 3 means y = CAR_SPACING * i.
        for (i, snap) in snaps.iter().enumerate().skip(1) {
            assert!((snap.pose.y - i as f32 * CAR_SPACING).abs() < 1e-4);
            assert!((snap.pose.x).abs() < 1e-4);
            assert_eq!(snap.velocity, Vec2::ZERO);
        }
    }

    #[test]
    fn identical_input_streams_yield_identical_snapshot_sequences() {
        let track_a = straight_track();
        let track_b = straight_track();
        let mut sim_a = Sim::new(track_a, 4);
        let mut sim_b = Sim::new(track_b, 4);

        // Generate a 500-tick scripted sequence exercising all four cars differently:
        // car 0: acceleration and steering zig-zag
        // car 1: accelerate then full brake to stop
        // car 2: brake at standstill to reverse, steer in reverse
        // car 3: coasting with drag
        let mut scripted_ticks = Vec::with_capacity(500);
        for tick in 0..500 {
            let t = tick as f32 * FIXED_DT;
            let input_0 = hold(1.0, 0.0, (t * 2.0).sin());
            let input_1 = if tick < 100 {
                hold(1.0, 0.0, 0.0)
            } else {
                hold(0.0, 1.0, 0.0)
            };
            let input_2 = if tick < 200 {
                hold(0.0, 1.0, 0.5)
            } else {
                hold(1.0, 0.0, -0.5)
            };
            let input_3 = if tick < 30 {
                hold(1.0, 0.0, 0.0)
            } else {
                hold(0.0, 0.0, 0.0)
            };
            scripted_ticks.push(vec![input_0, input_1, input_2, input_3]);
        }

        let mut snaps_a = Vec::with_capacity(500);
        let mut snaps_b = Vec::with_capacity(500);

        for inputs in &scripted_ticks {
            snaps_a.push(sim_a.tick(inputs));
            snaps_b.push(sim_b.tick(inputs));
        }

        assert_eq!(snaps_a.len(), 500);
        assert_eq!(snaps_b.len(), 500);

        // Every single snapshot across all 500 ticks and all 4 cars must match bit-for-bit.
        for tick in 0..500 {
            for car in 0..4 {
                let sa = snaps_a[tick][car];
                let sb = snaps_b[tick][car];
                assert_eq!(sa, sb, "snapshot mismatch at tick {tick}, car {car}");
                assert_eq!(
                    sa.pose.x.to_bits(),
                    sb.pose.x.to_bits(),
                    "pose.x bit mismatch at tick {tick}, car {car}"
                );
                assert_eq!(
                    sa.pose.y.to_bits(),
                    sb.pose.y.to_bits(),
                    "pose.y bit mismatch at tick {tick}, car {car}"
                );
                assert_eq!(
                    sa.heading.to_bits(),
                    sb.heading.to_bits(),
                    "heading bit mismatch at tick {tick}, car {car}"
                );
                assert_eq!(
                    sa.velocity.x.to_bits(),
                    sb.velocity.x.to_bits(),
                    "velocity.x bit mismatch at tick {tick}, car {car}"
                );
                assert_eq!(
                    sa.velocity.y.to_bits(),
                    sb.velocity.y.to_bits(),
                    "velocity.y bit mismatch at tick {tick}, car {car}"
                );
                assert_eq!(
                    sa.forward_speed.to_bits(),
                    sb.forward_speed.to_bits(),
                    "forward_speed bit mismatch at tick {tick}, car {car}"
                );
            }
        }
    }
}
