//! The simulation seam: Track + per-Car inputs in, snapshots out, at a fixed
//! 64 Hz step. Plain Rust, no engine types — the Bevy shell renders snapshots
//! and maps keys to [`CarInput`]; all game rules live here.

use glam::Vec2;

use crate::ai::{AiDriver, AiView};
use crate::track::{Surface, Track};

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
    /// Handbrake demand: cuts rear/lateral grip to provoke Drift.
    pub handbrake: bool,
}

impl Default for CarInput {
    fn default() -> Self {
        CarInput {
            throttle: 0.0,
            brake: 0.0,
            steer: 0.0,
            handbrake: false,
        }
    }
}

/// Standard race length in completed laps.
pub const TOTAL_LAPS: u32 = 3;

/// Default countdown duration in ticks (3.0 seconds at 64 Hz).
pub const DEFAULT_COUNTDOWN_TICKS: u32 = 192;

/// The high-level phase of a Race.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RacePhase {
    /// Fixed-tick countdown before racing begins. Controls are locked.
    Countdown { ticks_remaining: u32 },
    /// Active racing across 3 laps.
    Racing,
    /// The race has concluded (all laps completed).
    Finished,
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
    /// Steering demand applied on this tick, -1..1. Positive is left
    /// (counter-clockwise); drives the front-wheel visual angle.
    pub steer: f32,
    /// Surface the Car is currently driving on.
    pub surface: Surface,
    /// Whether the Car contacted a boundary wall during this tick.
    pub wall_contact: bool,
    /// Whether the Car is currently in a controlled Drift state.
    pub drifting: bool,
    /// Current phase of the Race.
    pub phase: RacePhase,
    /// Number of fully completed laps (0..=3).
    pub completed_laps: u32,
    /// Recorded elapsed times in seconds for completed laps [lap 1, lap 2, lap 3].
    pub lap_times: [Option<f32>; TOTAL_LAPS as usize],
    /// Elapsed time in seconds during the current in-progress lap.
    pub current_lap_time: f32,
    /// Best completed lap time in seconds, if at least one lap has been finished.
    pub best_lap_time: Option<f32>,
    /// Live race position (1st, 2nd, 3rd, 4th; 1-indexed).
    pub position: usize,
}

/// The headless world: Cars advancing over a Track at the fixed step.
pub struct Sim {
    track: Track,
    cars: Vec<CarState>,
    phase: RacePhase,
    /// Fixed ticks elapsed since the race phase began (green). The Race clock
    /// lives in the simulation so every consumer (HUD, overlays) reads one clock.
    racing_ticks: u32,
}

struct CarState {
    pose: Vec2,
    heading: f32,
    velocity: Vec2,
    steer: f32,
    reverse: bool,
    standstill_ticks: u32,
    completed_laps: u32,
    current_lap_ticks: u32,
    next_checkpoint: usize,
    last_cleared_checkpoint: usize,
    lap_times: [Option<f32>; TOTAL_LAPS as usize],
    best_lap_time: Option<f32>,
    ai: Option<AiDriver>,
}
impl Sim {
    /// Builds a sim with `car_count` Cars staged behind the Track's start
    /// line, starting in the `Racing` phase.
    pub fn new(track: Track, car_count: usize) -> Sim {
        Self::new_with_phase(track, car_count, RacePhase::Racing)
    }

    /// Builds a full race sim starting in the `Countdown` phase with AI opponents.
    pub fn new_race(track: Track, car_count: usize) -> Sim {
        let mut sim = Self::new_with_phase(
            track,
            car_count,
            RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS,
            },
        );
        sim.enable_ai_opponents();
        sim
    }

    /// Builds a sim with an explicit initial `RacePhase`.
    pub fn new_with_phase(track: Track, car_count: usize, phase: RacePhase) -> Sim {
        let cars = (0..car_count)
            .map(|i| {
                let pose = track.spawn_pose(i as f32 * CAR_SPACING);
                // Face each car along the leg it stages on so the grid lines up
                // with the track instead of starting broadside to it.
                let seg = track.nearest_segment(pose).segment_index;
                let dir = track.points[seg + 1] - track.points[seg];
                let heading = dir.y.atan2(dir.x);
                CarState {
                    pose,
                    heading,
                    velocity: Vec2::ZERO,
                    steer: 0.0,
                    reverse: false,
                    standstill_ticks: 0,
                    completed_laps: 0,
                    current_lap_ticks: 0,
                    next_checkpoint: 1,
                    last_cleared_checkpoint: 0,
                    lap_times: [None; TOTAL_LAPS as usize],
                    best_lap_time: None,
                    ai: None,
                }
            })
            .collect();
        Sim {
            track,
            cars,
            phase,
            racing_ticks: 0,
        }
    }

    /// Enables AI driver controllers for trailing cars (indices 1..car_count).
    pub fn enable_ai_opponents(&mut self) {
        for (i, car) in self.cars.iter_mut().enumerate().skip(1) {
            car.ai = Some(AiDriver::new(i));
        }
    }

    /// Sets (or clears) the AI driver for one car. Out-of-range indices are ignored.
    pub fn set_ai(&mut self, car_index: usize, ai: Option<AiDriver>) {
        if let Some(car) = self.cars.get_mut(car_index) {
            car.ai = ai;
        }
    }

    /// The parsed Track this sim runs on.
    pub fn track(&self) -> &Track {
        &self.track
    }

    /// Current phase of the Race.
    pub fn phase(&self) -> RacePhase {
        self.phase
    }

    /// Fixed ticks elapsed since the race phase began (green); 0 during countdown.
    pub fn racing_ticks(&self) -> u32 {
        self.racing_ticks
    }

    /// Whether car `car_index` currently has an AI driver attached.
    pub fn ai_enabled(&self, car_index: usize) -> bool {
        self.cars.get(car_index).is_some_and(|car| car.ai.is_some())
    }

    /// Puts the simulation into a countdown with the specified number of ticks.
    pub fn start_countdown(&mut self, ticks: u32) {
        self.phase = RacePhase::Countdown {
            ticks_remaining: ticks,
        };
    }

    /// Advances the world one fixed step. For player-controlled cars, inputs are
    /// consumed from `inputs` (defaulting to neutral if missing). For AI-controlled
    /// cars, the internal AI driver computes input unless overridden by an explicit
    /// entry in `inputs`.
    pub fn tick(&mut self, inputs: &[CarInput]) -> Vec<CarSnapshot> {
        // Phase management: countdown progression
        let controls_locked = match &mut self.phase {
            RacePhase::Countdown { ticks_remaining } => {
                if *ticks_remaining > 0 {
                    *ticks_remaining -= 1;
                    if *ticks_remaining == 0 {
                        self.phase = RacePhase::Racing;
                        false
                    } else {
                        true
                    }
                } else {
                    self.phase = RacePhase::Racing;
                    false
                }
            }
            RacePhase::Racing => {
                self.racing_ticks += 1;
                false
            }
            RacePhase::Finished => false,
        };
        let total_cps = self.track.points.len() - 1;
        let cp_radius = self.track.wall_distance() * 1.5;

        // Step all cars. AI drivers perceive the field (every Car's pose and
        // velocity) plus the Track, and answer with player-identical inputs.
        let field: Vec<(Vec2, Vec2)> = self.cars.iter().map(|c| (c.pose, c.velocity)).collect();
        let mut step_results = Vec::with_capacity(self.cars.len());
        for (i, car) in self.cars.iter_mut().enumerate() {
            let effective_input = if controls_locked {
                CarInput::default()
            } else if let Some(ai) = &mut car.ai {
                if inputs.len() > i && inputs[i] != CarInput::default() {
                    inputs[i]
                } else {
                    ai.compute_input(AiView {
                        car_index: i,
                        pose: car.pose,
                        heading: car.heading,
                        velocity: car.velocity,
                        track: &self.track,
                        field: &field,
                    })
                }
            } else {
                inputs.get(i).copied().unwrap_or_default()
            };

            car.steer = effective_input.steer;
            let (surface, wall_contact, drifting) = step_car(car, effective_input, &self.track);
            step_results.push((surface, wall_contact, drifting));

            // In Racing phase, accumulate lap time and check ordered progress
            if self.phase == RacePhase::Racing {
                advance_lap_progress(car, &self.track, total_cps, cp_radius);
            }
        }

        // Car-to-Car collisions: pairwise oriented-box collision response
        resolve_car_collisions(&mut self.cars);

        // If any car completed all laps, the race transitions to Finished
        if self.phase == RacePhase::Racing
            && self.cars.iter().any(|c| c.completed_laps >= TOTAL_LAPS)
        {
            self.phase = RacePhase::Finished;
        }

        // Calculate live positions: sorted by progress score
        let positions = compute_positions(&self.cars, &self.track);

        // Build final snapshots
        self.cars
            .iter()
            .zip(positions.iter())
            .zip(step_results.iter())
            .map(|((car, &position), &(surface, wall_contact, drifting))| {
                Self::make_snapshot(car, surface, wall_contact, drifting, position, self.phase)
            })
            .collect()
    }

    /// Projects each Car's current state into a snapshot without stepping.
    /// Surface and wall contact are sampled at the current pose; `drifting`
    /// is false because drift state is only meaningful after a step.
    pub fn snapshots(&self) -> Vec<CarSnapshot> {
        let positions = compute_positions(&self.cars, &self.track);
        self.cars
            .iter()
            .zip(positions.iter())
            .map(|(car, &position)| {
                Self::make_snapshot(
                    car,
                    self.track.sample_surface(car.pose),
                    self.track.wall_contact(car.pose).is_some(),
                    false,
                    position,
                    self.phase,
                )
            })
            .collect()
    }

    /// Shared CarState → CarSnapshot projection for tick and no-tick paths.
    fn make_snapshot(
        car: &CarState,
        surface: Surface,
        wall_contact: bool,
        drifting: bool,
        position: usize,
        phase: RacePhase,
    ) -> CarSnapshot {
        CarSnapshot {
            pose: car.pose,
            heading: car.heading,
            velocity: car.velocity,
            forward_speed: car.velocity.dot(forward(car.heading)),
            steer: car.steer,
            surface,
            wall_contact,
            drifting,
            phase,
            completed_laps: car.completed_laps,
            lap_times: car.lap_times,
            current_lap_time: car.current_lap_ticks as f32 * FIXED_DT,
            best_lap_time: car.best_lap_time,
            position,
        }
    }
}

/// Drag, rolling resistance, and traction modifiers for a Track surface.
struct SurfacePhysics {
    drag_factor: f32,
    rolling_factor: f32,
    traction_factor: f32,
}

impl SurfacePhysics {
    fn for_surface(surface: Surface) -> Self {
        match surface {
            Surface::Road => SurfacePhysics {
                drag_factor: 1.0,
                rolling_factor: 1.0,
                traction_factor: 1.0,
            },
            Surface::Gravel => SurfacePhysics {
                drag_factor: 1.3,
                rolling_factor: 1.6,
                traction_factor: 0.75,
            },
            Surface::Grass => SurfacePhysics {
                drag_factor: 1.8,
                rolling_factor: 3.0,
                traction_factor: 0.5,
            },
        }
    }
}

/// Advances one Car's lap/checkpoint progress for a single tick: accumulates
/// the current lap time and, on ordered checkpoint entry, records lap times,
/// best lap, completed laps, and the next checkpoint to clear.
fn advance_lap_progress(car: &mut CarState, track: &Track, total_cps: usize, cp_radius: f32) {
    car.current_lap_ticks += 1;

    let target_cp = car.next_checkpoint;
    let cp_pos = track.points[target_cp];
    let entry_dir = if target_cp == 0 {
        cp_pos - track.points[total_cps - 1]
    } else {
        cp_pos - track.points[target_cp - 1]
    };
    if (car.pose - cp_pos).length() < cp_radius && car.velocity.dot(entry_dir) > 0.0 {
        car.last_cleared_checkpoint = target_cp;
        if target_cp == 0 {
            // Crossed start/finish having visited all checkpoints in order
            if car.completed_laps < TOTAL_LAPS {
                let lap_time = car.current_lap_ticks as f32 * FIXED_DT;
                car.lap_times[car.completed_laps as usize] = Some(lap_time);
                car.best_lap_time = Some(match car.best_lap_time {
                    Some(best) => best.min(lap_time),
                    None => lap_time,
                });
                car.completed_laps += 1;
                car.current_lap_ticks = 0;
            }
            car.next_checkpoint = 1;
        } else {
            car.next_checkpoint = (target_cp + 1) % total_cps;
        }
    }
}

/// Computes live race positions from progress scores, returning a 1-indexed
/// position per car (index-aligned with `cars`). Higher completed laps and
/// cleared checkpoints rank ahead; closer distance to the next checkpoint
/// breaks ties.
fn compute_positions(cars: &[CarState], track: &Track) -> Vec<usize> {
    let mut car_scores: Vec<(usize, f32)> = cars
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let next_cp = track.points[c.next_checkpoint];
            let dist_to_next = (c.pose - next_cp).length();
            let score = c.completed_laps as f32 * 10000.0
                + c.last_cleared_checkpoint as f32 * 100.0
                - (dist_to_next / 1000.0);
            (i, score)
        })
        .collect();
    car_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut positions = vec![1; cars.len()];
    for (pos_idx, (car_idx, _)) in car_scores.iter().enumerate() {
        positions[*car_idx] = pos_idx + 1;
    }
    positions
}

fn step_car(car: &mut CarState, input: CarInput, track: &Track) -> (Surface, bool, bool) {
    let initial_surface = track.sample_surface(car.pose);
    let physics = SurfacePhysics::for_surface(initial_surface);

    let fwd = forward(car.heading);
    let left = Vec2::new(-fwd.y, fwd.x);
    let mut vf = car.velocity.dot(fwd);
    let vl = car.velocity.dot(left);

    let drag = (DRAG_COEFF * physics.drag_factor) * vf * vf.abs()
        + (ROLLING_RESIST * physics.rolling_factor) * vf;

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
            let drive = input.throttle * ENGINE_ACCEL * physics.traction_factor;
            let brake = input.brake * BRAKE_ACCEL * physics.traction_factor;
            let handbrake_drag = if input.handbrake {
                HANDBRAKE_DECEL
            } else {
                0.0
            };

            let accel = drive - drag - brake - handbrake_drag;
            vf += accel * FIXED_DT;

            if (input.brake > 0.0 || input.handbrake) && vf < 0.0 {
                vf = 0.0;
            }
        }
    } else {
        if vf.abs() < STANDSTILL_THRESHOLD && input.brake <= 0.0 {
            vf = 0.0;
            car.reverse = false;
            car.standstill_ticks = 0;
        } else {
            let reverse_drive = -input.brake * REVERSE_ACCEL * physics.traction_factor;
            let throttle_brake = input.throttle * BRAKE_ACCEL * physics.traction_factor;

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

    // Steering: angular velocity scales with forward speed, weight transfer, and steering input.
    // Braking shifts weight to the front wheels, increasing steering bite;
    // acceleration shifts weight to the rear, causing understeer.
    let weight_bias = if input.brake > 0.0 {
        1.0 + input.brake * BRAKE_WEIGHT_TRANSFER
    } else if input.throttle > 0.0 {
        1.0 - input.throttle * ACCEL_WEIGHT_TRANSFER
    } else {
        1.0
    };
    let angular_vel = (vf * STEER_SENSITIVITY * weight_bias * input.steer)
        .clamp(-MAX_ANGULAR_VEL, MAX_ANGULAR_VEL);
    car.heading += angular_vel * FIXED_DT;

    let fwd_new = forward(car.heading);
    let left_new = Vec2::new(-fwd_new.y, fwd_new.x);

    // Kinematic slip: inertia carries velocity forward as heading turns.
    let v_pre = fwd * vf + left * vl;
    let vf_post = v_pre.dot(fwd_new);
    let vl_post = v_pre.dot(left_new);

    // Lateral grip: tires resist sideways sliding.
    // Handbrake dramatically reduces lateral grip to provoke Drift.
    // Braking unloads the rear wheels, slightly reducing lateral grip.
    let handbrake_grip_mult = if input.handbrake {
        HANDBRAKE_LATERAL_GRIP_FACTOR
    } else {
        1.0
    };
    let rear_unload_mult = if input.brake > 0.0 {
        1.0 - input.brake * BRAKE_REAR_UNLOAD
    } else {
        1.0
    };
    let effective_grip =
        LATERAL_GRIP_RATE * physics.traction_factor * handbrake_grip_mult * rear_unload_mult;
    let grip_decay = (effective_grip * FIXED_DT).clamp(0.0, 1.0);
    let vl_damped = vl_post * (1.0 - grip_decay);

    // Update car velocity from forward and damped lateral components.
    car.velocity = fwd_new * vf_post + left_new * vl_damped;
    car.pose += car.velocity * FIXED_DT;

    // Wall collision response: bounce with speed loss and inward reflection.
    let wall_contact = if let Some(contact) = track.wall_contact(car.pose) {
        car.pose += contact.normal * contact.penetration;

        let vn = car.velocity.dot(contact.normal);
        let v_norm = contact.normal * vn;
        let v_tangent = car.velocity - v_norm;

        let reflected_vn = if vn < 0.0 { -WALL_RESTITUTION * vn } else { vn };
        let damped_vt = v_tangent * WALL_FRICTION;
        car.velocity = damped_vt + contact.normal * reflected_vn;

        true
    } else {
        false
    };

    // Drift state: controlled lateral slip from grip model, distinct from wall-slide.
    let vf_final = car.velocity.dot(fwd_new);
    let vl_final = car.velocity.dot(left_new);
    let lateral_slip = vl_final.abs();
    let drifting = !wall_contact
        && vf_final.abs() > DRIFT_SPEED_MIN
        && (lateral_slip > DRIFT_LATERAL_SLIP_MIN
            || (input.handbrake && lateral_slip > HANDBRAKE_DRIFT_SLIP_MIN));

    // Sample surface at post-move pose so CarSnapshot matches the final pose.
    let post_surface = track.sample_surface(car.pose);
    (post_surface, wall_contact, drifting)
}

/// Grid slot spacing in world units. Must clear the car's visual length (~4.4
/// nose to tail) so staged cars never render bumper-to-bumper at the start.
const CAR_SPACING: f32 = 5.0;
/// Unit vector along a heading (0 faces +x, positive is counter-clockwise).
fn forward(heading: f32) -> Vec2 {
    Vec2::new(heading.cos(), heading.sin())
}

/// Engine acceleration at full throttle, in world units per second squared.
const ENGINE_ACCEL: f32 = 24.0;
/// Quadratic air drag coefficient, per (unit/s)^2.
const DRAG_COEFF: f32 = 0.0075;
/// Linear rolling resistance, per unit/s.
const ROLLING_RESIST: f32 = 0.6;
/// Terminal speed reached at full throttle when engine accel balances quadratic
/// drag and rolling resistance (derived from ENGINE_ACCEL, DRAG_COEFF, ROLLING_RESIST;
/// pinned by the terminal_speed_matches_top_speed_const test).
pub const TOP_SPEED: f32 = 29.3;
/// Braking deceleration at full brake, in world units per second squared.
pub const BRAKE_ACCEL: f32 = 36.0;
/// Reverse acceleration at full demand, in world units per second squared.
const REVERSE_ACCEL: f32 = 12.0;
/// Top speed while reversing, in world units per second.
const REVERSE_MAX_SPEED: f32 = 10.0;
/// Speed threshold below which the Car is considered at a standstill.
const STANDSTILL_THRESHOLD: f32 = 0.05;
/// Number of fixed ticks holding brake at a standstill to engage reverse (0.25 s at 64 Hz).
const STANDSTILL_REVERSE_TICKS: u32 = 16;
/// Maximum angular velocity when steering, in radians per second (~180 deg/s).
pub const MAX_ANGULAR_VEL: f32 = 3.2;
/// Steering sensitivity factor translating forward speed and steer input into angular velocity.
pub const STEER_SENSITIVITY: f32 = 0.15;
/// Coefficient of restitution for boundary wall collisions (normal velocity bounce).
const WALL_RESTITUTION: f32 = 0.4;
/// Tangential friction coefficient during boundary wall collisions.
const WALL_FRICTION: f32 = 0.75;
/// Weight transfer factor increasing front steering bite under braking.
const BRAKE_WEIGHT_TRANSFER: f32 = 0.45;
/// Weight transfer factor decreasing front steering bite under acceleration (understeer).
const ACCEL_WEIGHT_TRANSFER: f32 = 0.08;
/// Rear axle grip reduction factor under braking.
const BRAKE_REAR_UNLOAD: f32 = 0.25;
/// Lateral grip restitution rate damping sideways velocity (1/s).
pub const LATERAL_GRIP_RATE: f32 = 24.0;
/// Lateral grip multiplier when handbrake is engaged (cutting grip).
const HANDBRAKE_LATERAL_GRIP_FACTOR: f32 = 0.12;
/// Deceleration applied along heading when handbrake is engaged.
const HANDBRAKE_DECEL: f32 = 12.0;
/// Minimum forward speed to be eligible for the Drift state.
const DRIFT_SPEED_MIN: f32 = 3.0;
/// Minimum lateral slip speed to enter the Drift state.
pub const DRIFT_LATERAL_SLIP_MIN: f32 = 1.2;
/// Minimum lateral slip speed under handbrake to enter the Drift state.
const HANDBRAKE_DRIFT_SLIP_MIN: f32 = 0.4;
/// Half length of the Car's collision box in world units, covering the nose
/// (x = 2.1) through the rear wing (x = -2.2).
pub const CAR_HALF_LENGTH: f32 = 2.2;
/// Half width of the Car's collision box in world units, covering the outer
/// tire edges (|y| = 1.29).
pub const CAR_HALF_WIDTH: f32 = 1.3;
/// Coefficient of restitution for car-to-car collisions.
pub const CAR_COLLISION_RESTITUTION: f32 = 0.5;

/// Resolves pairwise oriented-box collisions between all cars with separation
/// and elastic impulse (SAT on the two heading frames).
///
/// The old bounding circle (radius 1.6, diameter 3.2) was shorter than the
/// visual Car (~4.3 nose to tail), so bumper-to-bumper cars overlapped by a
/// full unit before the resolver engaged. The box matches the rendered
/// chassis, nose, wing, and tire extents.
fn resolve_car_collisions(cars: &mut [CarState]) {
    let num_cars = cars.len();
    for i in 0..num_cars {
        for j in (i + 1)..num_cars {
            if let Some((normal, penetration)) =
                obb_penetration(cars[i].pose, cars[i].heading, cars[j].pose, cars[j].heading)
            {
                cars[i].pose -= normal * (penetration * 0.5);
                cars[j].pose += normal * (penetration * 0.5);

                let rel_vel = cars[j].velocity - cars[i].velocity;
                let vn = rel_vel.dot(normal);
                if vn < 0.0 {
                    let impulse = -(1.0 + CAR_COLLISION_RESTITUTION) * vn * 0.5;
                    cars[i].velocity -= normal * impulse;
                    cars[j].velocity += normal * impulse;
                }
            }
        }
    }
}

/// SAT overlap of two Car boxes. Returns the minimum-penetration world-space
/// normal (pointing from `pose_a` toward `pose_b`) and penetration depth, or
/// `None` when separated on any of the four box axes.
fn obb_penetration(
    pose_a: Vec2,
    heading_a: f32,
    pose_b: Vec2,
    heading_b: f32,
) -> Option<(Vec2, f32)> {
    let fwd_a = forward(heading_a);
    let left_a = Vec2::new(-fwd_a.y, fwd_a.x);
    let fwd_b = forward(heading_b);
    let left_b = Vec2::new(-fwd_b.y, fwd_b.x);
    let delta = pose_b - pose_a;
    let axes = [fwd_a, left_a, fwd_b, left_b];
    let mut best: Option<(Vec2, f32)> = None;
    for axis in axes {
        let extent_a =
            CAR_HALF_LENGTH * fwd_a.dot(axis).abs() + CAR_HALF_WIDTH * left_a.dot(axis).abs();
        let extent_b =
            CAR_HALF_LENGTH * fwd_b.dot(axis).abs() + CAR_HALF_WIDTH * left_b.dot(axis).abs();
        let dist = delta.dot(axis).abs();
        let overlap = extent_a + extent_b - dist;
        if overlap <= 0.0 {
            return None;
        }
        let sign = if delta.dot(axis) < 0.0 { -1.0 } else { 1.0 };
        let normal = axis * sign;
        if best.is_none_or(|(_, depth)| overlap < depth) {
            best = Some((normal, overlap));
        }
    }
    best
}

/// Wraps an angle delta to the range $[-\pi, \pi]$ taking the shortest rotational path.
pub fn wrap_angle(mut delta: f32) -> f32 {
    while delta > std::f32::consts::PI {
        delta -= 2.0 * std::f32::consts::PI;
    }
    while delta < -std::f32::consts::PI {
        delta += 2.0 * std::f32::consts::PI;
    }
    delta
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track::{Surface, SAMPLE_CIRCUIT};

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
            handbrake: false,
        }
    }

    #[test]
    fn wrap_angle_normalizes_to_pi_range() {
        assert_eq!(wrap_angle(0.0), 0.0);
        assert_eq!(wrap_angle(2.0 * std::f32::consts::PI), 0.0);
        let pi = std::f32::consts::PI;
        assert!((wrap_angle(pi + 0.1) - (-pi + 0.1)).abs() < 1e-5);
        assert!((wrap_angle(-pi - 0.1) - (pi - 0.1)).abs() < 1e-5);
    }

    #[test]
    fn terminal_speed_matches_top_speed_const() {
        let track = straight_track();
        let full_throttle = vec![
            CarInput {
                throttle: 1.0,
                ..CarInput::default()
            };
            640
        ];
        let snaps = run(track, &full_throttle);
        let plateau = snaps.last().unwrap().forward_speed;
        assert!(
            (plateau - TOP_SPEED).abs() < 0.5,
            "terminal speed {plateau} must stay within 0.5 of TOP_SPEED {}",
            TOP_SPEED
        );
    }

    #[test]
    fn racing_clock_counts_from_green_and_snapshots_project_without_stepping() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut sim = Sim::new_race(track, 2);
        assert_eq!(sim.racing_ticks(), 0);

        // Projection without stepping: phase, pose, and countdown intact.
        let pre = sim.snapshots();
        assert_eq!(pre.len(), 2);
        assert_eq!(
            pre[0].phase,
            RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS
            }
        );
        assert_eq!(sim.racing_ticks(), 0);

        // The clock counts only green ticks, never countdown ticks.
        for _ in 0..DEFAULT_COUNTDOWN_TICKS + 5 {
            sim.tick(&[CarInput::default(), CarInput::default()]);
        }
        assert_eq!(sim.racing_ticks(), 5);
        assert_eq!(sim.snapshots()[0].phase, RacePhase::Racing);
    }

    fn run(track: Track, inputs: &[CarInput]) -> Vec<CarSnapshot> {
        let mut sim = Sim::new(track, 1);
        let mut snaps = Vec::with_capacity(inputs.len());
        for input in inputs {
            snaps.extend(sim.tick(&[*input]));
        }
        snaps
    }
    fn drive_step_towards_waypoints(
        sim: &mut Sim,
        waypoints: &[Vec2],
        current_wp: &mut usize,
        last_pose: &mut Vec2,
        last_heading: &mut f32,
    ) -> CarSnapshot {
        let target = waypoints[*current_wp];
        let to_target = target - *last_pose;
        let target_angle = to_target.y.atan2(to_target.x);
        let angle_diff = wrap_angle(target_angle - *last_heading);

        let steer = (angle_diff * 2.5).clamp(-1.0, 1.0);
        let throttle = if angle_diff.abs() > 0.4 { 0.5 } else { 1.0 };
        let brake = if angle_diff.abs() > 0.8 { 0.3 } else { 0.0 };

        let snap = sim.tick(&[CarInput {
            throttle,
            brake,
            steer,
            handbrake: false,
        }])[0];
        *last_pose = snap.pose;
        *last_heading = snap.heading;

        if (target - snap.pose).length() < 14.0 {
            *current_wp = (*current_wp + 1) % waypoints.len();
        }

        snap
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
    fn spawned_cars_face_their_legs_travel_direction() {
        let track = straight_track();
        let mut sim = Sim::new(track, 4);
        let snaps = sim.tick(&[]);
        // Lead car sits exactly on the start vertex: faces the opening straight (+x).
        assert!(
            (snaps[0].heading - 0.0).abs() < 1e-4,
            "lead car must face the opening straight (got {})",
            snaps[0].heading
        );
        // Trailing cars sit on the final leg: face its travel direction (south, -y).
        for (i, snap) in snaps.iter().enumerate().skip(1) {
            assert!(
                (snap.heading + std::f32::consts::FRAC_PI_2).abs() < 1e-4,
                "car {i} must face its leg south (got {})",
                snap.heading
            );
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

    #[test]
    fn leaving_the_road_measurably_slows_the_car_versus_staying_on_it() {
        let track = straight_track();
        let mut sim_road = Sim::new(track.clone(), 1);
        let mut sim_grass = Sim::new(track, 1);

        // Both cars accelerate down the straight for 80 ticks to build speed and
        // clear the start corner (x > 25).
        let setup = vec![hold(1.0, 0.0, 0.0); 80];
        for input in &setup {
            sim_road.tick(&[*input]);
            sim_grass.tick(&[*input]);
        }

        // Car on road continues straight under full throttle for 120 ticks.
        let road_snaps = {
            let mut s = Vec::new();
            for _ in 0..120 {
                s.extend(sim_road.tick(&[hold(1.0, 0.0, 0.0)]));
            }
            s
        };
        // Car B steers off the road into grass (road half-width = 10.0),
        // straightens out, and continues under full throttle for 100 ticks.
        let mut grass_snaps = Vec::new();
        for _ in 0..50 {
            grass_snaps.extend(sim_grass.tick(&[hold(1.0, 0.0, 0.8)]));
        }
        for _ in 0..40 {
            grass_snaps.extend(sim_grass.tick(&[hold(1.0, 0.0, -0.6)]));
        }
        for _ in 0..100 {
            grass_snaps.extend(sim_grass.tick(&[hold(1.0, 0.0, 0.0)]));
        }
        let final_road = road_snaps.last().unwrap();
        let final_grass = grass_snaps.last().unwrap();
        // Confirm surfaces:
        assert_eq!(final_road.surface, Surface::Road);
        assert_eq!(final_grass.surface, Surface::Grass);
        assert!(
            final_grass.pose.y.abs() > 10.0,
            "car must be off-road: y={}",
            final_grass.pose.y
        );

        // Leaving the road measurably slows the Car versus staying on it:
        // Car on road has much higher speed and covered much more distance.
        assert!(
            final_road.forward_speed > final_grass.forward_speed * 1.5,
            "road speed {} must exceed grass speed {} by > 50%",
            final_road.forward_speed,
            final_grass.forward_speed
        );
        assert!(
            final_road.pose.x > final_grass.pose.x * 1.3,
            "road x {} must exceed grass x {}",
            final_road.pose.x,
            final_grass.pose.x
        );
    }

    #[test]
    fn wall_contact_bleeds_speed_and_reflects_the_car_without_hard_stopping() {
        let track = straight_track();
        let mut sim = Sim::new(track, 1);

        // Accelerate straight down the road first, then steer towards the outside wall at y = -20.0.
        let mut hit_tick = None;
        let mut prev_snap = None;
        let mut contact_snap = None;

        // 40 ticks straight to build speed.
        for _ in 0..40 {
            let snaps = sim.tick(&[hold(1.0, 0.0, 0.0)]);
            prev_snap = Some(snaps[0]);
        }

        // Next 35 ticks: turn heading towards negative y wall.
        for tick in 40..75 {
            let snaps = sim.tick(&[hold(1.0, 0.0, -0.6)]);
            let snap = snaps[0];
            if snap.wall_contact {
                hit_tick = Some(tick);
                contact_snap = Some(snap);
                break;
            }
            prev_snap = Some(snap);
        }

        // Next ticks: drive straight into the wall at y = -20.0.
        if hit_tick.is_none() {
            for tick in 75..200 {
                let snaps = sim.tick(&[hold(1.0, 0.0, 0.0)]);
                let snap = snaps[0];
                if snap.wall_contact {
                    hit_tick = Some(tick);
                    contact_snap = Some(snap);
                    break;
                }
                prev_snap = Some(snap);
            }
        }
        assert!(hit_tick.is_some(), "car must hit the wall");
        let prev = prev_snap.unwrap();
        let contact = contact_snap.unwrap();

        // Speed before contact vs speed after contact:
        let speed_before = prev.velocity.length();
        let speed_after = contact.velocity.length();

        // 1. Bleeds speed:
        assert!(
            speed_after < speed_before,
            "wall contact must bleed speed: after {speed_after} < before {speed_before}"
        );

        // 2. Without hard-stopping:
        assert!(
            speed_after > 1.0,
            "wall contact must not hard-stop: speed after was {speed_after}"
        );

        // 3. Reflects the Car: velocity normal to wall (y component heading towards negative y wall)
        // must reverse direction away from the wall (become positive).
        assert!(
            prev.velocity.y < 0.0,
            "must have been moving towards negative y wall"
        );
        assert!(
            contact.velocity.y > 0.0,
            "velocity must be reflected back inward (got {})",
            contact.velocity.y
        );

        // 4. Car stays within wall boundary (y >= -20.0):
        assert!(
            contact.pose.y >= -20.0 - 1e-3,
            "car must not penetrate past wall: got {}",
            contact.pose.y
        );
    }

    #[test]
    fn scripted_lap_of_sample_circuit_completes_cleanly() {
        use crate::track::SAMPLE_CIRCUIT;
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut sim = Sim::new(track, 1);

        let waypoints = [
            Vec2::new(120.0, 0.0),
            Vec2::new(160.0, 30.0),
            Vec2::new(160.0, 90.0),
            Vec2::new(110.0, 120.0),
            Vec2::new(40.0, 120.0),
            Vec2::new(0.0, 90.0),
            Vec2::new(0.0, 0.0),
        ];

        let mut last_pose = Vec2::new(0.0, 0.0);
        let mut last_heading = 0.0;
        let mut current_wp = 0;
        let mut observed_gravel = false;
        let mut total_wall_contacts = 0;
        let mut completed_lap = false;
        let mut final_snap = None;

        // Drive up to 1500 ticks (~23.4 seconds).
        for _tick in 0..1500 {
            let target = waypoints[current_wp];
            let to_target = target - last_pose;
            let target_angle = to_target.y.atan2(to_target.x);
            let mut angle_diff = target_angle - last_heading;
            while angle_diff > std::f32::consts::PI {
                angle_diff -= 2.0 * std::f32::consts::PI;
            }
            while angle_diff < -std::f32::consts::PI {
                angle_diff += 2.0 * std::f32::consts::PI;
            }

            let steer = (angle_diff * 2.5).clamp(-1.0, 1.0);
            let throttle = if angle_diff.abs() > 0.4 { 0.4 } else { 1.0 };
            let brake = if angle_diff.abs() > 0.8 { 0.3 } else { 0.0 };

            let snaps = sim.tick(&[CarInput {
                throttle,
                brake,
                steer,
                handbrake: false,
            }]);
            let snap = snaps[0];
            last_pose = snap.pose;
            last_heading = snap.heading;
            final_snap = Some(snap);

            if snap.surface == Surface::Gravel {
                observed_gravel = true;
            }
            if snap.wall_contact {
                total_wall_contacts += 1;
            }

            if (target - snap.pose).length() < 14.0 {
                current_wp += 1;
                if current_wp >= waypoints.len() {
                    completed_lap = true;
                    break;
                }
            }
        }

        assert!(completed_lap, "must complete the lap within 1500 ticks");
        assert!(observed_gravel, "must traverse gravel surface segments");
        assert_eq!(total_wall_contacts, 0, "clean lap should not contact walls");
        let last = final_snap.unwrap();
        assert!(
            last.pose.distance(Vec2::new(0.0, 0.0)) < 15.0,
            "final pose must be near start/finish line: got {:?}",
            last.pose
        );
    }

    #[test]
    fn handbrake_cornering_shows_higher_lateral_slip_than_plain_cornering() {
        // Accelerate along straight to reach cornering speed.
        let straight = straight_track();
        let mut sim_plain = Sim::new(straight.clone(), 1);
        let mut sim_handbrake = Sim::new(straight, 1);

        let accel_inputs = vec![hold(1.0, 0.0, 0.0); 64];
        for input in &accel_inputs {
            sim_plain.tick(&[*input]);
            sim_handbrake.tick(&[*input]);
        }

        // Both cars now enter a left turn: plain cornering vs handbrake cornering.
        let plain_turn = CarInput {
            throttle: 0.5,
            brake: 0.0,
            steer: 1.0,
            handbrake: false,
        };
        let handbrake_turn = CarInput {
            throttle: 0.5,
            brake: 0.0,
            steer: 1.0,
            handbrake: true,
        };

        let mut max_slip_plain = 0.0f32;
        let mut max_slip_handbrake = 0.0f32;

        for _ in 0..32 {
            let snap_plain = sim_plain.tick(&[plain_turn])[0];
            let snap_hb = sim_handbrake.tick(&[handbrake_turn])[0];

            let left_plain = Vec2::new(-snap_plain.heading.sin(), snap_plain.heading.cos());
            let left_hb = Vec2::new(-snap_hb.heading.sin(), snap_hb.heading.cos());

            let slip_plain = snap_plain.velocity.dot(left_plain).abs();
            let slip_hb = snap_hb.velocity.dot(left_hb).abs();

            max_slip_plain = max_slip_plain.max(slip_plain);
            max_slip_handbrake = max_slip_handbrake.max(slip_hb);
        }

        assert!(
            max_slip_handbrake > max_slip_plain * 1.5,
            "handbrake cornering must exhibit significantly higher lateral slip: hb={}, plain={}",
            max_slip_handbrake,
            max_slip_plain
        );
    }

    #[test]
    fn braking_into_a_corner_changes_trajectory_versus_coasting_in() {
        let straight = straight_track();
        let mut sim_coast = Sim::new(straight.clone(), 1);
        let mut sim_brake = Sim::new(straight, 1);

        let accel_inputs = vec![hold(1.0, 0.0, 0.0); 64];
        for input in &accel_inputs {
            sim_coast.tick(&[*input]);
            sim_brake.tick(&[*input]);
        }

        let coast_turn = CarInput {
            throttle: 0.0,
            brake: 0.0,
            steer: 1.0,
            handbrake: false,
        };
        let brake_turn = CarInput {
            throttle: 0.0,
            brake: 0.6,
            steer: 1.0,
            handbrake: false,
        };

        let mut snap_coast = None;
        let mut snap_brake = None;
        for _ in 0..32 {
            snap_coast = Some(sim_coast.tick(&[coast_turn])[0]);
            snap_brake = Some(sim_brake.tick(&[brake_turn])[0]);
        }

        let coast = snap_coast.unwrap();
        let brake = snap_brake.unwrap();
        // Weight transfer shifts load forward, sharpening rotation and altering trajectory.
        assert!(
            (coast.pose - brake.pose).length() > 2.0,
            "braking into corner must change trajectory versus coasting: coast={:?}, brake={:?}",
            coast.pose,
            brake.pose
        );
        assert!(
            (coast.heading - brake.heading).abs() > 0.05,
            "headings must diverge under weight transfer"
        );
    }

    #[test]
    fn drift_state_is_exposed_on_the_snapshot() {
        let straight = straight_track();
        let mut sim = Sim::new(straight, 1);

        // Straight-line driving: not drifting.
        let straight_inputs = vec![hold(1.0, 0.0, 0.0); 64];
        for input in &straight_inputs {
            let snap = sim.tick(&[*input])[0];
            assert!(
                !snap.drifting,
                "straight acceleration must not trigger drift"
            );
        }

        // Hard turn with handbrake at speed: enters drift.
        let mut observed_drift = false;
        let hb_turn = CarInput {
            throttle: 0.5,
            brake: 0.0,
            steer: 1.0,
            handbrake: true,
        };
        for _ in 0..32 {
            let snap = sim.tick(&[hb_turn])[0];
            if snap.drifting {
                observed_drift = true;
                break;
            }
        }
        assert!(
            observed_drift,
            "hard handbrake turn at speed must trigger drifting state on snapshot"
        );
    }

    #[test]
    fn handbrake_at_speed_stops_without_reversing() {
        let straight = straight_track();
        let mut sim = Sim::new(straight, 1);
        for _ in 0..64 {
            sim.tick(&[hold(1.0, 0.0, 0.0)]);
        }
        let hb_stop = CarInput {
            throttle: 0.0,
            brake: 0.0,
            steer: 0.0,
            handbrake: true,
        };
        for _ in 0..128 {
            let snap = sim.tick(&[hb_stop])[0];
            assert!(
                snap.forward_speed >= 0.0,
                "handbrake must not drive the car in reverse: got {}",
                snap.forward_speed
            );
        }
    }

    #[test]
    fn scripted_drive_of_sample_circuit_completes_laps_with_recorded_times() {
        let text = SAMPLE_CIRCUIT;
        let track = Track::parse(text).unwrap();
        let mut sim = Sim::new(track.clone(), 1);

        let waypoints = &track.points[..track.points.len() - 1];
        let mut current_wp = 1;
        let mut last_pose = sim.cars[0].pose;
        let mut last_heading = sim.cars[0].heading;
        let mut completed_lap_snap = None;

        // Drive for up to 1800 ticks
        for _ in 0..1800 {
            let snap = drive_step_towards_waypoints(
                &mut sim,
                waypoints,
                &mut current_wp,
                &mut last_pose,
                &mut last_heading,
            );
            if snap.completed_laps >= 1 {
                completed_lap_snap = Some(snap);
                break;
            }
        }

        let snap = completed_lap_snap.expect("must complete at least 1 lap");
        assert_eq!(snap.completed_laps, 1);
        assert!(snap.lap_times[0].is_some(), "lap time 1 must be recorded");
        let t1 = snap.lap_times[0].unwrap();
        assert!(
            t1 > 5.0 && t1 < 30.0,
            "lap time should be realistic: got {t1}"
        );
        assert_eq!(snap.best_lap_time, Some(t1));
    }

    #[test]
    fn cut_attempts_and_wrong_way_progress_never_increment_the_lap_counter() {
        let text = SAMPLE_CIRCUIT;
        let track = Track::parse(text).unwrap();

        // Reversal / wrong-way driving: turn 180 degrees around and drive the circuit in reverse order:
        // 7 -> 6 -> 5 -> 4 -> 3 -> 2 -> 1 -> 0.
        let mut sim_reverse = Sim::new(track.clone(), 1);
        sim_reverse.cars[0].heading += std::f32::consts::PI;

        let reverse_waypoints: Vec<Vec2> = (0..track.points.len() - 1)
            .rev()
            .map(|idx| track.points[idx])
            .collect();
        let mut current_wp = 0;
        let mut last_pose = sim_reverse.cars[0].pose;
        let mut last_heading = sim_reverse.cars[0].heading;
        let mut traversed_wrong_way = false;

        for _ in 0..2000 {
            let snap = drive_step_towards_waypoints(
                &mut sim_reverse,
                &reverse_waypoints,
                &mut current_wp,
                &mut last_pose,
                &mut last_heading,
            );
            assert_eq!(
                snap.completed_laps, 0,
                "wrong-way progress must never increment lap counter"
            );
            // When current_wp cycles through the reverse waypoints back to 0, it has traversed the track wrong-way
            if current_wp > 5 {
                traversed_wrong_way = true;
            }
        }
        assert!(
            traversed_wrong_way,
            "car must actually traverse the circuit in the wrong direction"
        );

        // Infield cut attempt: start fresh, drive forward 30 ticks, steer sharply left across infield to (0,0)
        let mut sim_cut = Sim::new(track, 1);
        for _ in 0..30 {
            sim_cut.tick(&[hold(1.0, 0.0, 0.0)]);
        }
        // Steer left cutting across infield
        for _ in 0..80 {
            sim_cut.tick(&[hold(1.0, 0.0, 1.0)]);
        }
        // Drive straight back to start/finish line
        for _ in 0..200 {
            let snap = sim_cut.tick(&[hold(1.0, 0.0, 0.0)])[0];
            assert_eq!(
                snap.completed_laps, 0,
                "infield cut attempt must never increment lap counter"
            );
        }
    }

    #[test]
    fn race_phases_transition_countdown_racing_finished_across_three_laps() {
        let text = SAMPLE_CIRCUIT;
        let track = Track::parse(text).unwrap();
        let mut sim = Sim::new(track.clone(), 1);
        sim.start_countdown(16);

        // 1. Countdown phase: controls locked, speed stays 0
        for tick in 0..15 {
            let snap = sim.tick(&[hold(1.0, 0.0, 0.0)])[0];
            assert_eq!(
                snap.phase,
                RacePhase::Countdown {
                    ticks_remaining: 15 - tick
                }
            );
            assert_eq!(
                snap.forward_speed, 0.0,
                "controls must be locked during countdown"
            );
        }

        // 2. Transition to Racing at tick 16
        let snap = sim.tick(&[hold(1.0, 0.0, 0.0)])[0];
        assert_eq!(
            snap.phase,
            RacePhase::Racing,
            "must transition to Racing phase"
        );

        // 3. Drive 3 complete laps and observe Finished phase
        let waypoints = &track.points[..track.points.len() - 1];
        let mut current_wp = 1;
        let mut last_pose = snap.pose;
        let mut last_heading = snap.heading;
        let mut final_snap = None;

        for _ in 0..5000 {
            let snap = drive_step_towards_waypoints(
                &mut sim,
                waypoints,
                &mut current_wp,
                &mut last_pose,
                &mut last_heading,
            );
            final_snap = Some(snap);
            if snap.phase == RacePhase::Finished {
                break;
            }
        }

        let last = final_snap.expect("simulation must produce snapshots");
        assert_eq!(last.completed_laps, TOTAL_LAPS, "must complete 3 laps");
        assert_eq!(
            last.phase,
            RacePhase::Finished,
            "must transition to Finished phase after 3 laps"
        );
        assert!(last.lap_times[0].is_some());
        assert!(last.lap_times[1].is_some());
        assert!(last.lap_times[2].is_some());
    }

    #[test]
    fn all_three_ai_cars_complete_laps_of_the_sample_circuit() {
        let text = SAMPLE_CIRCUIT;
        let track = Track::parse(text).unwrap();
        let mut sim = Sim::new(track, 4);
        sim.enable_ai_opponents();
        let mut max_laps = [0u32; 4];
        for _ in 0..2500 {
            let snaps = sim.tick(&[CarInput::default()]);
            for i in 1..=3 {
                max_laps[i] = max_laps[i].max(snaps[i].completed_laps);
            }
        }

        for (i, &laps) in max_laps.iter().enumerate().skip(1) {
            assert!(
                laps >= 1,
                "AI Car {i} must complete at least 1 lap: got {laps}",
            );
        }
    }

    #[test]
    fn ai_speed_profile_shows_corner_slowdown_on_curved_sections() {
        let text = SAMPLE_CIRCUIT;
        let track = Track::parse(text).unwrap();
        let mut sim = Sim::new(track, 4);
        sim.enable_ai_opponents();
        // Race every Car so the measured profile reflects open-track racing,
        // not queuing behind a parked leader.
        sim.set_ai(0, Some(AiDriver::new(0)));
        let mut max_straight_speed = 0.0f32;
        let mut min_corner_speed = 100.0f32;
        for tick in 0..1200 {
            let snaps = sim.tick(&[CarInput::default()]);
            let ai_car = snaps[1];
            let speed = ai_car.forward_speed;
            // Tick 200..320 is flat out down the initial straightaway
            if (200..320).contains(&tick) {
                max_straight_speed = max_straight_speed.max(speed);
            }
            // Tick 380..460 is braking into and through corner 1
            if (380..460).contains(&tick) {
                min_corner_speed = min_corner_speed.min(speed);
            }
        }
        assert!(
            max_straight_speed > 22.0,
            "AI must reach high speed on straight: {max_straight_speed}"
        );
        assert!(
            min_corner_speed < 15.0,
            "AI must slow down for corner: {min_corner_speed}"
        );
        assert!(
            max_straight_speed > min_corner_speed + 8.0,
            "AI must measurably slow for corner versus straight: straight={max_straight_speed}, corner={min_corner_speed}"
        );
    }

    #[test]
    fn stuck_ai_reverses_out_of_a_wall_and_rejoins() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut sim = Sim::new(track, 4);
        sim.enable_ai_opponents();
        // Stage AI Car 1 nose-into the outer wall along the initial
        // straight, facing away from the racing surface.
        sim.cars[1].pose = Vec2::new(60.0, -13.5);
        sim.cars[1].velocity = Vec2::ZERO;
        sim.cars[1].heading = -std::f32::consts::FRAC_PI_2;

        let mut reversed = false;
        let mut rejoined = false;
        for _ in 0..900 {
            let snaps = sim.tick(&[CarInput::default()]);
            if snaps[1].forward_speed < -0.5 {
                reversed = true;
            }
            if reversed && snaps[1].pose.y.abs() < 7.0 {
                rejoined = true;
                break;
            }
        }
        assert!(reversed, "stuck AI must engage reverse to recover");
        assert!(rejoined, "AI must drive back onto the road after reversing");
    }

    #[test]
    fn ai_field_races_three_laps_cleanly() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut sim = Sim::new(track, 4);
        sim.enable_ai_opponents();
        sim.set_ai(0, Some(AiDriver::new(0)));

        // Measured 2026-09-07 over this exact race: 4921 ticks, 0 wall
        // contacts, 0 off-track ticks, 34 drift ticks (0.17%). The sim is
        // deterministic, so these pins are stable; any wall hit is a driving
        // regression, not noise.
        let mut wall_ticks = 0u32;
        let mut grass_ticks = 0u32;
        let mut drift_ticks = 0u32;
        let mut race_ticks = 0u32;
        let mut leader_best = f32::INFINITY;
        let mut finished = false;
        for _ in 0..8000 {
            let snaps = sim.tick(&[CarInput::default()]);
            race_ticks += 1;
            wall_ticks += snaps.iter().filter(|s| s.wall_contact).count() as u32;
            grass_ticks += snaps.iter().filter(|s| s.surface == Surface::Grass).count() as u32;
            drift_ticks += snaps.iter().filter(|s| s.drifting).count() as u32;
            if snaps.iter().all(|s| s.phase == RacePhase::Finished) {
                leader_best = snaps
                    .iter()
                    .filter_map(|s| s.best_lap_time)
                    .fold(f32::INFINITY, f32::min);
                finished = true;
                break;
            }
        }

        assert!(finished, "the AI field must complete the 3-lap race");
        assert_eq!(wall_ticks, 0, "AI must not touch walls on a clean race");
        assert_eq!(grass_ticks, 0, "AI must stay on authored surfaces");
        let drift_fraction = drift_ticks as f32 / (race_ticks * 4) as f32;
        assert!(
            drift_fraction < 0.01,
            "planned corners must stay gripped, drifted {drift_fraction:.3} of ticks"
        );
        assert!(
            leader_best < 28.0,
            "leader pace must beat 28 s/lap, got {leader_best}"
        );
    }

    #[test]
    fn identical_seeds_produce_identical_ai_behavior() {
        let text = SAMPLE_CIRCUIT;
        let track = Track::parse(text).unwrap();
        let mut sim_a = Sim::new(track.clone(), 4);
        sim_a.enable_ai_opponents();
        let mut sim_b = Sim::new(track, 4);
        sim_b.enable_ai_opponents();

        for _ in 0..300 {
            let snaps_a = sim_a.tick(&[CarInput::default()]);
            let snaps_b = sim_b.tick(&[CarInput::default()]);
            assert_eq!(
                snaps_a, snaps_b,
                "AI simulations must be bit-for-bit deterministic"
            );
        }
    }

    #[test]
    fn ai_cars_collide_with_walls_surfaces_and_other_cars() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut sim = Sim::new(track, 4);
        sim.enable_ai_opponents();

        // 1. Car-to-Car collision between player Car 0 and AI Car 1:
        // Stage Car 0 directly behind AI Car 1 moving faster down the straightaway
        sim.cars[0].pose = Vec2::new(40.0, 0.0);
        sim.cars[0].velocity = Vec2::new(18.0, 0.0);
        sim.cars[0].heading = 0.0;

        sim.cars[1].pose = Vec2::new(42.5, 0.0);
        sim.cars[1].velocity = Vec2::new(6.0, 0.0);
        sim.cars[1].heading = 0.0;

        let v0_before = sim.cars[0].velocity.x;
        let v1_before = sim.cars[1].velocity.x;

        let snaps = sim.tick(&[CarInput {
            throttle: 1.0,
            brake: 0.0,
            steer: 0.0,
            handbrake: false,
        }]);
        let v0_after = snaps[0].velocity.x;
        let v1_after = snaps[1].velocity.x;

        // Momentum transferred: Car 0 slows down, struck AI Car 1 speeds up
        assert!(
            v0_after < v0_before,
            "striking car must bleed speed on impact: before {v0_before}, after {v0_after}"
        );
        assert!(
            v1_after > v1_before,
            "struck AI car must gain speed from impact: before {v1_before}, after {v1_after}"
        );

        // 2. AI cars interact with track surfaces (Gravel) and boundary walls during race navigation:
        let mut sim_circuit = Sim::new(Track::parse(SAMPLE_CIRCUIT).unwrap(), 4);
        sim_circuit.enable_ai_opponents();
        let mut observed_gravel = false;

        for _ in 0..1000 {
            let snaps = sim_circuit.tick(&[CarInput::default()]);
            // Check AI Car 1 traversing gravel
            if snaps[1].surface == Surface::Gravel {
                observed_gravel = true;
            }
        }
        assert!(
            observed_gravel,
            "AI cars must drive across authored track surfaces like Gravel"
        );

        // 3. AI car wall collision: stage AI Car 2 colliding into the boundary wall
        // Segment 0 runs along y = 0.0 with wall_distance = 14.0 (outer wall at y = -14.0).
        sim_circuit.cars[2].pose = Vec2::new(60.0, -13.95);
        sim_circuit.cars[2].velocity = Vec2::new(0.0, -10.0);
        sim_circuit.cars[2].heading = -std::f32::consts::FRAC_PI_2;

        let v_before_wall = sim_circuit.cars[2].velocity.length();
        let snap_wall = sim_circuit.tick(&[CarInput::default()])[2];

        assert!(
            snap_wall.wall_contact,
            "AI car must trigger wall_contact upon boundary collision"
        );
        assert!(
            snap_wall.velocity.length() < v_before_wall,
            "AI car must bleed speed upon wall impact"
        );
        assert!(
            snap_wall.velocity.y > 0.0,
            "AI car normal velocity must reflect off wall"
        );
    }

    #[test]
    fn bumper_to_bumper_cars_separate_to_full_visual_length() {
        // Regression: the old bounding circle (diameter 3.2) never even fired
        // at this gap, yet the visual cars (~4.3 nose to tail) already
        // overlapped. The oriented box must hold the full
        // 2 * CAR_HALF_LENGTH nose-to-tail gap.
        let mut sim = Sim::new(straight_track(), 2);
        for (k, x) in [100.0, 103.5].iter().enumerate() {
            sim.cars[k].pose = Vec2::new(*x, 0.0);
            sim.cars[k].heading = 0.0;
            sim.cars[k].velocity = Vec2::ZERO;
        }
        sim.tick(&[CarInput::default(), CarInput::default()]);
        let dist = (sim.cars[1].pose - sim.cars[0].pose).length();
        assert!(
            dist >= 2.0 * CAR_HALF_LENGTH - 1e-3,
            "nose-to-tail cars must rest {} apart, got {dist}",
            2.0 * CAR_HALF_LENGTH,
        );
    }

    #[test]
    fn side_by_side_cars_separate_to_full_visual_width() {
        // Same overlap class across the lateral axis: door-to-door cars must
        // hold 2 * CAR_HALF_WIDTH (outer tire edge to outer tire edge).
        let mut sim = Sim::new(straight_track(), 2);
        sim.cars[0].pose = Vec2::new(100.0, 0.0);
        sim.cars[0].heading = 0.0;
        sim.cars[0].velocity = Vec2::ZERO;
        sim.cars[1].pose = Vec2::new(100.0, 1.5);
        sim.cars[1].heading = 0.0;
        sim.cars[1].velocity = Vec2::ZERO;
        sim.tick(&[CarInput::default(), CarInput::default()]);
        let lateral = (sim.cars[1].pose.y - sim.cars[0].pose.y).abs();
        assert!(
            lateral >= 2.0 * CAR_HALF_WIDTH - 1e-3,
            "side-by-side cars must rest {} apart, got {lateral}",
            2.0 * CAR_HALF_WIDTH,
        );
    }

    #[test]
    fn clear_side_by_side_cars_are_not_pushed_apart() {
        // Mirror regression: at 2.8 laterally the boxes (2.6 wide together)
        // are clear, so the resolver must leave them alone. The old circle
        // (diameter 3.2) wrongly shoved them apart here.
        let mut sim = Sim::new(straight_track(), 2);
        sim.cars[0].pose = Vec2::new(100.0, 0.0);
        sim.cars[0].heading = 0.0;
        sim.cars[0].velocity = Vec2::ZERO;
        sim.cars[1].pose = Vec2::new(100.0, 2.8);
        sim.cars[1].heading = 0.0;
        sim.cars[1].velocity = Vec2::ZERO;
        sim.tick(&[CarInput::default(), CarInput::default()]);
        let lateral = (sim.cars[1].pose.y - sim.cars[0].pose.y).abs();
        assert!(
            (lateral - 2.8).abs() < 1e-3,
            "clear cars must keep their gap, got {lateral}",
        );
    }

    #[test]
    fn perpendicular_cars_separate_without_overlap() {
        // A T-bone overlap must resolve on the minimum-penetration axis so no
        // corner of either box stays inside the other.
        let mut sim = Sim::new(straight_track(), 2);
        sim.cars[0].pose = Vec2::new(100.0, 0.0);
        sim.cars[0].heading = 0.0;
        sim.cars[0].velocity = Vec2::ZERO;
        sim.cars[1].pose = Vec2::new(101.0, 0.5);
        sim.cars[1].heading = std::f32::consts::FRAC_PI_2;
        sim.cars[1].velocity = Vec2::ZERO;
        sim.tick(&[CarInput::default(), CarInput::default()]);
        assert!(
            obb_penetration(
                sim.cars[0].pose,
                sim.cars[0].heading,
                sim.cars[1].pose,
                sim.cars[1].heading,
            )
            .is_none(),
            "perpendicular cars must be separated after resolution, got {:?} / {:?}",
            sim.cars[0].pose,
            sim.cars[1].pose,
        );
    }

    #[test]
    fn snapshot_carries_applied_steer_for_wheel_visuals() {
        // The shell renders front-wheel yaw from the snapshot, so the tick
        // must publish the applied steer and the no-tick projection must keep it.
        let mut sim = Sim::new(straight_track(), 1);
        sim.cars[0].velocity = Vec2::new(10.0, 0.0);
        let snaps = sim.tick(&[hold(0.5, 0.0, 1.0)]);
        assert_eq!(snaps[0].steer, 1.0);
        let snaps = sim.tick(&[hold(0.5, 0.0, -0.5)]);
        assert_eq!(snaps[0].steer, -0.5);
        assert_eq!(sim.snapshots()[0].steer, -0.5);
    }
}
