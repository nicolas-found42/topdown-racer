//! The simulation seam: Track + per-Car inputs in, snapshots out, at a fixed
//! 120 Hz step. Plain Rust, no engine types — the Bevy shell renders snapshots
//! and maps keys to [`CarInput`]; all game rules live here.

use glam::Vec2;

use crate::ai::{AiDriver, AiView, OpponentPace};
use crate::track::{Surface, Track};
use crate::vehicle::Vehicle;
pub use crate::vehicle::{WheelSnapshot, ASSIST_MOMENT_LIMIT, WHEEL_POSITIONS};

/// Fixed simulation rate in steps per second.
pub const FIXED_HZ: f32 = 120.0;

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
    /// Handbrake demand: brakes the rear wheels to provoke Drift.
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

/// Explicit owner of the player's controls for a fixed tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrivingMode {
    #[default]
    Manual,
    Autopilot,
}

impl DrivingMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Manual => "MANUAL",
            Self::Autopilot => "DEMO / AUTOPILOT",
        }
    }
    pub fn toggled(self) -> Self {
        match self {
            Self::Manual => Self::Autopilot,
            Self::Autopilot => Self::Manual,
        }
    }
}

/// Standard race length in completed laps.
pub const TOTAL_LAPS: u32 = 3;

/// Default countdown duration in ticks (3.0 seconds at 120 Hz).
pub const DEFAULT_COUNTDOWN_TICKS: u32 = 3 * FIXED_HZ as u32;

pub const FINISH_WINDOW_TICKS: u32 = 45 * FIXED_HZ as u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FinishStatus {
    #[default]
    Racing,
    Finished {
        place: usize,
    },
    Dnf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryError {
    TooFast,
    Occupied,
    Cooldown,
    Unavailable,
}

impl RecoveryError {
    pub fn message(self) -> &'static str {
        match self {
            Self::TooFast => "Slow below 1 u/s before recovery",
            Self::Occupied => "No clear recovery position available",
            Self::Cooldown => "Recovery available after 3 seconds of Race time",
            Self::Unavailable => "Recovery unavailable outside active racing",
        }
    }
}

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
    pub wheels: [WheelSnapshot; 4],
    pub yaw_rate: f32,
    pub steering_angle: f32,
    pub engine_rpm: f32,
    pub gear: i8,
    pub stability_assist: bool,
    pub assist_moment: f32,
    pub current_lap_invalidated: bool,
    pub lap_invalidated: [bool; 3],
    pub finish_status: FinishStatus,
    pub finish_window_ticks: Option<u32>,
    pub driving_mode: DrivingMode,
    /// Sticky until the current lap ends, including after returning to Manual.
    pub current_lap_assisted: bool,
    pub lap_assisted: [bool; TOTAL_LAPS as usize],
    /// Best wholly Manual flying lap eligible for the legacy record.
    pub best_manual_lap_time: Option<f32>,
    /// World position.
    pub pose: Vec2,
    /// Heading in radians; 0 faces +x, positive is counter-clockwise.
    pub heading: f32,
    /// World velocity.
    pub velocity: Vec2,
    /// Signed speed along the Car's heading; negative while reversing.
    pub forward_speed: f32,
    /// Steering demand applied on this tick, -1..1. Positive is left
    /// (counter-clockwise); actual wheel angle is exposed separately.
    pub steer: f32,
    /// Forward acceleration demand applied on this tick, 0..1.
    pub throttle: f32,
    /// Braking demand applied on this tick, 0..1.
    pub brake: f32,
    /// Handbrake demand applied on this tick.
    pub handbrake: bool,
    /// Surface the Car is currently driving on.
    pub surface: Surface,
    /// Whether the Car contacted a boundary wall during this tick.
    pub wall_contact: bool,
    pub car_contact: bool,
    /// Largest pre-impulse closing speed along a Car contact normal this tick.
    pub car_contact_speed: f32,
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

/// Initial Car motion for a deterministic rolling grid or encounter. These
/// are initial conditions only: subsequent movement always goes through physics.
#[derive(Debug, Clone, Copy)]
pub struct GridCar {
    pub pose: Vec2,
    pub heading: f32,
    pub velocity: Vec2,
}

/// The headless world: Cars advancing over a Track at the fixed step.
pub struct Sim {
    paused: bool,
    resume_ticks: u32,
    winner_tick: Option<u32>,
    track: Track,
    cars: Vec<CarState>,
    phase: RacePhase,
    /// Fixed ticks elapsed since the race phase began (green). The Race clock
    /// lives in the simulation so every consumer (HUD, overlays) reads one clock.
    racing_ticks: u32,
    pending_player_mode: Option<DrivingMode>,
    opponent_pace: OpponentPace,
}

struct CarState {
    vehicle: Vehicle,
    stability_assist: bool,
    current_lap_invalidated: bool,
    lap_invalidated: [bool; 3],
    last_recovery_tick: Option<u32>,
    recovery_distance_floor: Option<f32>,
    finish_status: FinishStatus,
    pose: Vec2,
    heading: f32,
    velocity: Vec2,
    steer: f32,
    throttle: f32,
    brake: f32,
    handbrake: bool,
    car_contact: bool,
    car_contact_speed: f32,
    reverse: bool,
    standstill_ticks: u32,
    completed_laps: u32,
    current_lap_ticks: u32,
    next_checkpoint: usize,
    last_cleared_checkpoint: usize,
    lap_times: [Option<f32>; TOTAL_LAPS as usize],
    best_lap_time: Option<f32>,
    ai: Option<AiDriver>,
    current_lap_assisted: bool,
    lap_assisted: [bool; TOTAL_LAPS as usize],
    best_manual_lap_time: Option<f32>,
}
impl Sim {
    pub fn stability_assist(&self) -> bool {
        self.cars.first().is_some_and(|car| car.stability_assist)
    }
    pub fn set_stability_assist(&mut self, enabled: bool) {
        if let Some(car) = self.cars.first_mut() {
            if car.stability_assist != enabled {
                car.stability_assist = enabled;
                car.best_manual_lap_time = None;
                if self.phase == RacePhase::Racing && self.racing_ticks > 0 {
                    car.current_lap_invalidated = true;
                }
            }
        }
    }
    /// Builds a sim with `car_count` Cars staged behind the Track's start
    /// line, starting in the `Racing` phase.
    pub fn new(track: Track, car_count: usize) -> Sim {
        Self::new_with_phase(track, car_count, RacePhase::Racing)
    }

    /// Starts a rolling grid with fresh lap state and no drivers attached.
    /// Callers provide finite poses, headings and velocities. Every Car must
    /// validate the full ordered route from the Race origin before claiming a lap.
    pub fn from_grid(track: Track, grid: &[GridCar]) -> Sim {
        let mut sim = Self::new(track, grid.len());
        for (car, initial) in sim.cars.iter_mut().zip(grid) {
            assert!(
                initial.pose.is_finite()
                    && initial.heading.is_finite()
                    && initial.velocity.is_finite()
            );
            car.pose = initial.pose;
            car.heading = initial.heading;
            car.velocity = initial.velocity;
        }
        sim
    }

    /// Builds a full race sim starting in the `Countdown` phase with AI opponents.
    pub fn new_race(track: Track, car_count: usize) -> Sim {
        Self::new_race_with_pace(track, car_count, OpponentPace::default())
    }

    pub fn new_race_with_pace(track: Track, car_count: usize, pace: OpponentPace) -> Sim {
        let mut sim = Self::new_with_phase(
            track,
            car_count,
            RacePhase::Countdown {
                ticks_remaining: DEFAULT_COUNTDOWN_TICKS,
            },
        );
        sim.opponent_pace = pace;
        sim.enable_ai_opponents();
        sim
    }

    /// Builds a sim with an explicit initial `RacePhase`.
    pub fn new_with_phase(track: Track, car_count: usize, phase: RacePhase) -> Sim {
        let total_cps = track.points.len() - 1;
        let start = track.start_segment;
        let cars = (0..car_count)
            .map(|i| {
                let pose = track.spawn_pose(i as f32 * CAR_SPACING);
                // Face each car along the leg it stages on so the grid lines up
                // with the track instead of starting broadside to it.
                let seg = track.nearest_segment(pose).segment_index;
                let dir = track.points[seg + 1] - track.points[seg];
                let heading = dir.y.atan2(dir.x);
                CarState {
                    vehicle: Vehicle::default(),
                    stability_assist: i == 0,
                    current_lap_invalidated: false,
                    lap_invalidated: [false; 3],
                    last_recovery_tick: None,
                    recovery_distance_floor: None,
                    finish_status: FinishStatus::Racing,
                    pose,
                    heading,
                    velocity: Vec2::ZERO,
                    steer: 0.0,
                    throttle: 0.0,
                    brake: 0.0,
                    handbrake: false,
                    car_contact: false,
                    car_contact_speed: 0.0,
                    reverse: false,
                    standstill_ticks: 0,
                    completed_laps: 0,
                    current_lap_ticks: 0,
                    next_checkpoint: (start + 1) % total_cps,
                    last_cleared_checkpoint: start,
                    lap_times: [None; TOTAL_LAPS as usize],
                    best_lap_time: None,
                    ai: None,
                    current_lap_assisted: false,
                    lap_assisted: [false; TOTAL_LAPS as usize],
                    best_manual_lap_time: None,
                }
            })
            .collect();
        Sim {
            paused: false,
            resume_ticks: 0,
            winner_tick: None,
            track,
            cars,
            phase,
            racing_ticks: 0,
            pending_player_mode: None,
            opponent_pace: OpponentPace::default(),
        }
    }

    /// Enables AI driver controllers for trailing cars (indices 1..car_count).
    pub fn enable_ai_opponents(&mut self) {
        for (i, car) in self.cars.iter_mut().enumerate().skip(1) {
            car.ai = Some(AiDriver::with_pace(i, self.opponent_pace));
        }
    }

    /// Sets (or clears) the AI driver for one car. Out-of-range indices are ignored.
    pub fn set_ai(&mut self, car_index: usize, ai: Option<AiDriver>) {
        if let Some(car) = self.cars.get_mut(car_index) {
            car.ai = ai;
        }
    }

    /// Queues ownership for the next fixed step, which discards stale player input.
    pub fn request_player_mode(&mut self, mode: DrivingMode) {
        self.pending_player_mode = Some(mode);
    }

    /// Visible selection, including a change awaiting the next fixed tick.
    pub fn player_mode(&self) -> DrivingMode {
        self.pending_player_mode.unwrap_or(if self.ai_enabled(0) {
            DrivingMode::Autopilot
        } else {
            DrivingMode::Manual
        })
    }

    pub fn pass_stats(&self, car_index: usize) -> Option<crate::ai::PassStats> {
        self.cars
            .get(car_index)?
            .ai
            .as_ref()
            .map(AiDriver::pass_stats)
    }

    pub fn opponent_pace(&self) -> OpponentPace {
        self.opponent_pace
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

    /// Advances the world one fixed step. Manual Cars consume `inputs`, defaulting
    /// to neutral. Player Autopilot ignores external input completely. Explicit
    /// non-neutral inputs can override AI Opponents for headless scenarios.
    pub fn tick(&mut self, inputs: &[CarInput]) -> Vec<CarSnapshot> {
        if self.paused {
            return self.snapshots();
        }
        if self.resume_ticks > 0 {
            self.resume_ticks -= 1;
            return self.snapshots();
        }
        if self.phase == RacePhase::Finished {
            return self.snapshots();
        }
        let switching = self.pending_player_mode.take();
        if let Some(mode) = switching {
            self.set_ai(
                0,
                match mode {
                    DrivingMode::Manual => None,
                    DrivingMode::Autopilot => Some(AiDriver::with_pace(0, self.opponent_pace)),
                },
            );
        }
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

        // Step all cars. AI drivers perceive the field (every Car's pose and
        // velocity) plus the Track, and answer with player-identical inputs.
        let field: Vec<(Vec2, Vec2)> = self.cars.iter().map(|c| (c.pose, c.velocity)).collect();
        let active: Vec<bool> = self
            .cars
            .iter()
            .map(|c| c.finish_status == FinishStatus::Racing)
            .collect();
        let mut step_results = Vec::with_capacity(self.cars.len());
        let previous_laps: Vec<_> = self.cars.iter().map(|c| c.completed_laps).collect();
        for (i, car) in self.cars.iter_mut().enumerate() {
            if car.finish_status != FinishStatus::Racing {
                car.velocity *= (-3.96 * FIXED_DT).exp();
                for wheel in &mut car.vehicle.wheels {
                    wheel.normalized_slip = 0.0;
                    wheel.beyond_limit = false;
                }
                car.pose += car.velocity * FIXED_DT;
                car.throttle = 0.0;
                car.brake = 0.0;
                car.handbrake = false;
                step_results.push((self.track.sample_surface(car.pose), false, false));
                continue;
            }
            let effective_input = if controls_locked {
                CarInput::default()
            } else if let Some(ai) = &mut car.ai {
                if i != 0 && inputs.len() > i && inputs[i] != CarInput::default() {
                    inputs[i]
                } else {
                    ai.compute_input(AiView {
                        active: Some(&active),
                        car_index: i,
                        pose: car.pose,
                        heading: car.heading,
                        velocity: car.velocity,
                        track: &self.track,
                        field: &field,
                    })
                }
            } else if i == 0 && switching.is_some() {
                CarInput::default()
            } else {
                inputs.get(i).copied().unwrap_or_default()
            };

            car.steer = effective_input.steer;
            car.throttle = effective_input.throttle;
            car.brake = effective_input.brake;
            car.handbrake = effective_input.handbrake;
            let previous_pose = car.pose;
            let (surface, wall_contact, drifting) = step_car(car, effective_input, &self.track);
            step_results.push((surface, wall_contact, drifting));

            // In Racing phase, accumulate lap time and check ordered progress
            if self.phase == RacePhase::Racing {
                car.current_lap_assisted |= car.ai.is_some();
                advance_lap_progress(car, &self.track, total_cps, previous_pose);
            }
        }

        // Car-to-Car collisions: pairwise oriented-box collision response
        resolve_car_collisions(&mut self.cars);

        if self.phase == RacePhase::Racing {
            let winner = self
                .cars
                .iter()
                .position(|c| c.completed_laps >= TOTAL_LAPS);
            if self.winner_tick.is_none() {
                if let Some(winner) = winner {
                    self.winner_tick = Some(self.racing_ticks);
                    self.cars[winner].finish_status = FinishStatus::Finished { place: 1 };
                }
            }
            if let Some(winner_tick) = self.winner_tick {
                let mut place = self
                    .cars
                    .iter()
                    .filter(|c| matches!(c.finish_status, FinishStatus::Finished { .. }))
                    .count()
                    + 1;
                // Equal-tick crossings use grid index after the winner.
                for (car, previous) in self.cars.iter_mut().zip(previous_laps) {
                    if car.finish_status == FinishStatus::Racing && car.completed_laps > previous {
                        car.finish_status = FinishStatus::Finished { place };
                        place += 1;
                    }
                }
                if self.racing_ticks - winner_tick >= FINISH_WINDOW_TICKS {
                    for car in &mut self.cars {
                        if car.finish_status == FinishStatus::Racing {
                            car.finish_status = FinishStatus::Dnf;
                        }
                    }
                }
                if self
                    .cars
                    .iter()
                    .all(|c| c.finish_status != FinishStatus::Racing)
                {
                    self.phase = RacePhase::Finished;
                }
            }
        }

        // Calculate live positions: sorted by progress score
        let positions = compute_positions(&self.cars, &self.track);

        // Build final snapshots
        self.cars
            .iter()
            .zip(positions.iter())
            .zip(step_results.iter())
            .map(|((car, &position), &(surface, wall_contact, drifting))| {
                let mut snapshot =
                    Self::make_snapshot(car, surface, wall_contact, drifting, position, self.phase);
                snapshot.finish_window_ticks = self.finish_window_remaining();
                snapshot
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
                let mut snapshot = Self::make_snapshot(
                    car,
                    self.track.sample_surface(car.pose),
                    self.track.wall_contact(car.pose).is_some(),
                    false,
                    position,
                    self.phase,
                );
                snapshot.finish_window_ticks = self.finish_window_remaining();
                snapshot
            })
            .collect()
    }

    fn finish_window_remaining(&self) -> Option<u32> {
        self.winner_tick
            .map(|tick| FINISH_WINDOW_TICKS.saturating_sub(self.racing_ticks - tick))
    }

    pub fn pause(&mut self) {
        if self.phase != RacePhase::Finished {
            self.paused = true;
            self.resume_ticks = 0;
        }
    }
    pub fn resume(&mut self) {
        if self.paused {
            self.paused = false;
            self.resume_ticks = FIXED_HZ as u32;
        }
    }
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    pub fn preparation_ticks(&self) -> u32 {
        self.resume_ticks
    }

    /// Returns behind the last validated checkpoint, with no checkpoint credit.
    /// Available from Pause as well as active racing, below 1 u/s. The three-second
    /// cooldown advances only with racing ticks; pause/preparation cannot bypass it.
    /// Searches backward in one-unit steps along the preceding Track segment,
    /// keeping the original five-unit setback as the first candidate.
    pub fn recover_player(&mut self) -> Result<(), RecoveryError> {
        let car = self.cars.first().ok_or(RecoveryError::Unavailable)?;
        if self.phase != RacePhase::Racing || car.finish_status != FinishStatus::Racing {
            return Err(RecoveryError::Unavailable);
        }
        if car
            .last_recovery_tick
            .is_some_and(|tick| self.racing_ticks - tick < 3 * FIXED_HZ as u32)
        {
            return Err(RecoveryError::Cooldown);
        }
        if car.velocity.length() >= 1.0 {
            return Err(RecoveryError::TooFast);
        }
        let arc: f32 = self
            .track
            .points
            .windows(2)
            .take(car.last_cleared_checkpoint)
            .map(|pair| pair[0].distance(pair[1]))
            .sum();
        let segments = self.track.points.len() - 1;
        let previous = (car.last_cleared_checkpoint + segments - 1) % segments;
        // Do not search around the whole circuit: wrapping into an unvalidated
        // segment could put the Car just before its next checkpoint as a shortcut.
        // On very short segments retain the existing five-unit setback.
        let max_setback = self.track.points[previous]
            .distance(self.track.points[previous + 1])
            .max(5.0);
        let pose = (0..=(max_setback - 5.0).ceil() as usize)
            .map(|step| {
                self.track
                    .point_at_arc(arc - (5.0 + step as f32).min(max_setback))
            })
            .find(|candidate| {
                self.cars
                    .iter()
                    .skip(1)
                    .all(|rival| rival.pose.distance(*candidate) >= 6.0)
            })
            .ok_or(RecoveryError::Occupied)?;
        let direction = self.track.centerline_frame(pose).direction;
        let car = &mut self.cars[0];
        car.recovery_distance_floor = Some(
            car.pose
                .distance(self.track.points[car.next_checkpoint])
                .max(car.recovery_distance_floor.unwrap_or(0.0)),
        );
        if let Some(driver) = &mut car.ai {
            driver.reset_after_recovery();
        }
        car.pose = pose;
        car.heading = direction.y.atan2(direction.x);
        car.velocity = Vec2::ZERO;
        car.vehicle = Vehicle::default();
        car.reverse = false;
        car.standstill_ticks = 0;
        car.steer = 0.0;
        car.throttle = 0.0;
        car.brake = 0.0;
        car.handbrake = false;
        car.current_lap_invalidated = true;
        car.last_recovery_tick = Some(self.racing_ticks);
        Ok(())
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
            wheels: car.vehicle.wheels,
            yaw_rate: car.vehicle.yaw_rate,
            steering_angle: car.vehicle.steering_angle,
            engine_rpm: car.vehicle.rpm,
            gear: if car.reverse {
                -1
            } else {
                car.vehicle.gear as i8 + 1
            },
            stability_assist: car.stability_assist,
            assist_moment: car.vehicle.assist_moment,
            current_lap_invalidated: car.current_lap_invalidated,
            lap_invalidated: car.lap_invalidated,
            finish_status: car.finish_status,
            finish_window_ticks: None,
            driving_mode: if car.ai.is_some() {
                DrivingMode::Autopilot
            } else {
                DrivingMode::Manual
            },
            current_lap_assisted: car.current_lap_assisted,
            lap_assisted: car.lap_assisted,
            best_manual_lap_time: car.best_manual_lap_time,
            pose: car.pose,
            heading: car.heading,
            velocity: car.velocity,
            forward_speed: car.velocity.dot(forward(car.heading)),
            steer: car.steer,
            throttle: car.throttle,
            brake: car.brake,
            handbrake: car.handbrake,
            surface,
            wall_contact,
            car_contact: car.car_contact,
            car_contact_speed: car.car_contact_speed,
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

/// Advances one Car's lap/checkpoint progress for a single tick: accumulates
/// the current lap time and, on ordered checkpoint entry, records lap times,
/// best lap, completed laps, and the next checkpoint to clear.
fn advance_lap_progress(car: &mut CarState, track: &Track, total_cps: usize, previous_pose: Vec2) {
    car.current_lap_ticks += 1;

    let start = track.start_segment;
    let target_cp = car.next_checkpoint;
    // Checkpoint clearing radius follows the checkpoint vertex's local
    // width (uniform-width tracks: wall_distance * 1.5, as before).
    let cp_radius = track.widths[target_cp] * CHECKPOINT_CLEAR_WIDTH_FACTOR;
    let cp_pos = track.points[target_cp];
    let entry_dir = if target_cp == 0 {
        cp_pos - track.points[total_cps - 1]
    } else {
        cp_pos - track.points[target_cp - 1]
    };
    let cleared = if target_cp == start {
        track.finish_gate().crossed(previous_pose, car.pose)
    } else {
        (car.pose - cp_pos).length() < cp_radius && car.velocity.dot(entry_dir) > 0.0
    };
    if cleared {
        car.recovery_distance_floor = None;
        car.last_cleared_checkpoint = target_cp;
        if target_cp == start {
            // Crossed start/finish having visited all checkpoints in order
            if car.completed_laps < TOTAL_LAPS {
                let lap_time = car.current_lap_ticks as f32 * FIXED_DT;
                car.lap_times[car.completed_laps as usize] = Some(lap_time);
                car.best_lap_time = Some(match car.best_lap_time {
                    Some(best) => best.min(lap_time),
                    None => lap_time,
                });
                car.lap_assisted[car.completed_laps as usize] = car.current_lap_assisted;
                car.lap_invalidated[car.completed_laps as usize] = car.current_lap_invalidated;
                if !car.current_lap_assisted
                    && !car.current_lap_invalidated
                    && car.completed_laps > 0
                {
                    car.best_manual_lap_time = Some(
                        car.best_manual_lap_time
                            .map_or(lap_time, |best| best.min(lap_time)),
                    );
                }
                car.current_lap_assisted = false;
                car.current_lap_invalidated = false;
                car.completed_laps += 1;
                car.current_lap_ticks = 0;
            }
            car.next_checkpoint = (start + 1) % total_cps;
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
    let total_cps = track.points.len() - 1;
    let start = track.start_segment;
    let mut car_scores: Vec<(usize, f32)> = cars
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let next_cp = track.points[c.next_checkpoint];
            let dist_to_next = (c.pose - next_cp)
                .length()
                .max(c.recovery_distance_floor.unwrap_or(0.0));
            // Checkpoints cleared since the start/finish line, so the order
            // stays monotonic when the line is overridden (identical to the
            // raw index when the line is at segment 0).
            let progress = (c.last_cleared_checkpoint + total_cps - start) % total_cps;
            let score = c.completed_laps as f32 * 10000.0 + progress as f32 * 100.0
                - (dist_to_next / 1000.0);
            (i, score)
        })
        .collect();
    car_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    car_scores.sort_by_key(|(i, _)| match cars[*i].finish_status {
        FinishStatus::Finished { place } => place,
        _ => usize::MAX,
    });
    let mut positions = vec![1; cars.len()];
    for (pos_idx, (car_idx, _)) in car_scores.iter().enumerate() {
        positions[*car_idx] = pos_idx + 1;
    }
    positions
}

fn step_car(car: &mut CarState, input: CarInput, track: &Track) -> (Surface, bool, bool) {
    let vf = car.velocity.dot(forward(car.heading));
    if !car.reverse && vf.abs() < 0.05 && input.throttle <= 0.0 && input.brake > 0.0 {
        car.standstill_ticks += 1;
        if car.standstill_ticks >= (FIXED_HZ / 4.0) as u32 {
            car.reverse = true;
            car.standstill_ticks = 0;
        }
    } else {
        car.standstill_ticks = 0;
    }
    if car.reverse && vf >= -0.05 && input.brake <= 0.0 {
        car.reverse = false;
    }
    car.vehicle.step(
        (&mut car.pose, &mut car.heading, &mut car.velocity),
        input,
        car.reverse,
        car.stability_assist,
        track,
    );
    let mut wall_contact = false;
    // Resolve each corner of the oriented collision box against the Track.
    for x in [-CAR_HALF_LENGTH, CAR_HALF_LENGTH] {
        for y in [-CAR_HALF_WIDTH, CAR_HALF_WIDTH] {
            let fwd = forward(car.heading);
            let offset = fwd * x + fwd.perp() * y;
            if let Some(contact) = track.wall_contact(car.pose + offset) {
                car.pose += contact.normal * contact.penetration;
                car.vehicle
                    .wall_impulse(&mut car.velocity, offset, contact.normal);
                wall_contact = true;
            }
        }
    }
    let body_slip = car.velocity.dot(forward(car.heading).perp());
    let rear_sliding = car.vehicle.wheels[2..]
        .iter()
        .any(|wheel| wheel.normalized_lateral_slip.abs() > 1.0);
    let drifting = !wall_contact
        && car.velocity.dot(forward(car.heading)) > 3.0
        && rear_sliding
        && input.steer * body_slip > 0.0;
    (track.sample_surface(car.pose), wall_contact, drifting)
}

/// Grid slot spacing in world units. Must clear the car's visual length (~4.4
/// nose to tail) so staged cars never render bumper-to-bumper at the start.
const CAR_SPACING: f32 = 5.0;
/// Unit vector along a heading (0 faces +x, positive is counter-clockwise).
fn forward(heading: f32) -> Vec2 {
    Vec2::new(heading.cos(), heading.sin())
}

/// Straight-line envelope of the four-speed drivetrain.
pub const TOP_SPEED: f32 = 29.3;
/// Conservative brake deceleration available to the AI speed plan.
pub const BRAKE_ACCEL: f32 = 8.0;
/// Factor over the checkpoint vertex's local road width that yields the
/// checkpoint clearing radius (uniform-width tracks: wall_distance * 1.5,
/// as before).
const CHECKPOINT_CLEAR_WIDTH_FACTOR: f32 = 1.5;

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
    for car in cars.iter_mut() {
        car.car_contact = false;
        car.car_contact_speed = 0.0;
    }
    let num_cars = cars.len();
    for i in 0..num_cars {
        for j in (i + 1)..num_cars {
            if cars[i].finish_status != FinishStatus::Racing
                || cars[j].finish_status != FinishStatus::Racing
            {
                continue;
            }
            if let Some((normal, penetration)) =
                obb_penetration(cars[i].pose, cars[i].heading, cars[j].pose, cars[j].heading)
            {
                cars[i].pose -= normal * (penetration * 0.5);
                cars[j].pose += normal * (penetration * 0.5);

                let rel_vel = cars[j].velocity - cars[i].velocity;
                let vn = rel_vel.dot(normal);
                cars[i].car_contact = true;
                cars[j].car_contact = true;
                cars[i].car_contact_speed = cars[i].car_contact_speed.max((-vn).max(0.0));
                cars[j].car_contact_speed = cars[j].car_contact_speed.max((-vn).max(0.0));
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
            widths: vec![20.0; 5],
            points: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(2000.0, 0.0),
                Vec2::new(2000.0, 200.0),
                Vec2::new(0.0, 200.0),
                Vec2::new(0.0, 0.0),
            ],
            surfaces: vec![Surface::Road; 4],
            props: Vec::new(),
            zones: Vec::new(),
            theme: crate::track::Theme::Hillside,
            start_segment: 0,
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
            4800
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
    // Manual commands generated from the real driver's geometric plan. No AI owner
    // is attached, so these runs still exercise eligible Manual lap recording.
    fn drive_manual_with_planner(sim: &mut Sim, driver: &mut AiDriver) -> CarSnapshot {
        let snapshot = sim.snapshots()[0];
        let field = [(snapshot.pose, snapshot.velocity)];
        let input = driver.compute_input(AiView {
            active: None,
            car_index: 0,
            pose: snapshot.pose,
            heading: snapshot.heading,
            velocity: snapshot.velocity,
            track: sim.track(),
            field: &field,
        });
        sim.tick(&[input])[0]
    }

    #[test]
    fn held_throttle_accelerates_the_car_from_rest() {
        let inputs = vec![hold(1.0, 0.0, 0.0); FIXED_HZ as usize];
        let snaps = run(straight_track(), &inputs);

        // After the first tick (1/120 s), the Car has just started rolling.
        assert!(
            snaps[0].forward_speed > 0.0 && snaps[0].forward_speed < 1.0,
            "starts rolling: {}",
            snaps[0].forward_speed
        );
        let speeds: Vec<f32> = snaps.iter().map(|s| s.forward_speed).collect();
        // Gear changes interrupt torque; compare elapsed half-second windows
        // rather than requiring acceleration during every shift tick.
        assert!(speeds.last().unwrap() > &speeds[(FIXED_HZ / 2.0) as usize]);
        // A full second of throttle carries the Car well down the straight.
        let last = *snaps.last().unwrap();
        assert!(
            last.forward_speed > 4.0,
            "top of first second, got {}",
            last.forward_speed
        );
        assert!(
            last.pose.x > 2.0,
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
        inputs.extend(vec![hold(0.0, 1.0, 0.0); 75]);
        let snaps = run(straight_track(), &inputs);

        let peak = snaps[FIXED_HZ as usize - 1];
        assert!(peak.forward_speed > 4.0);

        let last = *snaps.last().unwrap();
        assert_eq!(last.forward_speed, 0.0, "car must come to a complete stop");
        assert_eq!(last.velocity, Vec2::ZERO);
    }

    #[test]
    fn held_brake_at_standstill_engages_reverse_and_throttle_recovers() {
        // Hold brake for one second.
        let mut inputs = vec![hold(0.0, 1.0, 0.0); FIXED_HZ as usize];
        // Then hold throttle for two seconds to brake out of reverse and start moving forward.
        inputs.extend(vec![hold(1.0, 0.0, 0.0); 240]);
        let snaps = run(straight_track(), &inputs);

        // After a quarter-second reverse engages; after one second the Car is reversing.
        let reverse_peak = snaps[FIXED_HZ as usize - 1];
        assert!(
            reverse_peak.forward_speed < -3.0,
            "must be moving in reverse: {}",
            reverse_peak.forward_speed
        );
        assert!(
            reverse_peak.forward_speed >= -10.0,
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
    fn handbrake_at_speed_stops_without_reversing() {
        let straight = straight_track();
        let mut sim = Sim::new(straight, 1);
        for _ in 0..FIXED_HZ as u32 {
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

        let mut driver = AiDriver::new(0);
        let mut completed_lap_snap = None;

        // Drive for up to 1800 ticks
        for _ in 0..24000 {
            let snap = drive_manual_with_planner(&mut sim, &mut driver);
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
            t1 > 30.0 && t1 < 45.0,
            "lap time should be realistic: got {t1}"
        );
        assert_eq!(snap.best_lap_time, Some(t1));
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
        let mut driver = AiDriver::new(0);
        let mut final_snap = None;

        for _ in 0..20000 {
            let snap = drive_manual_with_planner(&mut sim, &mut driver);
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
        // A stationary player occupies the finish approach; allow a missed
        // gate crossing to be recovered on the next full circuit.
        for _ in 0..12000 {
            let snaps = sim.tick(&[CarInput::default()]);
            for i in 1..=3 {
                max_laps[i] = max_laps[i].max(snaps[i].completed_laps);
            }
        }

        for (i, &laps) in max_laps.iter().enumerate().skip(1) {
            assert!(
                laps >= 1,
                "AI Car {i} must complete at least 1 lap: got {laps}; cars={:?}",
                sim.snapshots()
                    .iter()
                    .map(|s| (s.pose, s.forward_speed, s.completed_laps, s.finish_status))
                    .collect::<Vec<_>>(),
            );
        }
    }

    #[test]
    fn ai_speed_profile_shows_corner_slowdown_on_curved_sections() {
        let text = SAMPLE_CIRCUIT;
        let track = Track::parse(text).unwrap();
        let mut sim = Sim::new(track, 1);
        sim.enable_ai_opponents();
        // Race every Car so the measured profile reflects open-track racing,
        // not queuing behind a parked leader.
        sim.set_ai(0, Some(AiDriver::new(0)));
        let mut max_straight_speed = 0.0f32;
        let mut min_corner_speed = 100.0f32;
        for _ in 0..12000 {
            let snaps = sim.tick(&[CarInput::default()]);
            let ai_car = snaps[0];
            let speed = ai_car.forward_speed;
            // Tick 200..320 is flat out down the initial straightaway
            if ai_car.pose.x > 45.0 && ai_car.pose.x < 85.0 && ai_car.pose.y.abs() < 3.0 {
                max_straight_speed = max_straight_speed.max(speed);
            }
            // Tick 380..460 is braking into and through corner 1
            if ai_car.pose.x > 145.0 && ai_car.pose.y > 25.0 && ai_car.pose.y < 45.0 {
                min_corner_speed = min_corner_speed.min(speed);
            }
        }
        assert!(
            max_straight_speed > 16.0,
            "AI must reach high speed on straight: {max_straight_speed}"
        );
        assert!(
            min_corner_speed < 15.0,
            "AI must slow down for corner: {min_corner_speed}"
        );
        assert!(
            max_straight_speed > min_corner_speed + 5.0,
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
        for _ in 0..2400 {
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

        // Preserve the clean four-Car bar without manufacturing overtakes.
        // Encounter diagnostics distinguish clean running from time spent
        // racing nearby Cars; scenario tests exercise actual passing moves.
        let mut wall_ticks = 0u32;
        let mut grass_ticks = 0u32;
        let mut drift_ticks = 0u32;
        let mut race_ticks = 0u32;
        let mut leader_best = f32::INFINITY;
        let mut contacts = 0;
        let mut max_contact_speed = 0.0_f32;
        let mut near_pair_ticks = 0;
        let mut finished = false;
        for _ in 0..24000 {
            let snaps = sim.tick(&[CarInput::default()]);
            race_ticks += 1;
            wall_ticks += snaps.iter().filter(|s| s.wall_contact).count() as u32;
            grass_ticks += snaps.iter().filter(|s| s.surface == Surface::Grass).count() as u32;
            drift_ticks += snaps.iter().filter(|s| s.drifting).count() as u32;
            contacts += snaps.iter().filter(|s| s.car_contact).count();
            for (i, snap) in snaps.iter().enumerate() {
                max_contact_speed = max_contact_speed.max(snap.car_contact_speed);
                near_pair_ticks += snaps[i + 1..]
                    .iter()
                    .filter(|rival| snap.pose.distance(rival.pose) < 28.0)
                    .count();
            }
            if snaps.iter().all(|s| s.phase == RacePhase::Finished) {
                leader_best = snaps
                    .iter()
                    .filter_map(|s| s.best_lap_time)
                    .fold(f32::INFINITY, f32::min);
                finished = true;
                break;
            }
        }
        eprintln!("clean field: ticks={race_ticks}, pass_stats={:?}, contacts={contacts}, severity={max_contact_speed}, near_pair_ticks={near_pair_ticks}, wall_ticks={wall_ticks}, grass_ticks={grass_ticks}, drift_ticks={drift_ticks}, leader_best={leader_best}", (0..4).map(|i| sim.pass_stats(i)).collect::<Vec<_>>());
        assert_eq!(contacts, 0, "a clean field must not contact rival Cars");

        assert!(finished, "the AI field must complete the 3-lap race");
        assert_eq!(wall_ticks, 0, "AI must not touch walls on a clean race");
        assert_eq!(grass_ticks, 0, "AI must stay on authored surfaces");
        let drift_fraction = drift_ticks as f32 / (race_ticks * 4) as f32;
        assert!(
            drift_fraction < 0.01,
            "planned corners must stay gripped, drifted {drift_fraction:.3} of ticks"
        );
        assert!(
            (33.0..=38.0).contains(&leader_best),
            "new-model leader pace must stay in the measured 33–38 s band, got {leader_best}"
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

        for _ in 0..12000 {
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
            snap_wall.velocity.y > -10.0 && snap_wall.yaw_rate.abs() > 0.1,
            "wall impulse must reduce inward motion and rotate an offset contact"
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
    /// A small square loop with the start/finish line overridden to segment 2
    /// (the vertex at (100, 100)). Width 20 gives a checkpoint radius of 30.
    fn overridden_start_track() -> Track {
        let text = r#"{
            "name": "Override",
            "width": 20.0,
            "points": [[0,0],[100,0],[100,100],[0,100],[0,0]],
            "surfaces": [],
            "start_line": {"segment": 2}
        }"#;
        Track::parse(text).unwrap()
    }

    /// Teleports a car onto a checkpoint with lawful entry velocity, then
    /// steps one tick and returns the snapshot.
    fn cross_checkpoint(sim: &mut Sim, at: Vec2, velocity: Vec2) -> CarSnapshot {
        sim.cars[0].pose = at;
        sim.cars[0].velocity = velocity;
        sim.tick(&[CarInput::default()])[0]
    }

    #[test]
    fn overridden_start_line_shifts_grid_spawn() {
        let track = overridden_start_track();
        assert_eq!(track.start_segment, 2);
        let mut sim = Sim::new(track.clone(), 3);
        let snaps = sim.tick(&[]);

        // Lead car stages exactly on the overridden line's vertex.
        assert_eq!(snaps[0].pose, Vec2::new(100.0, 100.0));
        // Trailing cars stage behind the line along the loop, facing the
        // leg they stage on (+y up segment 1).
        assert_eq!(snaps[1].pose, track.spawn_pose(5.0));
        assert_eq!(snaps[1].pose, Vec2::new(100.0, 95.0));
        assert_eq!(snaps[2].pose, track.spawn_pose(10.0));
        for snap in &snaps {
            assert!(
                (snap.heading - std::f32::consts::FRAC_PI_2).abs() < 1e-4,
                "grid must face the overridden leg, got {}",
                snap.heading
            );
        }
    }

    #[test]
    fn overridden_start_line_shifts_lap_origin_and_completion() {
        let track = overridden_start_track();
        let mut sim = Sim::new(track, 1);

        // Crossing the old line (vertex 0) out of order clears nothing.
        let snap = cross_checkpoint(&mut sim, Vec2::new(0.0, 0.0), Vec2::new(0.0, -10.0));
        assert_eq!(snap.completed_laps, 0);
        // Teleporting straight onto the overridden line out of order clears
        // nothing either.
        let snap = cross_checkpoint(&mut sim, Vec2::new(100.0, 100.0), Vec2::new(0.0, 10.0));
        assert_eq!(snap.completed_laps, 0);

        // Walk the ordered checkpoints starting after the overridden line:
        // 3 -> 0 -> 1 -> 2 completes exactly one lap at the overridden line.
        let legs = [
            (Vec2::new(0.0, 100.0), Vec2::new(-10.0, 0.0)),
            (Vec2::new(0.0, 0.0), Vec2::new(0.0, -10.0)),
            (Vec2::new(100.0, 0.0), Vec2::new(10.0, 0.0)),
        ];
        for (at, velocity) in legs {
            let snap = cross_checkpoint(&mut sim, at, velocity);
            assert_eq!(snap.completed_laps, 0);
        }
        let snap = cross_checkpoint(&mut sim, Vec2::new(87.05, 100.0), Vec2::new(-10.0, 0.0));
        assert_eq!(snap.completed_laps, 1);
        assert!(snap.lap_times[0].is_some());
    }

    #[test]
    fn checkpoint_radius_follows_local_vertex_width() {
        // Straight ramp: widths 10 -> 30 -> 30 -> 10. Checkpoint 1 sits on
        // the wide vertex (120, 0): its clearing radius is 30 * 1.5 = 45.
        // The global-minimum floor (10 * 1.5 = 15) would refuse a 30-unit
        // clear, breaking the ordered walk below.
        let track = Track::parse(
            r#"{
                "name": "Ramp",
                "width": 10.0,
                "widths": [10.0, 30.0, 30.0, 10.0, 10.0],
                "points": [[0,0],[120,0],[120,60],[0,60],[0,0]],
                "surfaces": []
            }"#,
        )
        .unwrap();
        let mut sim = Sim::new(track, 1);

        // Thirty units short of checkpoint 1, charging at it: clears under
        // the local radius, misses under the global-minimum radius.
        let snap = cross_checkpoint(&mut sim, Vec2::new(90.0, 0.0), Vec2::new(20.0, 0.0));
        assert_eq!(snap.completed_laps, 0);

        // Walk the remaining ordered checkpoints to the start line.
        let legs = [
            (Vec2::new(120.0, 60.0), Vec2::new(0.0, 20.0)),
            (Vec2::new(0.0, 60.0), Vec2::new(-20.0, 0.0)),
        ];
        for (at, velocity) in legs {
            let snap = cross_checkpoint(&mut sim, at, velocity);
            assert_eq!(snap.completed_laps, 0);
        }
        let snap = cross_checkpoint(&mut sim, Vec2::new(7.95, 0.0), Vec2::new(20.0, 0.0));
        assert_eq!(
            snap.completed_laps, 1,
            "30-unit clear of the wide checkpoint must hold"
        );
        assert!(snap.lap_times[0].is_some());
    }

    /// Teleports car 0 through the Sample Circuit's checkpoints 1..=6 in
    /// order, each with a lawful entry velocity, leaving `next_checkpoint`
    /// back on the start segment with no lap credited yet.
    fn walk_ordered_checkpoints(sim: &mut Sim) {
        let legs = [
            (Vec2::new(120.0, 0.0), Vec2::new(20.0, 0.0)),
            (Vec2::new(160.0, 30.0), Vec2::new(40.0, 30.0)),
            (Vec2::new(160.0, 90.0), Vec2::new(0.0, 20.0)),
            (Vec2::new(110.0, 120.0), Vec2::new(-50.0, 30.0)),
            (Vec2::new(40.0, 120.0), Vec2::new(-20.0, 0.0)),
            (Vec2::new(0.0, 90.0), Vec2::new(-40.0, -30.0)),
        ];
        let laps_before = sim.cars[0].completed_laps;
        for (checkpoint, (at, velocity)) in legs.into_iter().enumerate() {
            let snap = cross_checkpoint(sim, at, velocity);
            assert_eq!(
                snap.completed_laps,
                laps_before,
                "checkpoint {} of the ordered walk must not credit a lap",
                checkpoint + 1
            );
        }
        assert_eq!(
            sim.cars[0].next_checkpoint, sim.track.start_segment,
            "the ordered walk must hand the route back to the start segment"
        );
    }

    #[test]
    fn high_speed_swept_crossing_credits_exactly_one_lap() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut sim = Sim::new(track, 1);
        walk_ordered_checkpoints(&mut sim);

        // One tick at 2000 u/s covers ~24 units, so the car goes straight
        // from ten units behind the checker to well past it between poses:
        // only the swept crossing can credit the lap.
        let snap = cross_checkpoint(&mut sim, Vec2::new(0.0, 0.0), Vec2::new(2000.0, 0.0));
        assert_eq!(snap.completed_laps, 1);
        assert!(snap.lap_times[0].is_some());

        // The next tick starts in front of the checker: no second credit.
        let snap = sim.tick(&[CarInput::default()])[0];
        assert_eq!(snap.completed_laps, 1);
    }

    #[test]
    fn oscillation_across_the_gate_never_grants_extra_laps() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let gate = track.finish_gate();
        let mut sim = Sim::new(track, 1);
        walk_ordered_checkpoints(&mut sim);
        let snap = cross_checkpoint(&mut sim, Vec2::new(0.0, 0.0), Vec2::new(2000.0, 0.0));
        assert_eq!(
            snap.completed_laps, 1,
            "fixture must credit the ordered lap"
        );

        // Ten forward sweeps from behind the plane and ten backward sweeps
        // from ahead of it. Re-crossing cannot credit again: the route has
        // not been walked, and the backward sweeps start in front of the
        // plane, where a swept crossing may never fire.
        for i in 0..20 {
            let (start, velocity) = if i % 2 == 0 {
                (gate.center - gate.direction * 5.0, gate.direction * 2000.0)
            } else {
                (
                    gate.center + gate.direction * 15.0,
                    gate.direction * -2000.0,
                )
            };
            let snap = cross_checkpoint(&mut sim, start, velocity);
            assert_eq!(
                snap.completed_laps, 1,
                "oscillation tick {i} must not credit a lap"
            );
        }
    }

    #[test]
    fn stationary_overlap_on_the_gate_never_credits_a_lap() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let gate = track.finish_gate();
        let mut sim = Sim::new(track, 1);
        walk_ordered_checkpoints(&mut sim);

        // Park the car exactly on the checker with no motion for over 1.5 s:
        // overlap is not a crossing, no matter how long it lasts.
        sim.cars[0].pose = gate.center;
        sim.cars[0].velocity = Vec2::ZERO;
        for tick in 0..100 {
            let snap = sim.tick(&[CarInput::default()])[0];
            assert_eq!(snap.completed_laps, 0, "tick {tick} parked on the checker");
        }
    }

    #[test]
    fn lateral_crossing_outside_the_gate_width_never_credits_a_lap() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let gate = track.finish_gate();
        let lateral = Vec2::new(-gate.direction.y, gate.direction.x);
        let mut sim = Sim::new(track, 1);
        walk_ordered_checkpoints(&mut sim);

        // Sweep across the checker plane `half_width + 6` units to the side:
        // the swept intersection lands outside the painted gate span.
        let outside = gate.half_width + 6.0;
        assert!(outside > gate.half_width);
        let start = gate.center - gate.direction * 5.0 + lateral * outside;
        let snap = cross_checkpoint(&mut sim, start, gate.direction * 2000.0);
        assert_eq!(
            snap.completed_laps, 0,
            "a crossing outside the gate width must not credit a lap"
        );
        assert_eq!(
            sim.cars[0].next_checkpoint, sim.track.start_segment,
            "the wide sweep must leave the start checkpoint pending"
        );

        // Positive control: the same sweep through the gate center, with the
        // start checkpoint still pending, does credit a lap.
        let start = gate.center - gate.direction * 5.0;
        let snap = cross_checkpoint(&mut sim, start, gate.direction * 2000.0);
        assert_eq!(snap.completed_laps, 1);
        assert!(snap.lap_times[0].is_some());
    }

    #[test]
    fn finished_cars_coast_without_extra_laps_or_orders() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut sim = Sim::new(track, 2);
        let rival_spawn = sim.cars[1].pose;

        // Three ordered laps classify car 0 as the winner.
        for lap in 0..TOTAL_LAPS {
            walk_ordered_checkpoints(&mut sim);
            let snap = cross_checkpoint(&mut sim, Vec2::new(0.0, 0.0), Vec2::new(2000.0, 0.0));
            assert_eq!(snap.completed_laps, lap + 1);
        }
        let winner = sim.snapshots()[0];
        assert_eq!(winner.finish_status, FinishStatus::Finished { place: 1 });
        assert_eq!(winner.position, 1);
        assert_eq!(
            sim.cars[1].pose, rival_spawn,
            "the parked rival must not be disturbed during the winner's laps"
        );

        // Coast the finished winner straight through the still-racing rival:
        // the frozen classification and the rival's contact flag must hold.
        sim.cars[0].pose = Vec2::new(0.0, 10.0);
        sim.cars[0].heading = -std::f32::consts::FRAC_PI_2;
        sim.cars[0].velocity = Vec2::new(0.0, -30.0);
        let mut overlapped = false;
        for _ in 0..40 {
            let snaps = sim.tick(&[]);
            overlapped |= obb_penetration(
                sim.cars[0].pose,
                sim.cars[0].heading,
                sim.cars[1].pose,
                sim.cars[1].heading,
            )
            .is_some();
            assert_eq!(snaps[0].completed_laps, TOTAL_LAPS);
            assert_eq!(snaps[0].finish_status, FinishStatus::Finished { place: 1 });
            assert_eq!(snaps[0].position, 1);
            assert!(
                !snaps[0].car_contact,
                "a finished car cannot contact anyone"
            );
            assert!(
                !snaps[1].car_contact,
                "a finished car must never mark the racing car"
            );
            assert_eq!(snaps[1].car_contact_speed, 0.0);
        }
        assert!(
            overlapped,
            "the fixture must actually coast the finished car through the rival"
        );
    }

    #[test]
    fn recovery_finds_clear_space_behind_finished_and_dnf_cars() {
        for status in [FinishStatus::Finished { place: 1 }, FinishStatus::Dnf] {
            for checkpoint in [0, 3, 6] {
                let mut sim = Sim::new(Track::parse(SAMPLE_CIRCUIT).unwrap(), 3);
                sim.cars[0].pose = Vec2::new(50.0, 40.0);
                sim.cars[0].last_cleared_checkpoint = checkpoint;
                sim.cars[0].next_checkpoint = checkpoint + 1;
                let arc: f32 = sim
                    .track
                    .points
                    .windows(2)
                    .take(checkpoint)
                    .map(|pair| pair[0].distance(pair[1]))
                    .sum();
                for (rival, distance) in sim.cars.iter_mut().skip(1).zip([5.0, 12.0]) {
                    rival.pose = sim.track.point_at_arc(arc - distance);
                    rival.finish_status = status;
                }
                sim.pause();
                let before = sim.snapshots()[0];
                sim.recover_player().unwrap();
                let recovered = sim.snapshots()[0];
                assert!(sim.is_paused());
                assert_ne!(recovered.surface, Surface::Grass);
                assert_eq!(recovered.velocity, Vec2::ZERO);
                assert_eq!(recovered.completed_laps, before.completed_laps);
                assert!(recovered.position >= before.position);
                assert_eq!(sim.cars[0].next_checkpoint, checkpoint + 1);
                assert_eq!(sim.cars[0].last_cleared_checkpoint, checkpoint);
                assert!(recovered.current_lap_invalidated);
                let previous = (checkpoint + 6) % 7;
                assert_eq!(
                    sim.track.nearest_segment(recovered.pose).segment_index,
                    previous
                );
                let direction =
                    (sim.track.points[previous + 1] - sim.track.points[previous]).normalize();
                assert!((recovered.heading - direction.y.atan2(direction.x)).abs() < 0.001);
                for rival in sim.cars.iter().skip(1) {
                    assert!(recovered.pose.distance(rival.pose) >= 6.0);
                }
                assert_eq!(sim.recover_player(), Err(RecoveryError::Cooldown));
            }
        }
    }

    #[test]
    fn recovery_reports_occupied_without_mutation_when_the_previous_segment_is_full() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let blockers: Vec<_> = (0..=18)
            .map(|step| GridCar {
                pose: track.spawn_pose(step as f32 * 5.0),
                heading: 0.0,
                velocity: Vec2::ZERO,
            })
            .collect();
        let mut grid = vec![GridCar {
            pose: Vec2::new(50.0, 40.0),
            heading: 2.0,
            velocity: Vec2::ZERO,
        }];
        grid.extend(blockers);
        let mut sim = Sim::from_grid(track, &grid);
        let before = sim.snapshots();
        assert_eq!(sim.recover_player(), Err(RecoveryError::Occupied));
        assert_eq!(sim.snapshots(), before);
        assert_eq!(sim.cars[0].last_recovery_tick, None);
    }

    #[test]
    fn recovery_before_and_after_the_seam_cannot_credit_progress() {
        let mut sim = Sim::new(Track::parse(SAMPLE_CIRCUIT).unwrap(), 1);
        walk_ordered_checkpoints(&mut sim);
        sim.cars[0].velocity = Vec2::ZERO;
        let next = sim.cars[0].next_checkpoint;
        sim.recover_player().unwrap();
        assert_eq!(sim.cars[0].next_checkpoint, next);
        assert_eq!(sim.cars[0].completed_laps, 0);
        assert_eq!(sim.cars[0].last_cleared_checkpoint, 6);
        assert_eq!(
            sim.cars[0].pose,
            sim.track.point_at_arc(
                sim.track.total_length() - sim.track.points[6].distance(sim.track.points[7]) - 5.0
            )
        );
        // Finish the invalidated lap, then recover just after the seam.
        cross_checkpoint(&mut sim, Vec2::ZERO, Vec2::new(2000.0, 0.0));
        sim.cars[0].last_recovery_tick = None;
        sim.cars[0].velocity = Vec2::ZERO;
        sim.recover_player().unwrap();
        assert_eq!(sim.cars[0].completed_laps, 1);
        assert_eq!(sim.cars[0].last_cleared_checkpoint, 0);
        assert_eq!(sim.cars[0].next_checkpoint, 1);
        assert_eq!(sim.cars[0].pose, sim.track.spawn_pose(5.0));
        // Crossing the checker again cannot increment a lap without the route.
        let snap = cross_checkpoint(&mut sim, Vec2::ZERO, Vec2::new(2000.0, 0.0));
        assert_eq!(snap.completed_laps, 1);
        assert!(snap.current_lap_invalidated);
    }

    #[test]
    fn finish_window_and_finished_cars_stay_frozen_through_resume() {
        let mut sim = Sim::new(Track::parse(SAMPLE_CIRCUIT).unwrap(), 2);
        for _ in 0..TOTAL_LAPS {
            walk_ordered_checkpoints(&mut sim);
            cross_checkpoint(&mut sim, Vec2::ZERO, Vec2::new(2000.0, 0.0));
        }
        assert_eq!(sim.phase(), RacePhase::Racing);
        assert_eq!(sim.recover_player(), Err(RecoveryError::Unavailable));
        let winner_tick = sim.winner_tick.unwrap();
        let frozen = sim.snapshots();
        let racing_ticks = sim.racing_ticks();
        sim.pause();
        for _ in 0..12000 {
            assert_eq!(sim.tick(&[]), frozen);
            assert_eq!(sim.racing_ticks(), racing_ticks);
            assert_eq!(sim.phase(), RacePhase::Racing);
        }
        sim.resume();
        for _ in 0..FIXED_HZ as u32 {
            assert_eq!(sim.tick(&[]), frozen);
        }
        assert_eq!(sim.winner_tick, Some(winner_tick));
        for _ in 0..45 * FIXED_HZ as u32 - 1 {
            sim.tick(&[]);
            assert_eq!(sim.phase(), RacePhase::Racing);
        }
        sim.tick(&[]);
        assert_eq!(sim.phase(), RacePhase::Finished);
        assert_eq!(sim.snapshots()[1].finish_status, FinishStatus::Dnf);
    }

    #[test]
    fn standing_start_lap_is_never_a_manual_record() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut sim = Sim::new(track.clone(), 1);
        let mut driver = AiDriver::new(0);

        let mut first_lap = None;
        for _ in 0..20000 {
            let snap = drive_manual_with_planner(&mut sim, &mut driver);
            if snap.completed_laps == 1 && first_lap.is_none() {
                first_lap = Some(snap);
            }
            if snap.completed_laps >= 2 {
                break;
            }
        }

        // Lap 1 starts from the grid, so it is never an eligible manual
        // record even though it contributes to the Race duration.
        let first = first_lap.expect("manual driving must complete the standing-start lap");
        assert!(first.lap_times[0].is_some(), "lap 1 time is recorded");
        assert!(
            first.best_manual_lap_time.is_none(),
            "the standing-start lap is never an eligible manual record"
        );

        // The first flying manual lap completes on lap 2 and is eligible.
        let second = sim.snapshots()[0];
        assert!(
            second.completed_laps >= 2,
            "manual driving must complete a flying lap"
        );
        assert!(second.lap_times[1].is_some(), "lap 2 time is recorded");
        assert!(
            second.best_manual_lap_time.is_some(),
            "the first flying manual lap must earn the manual record"
        );
    }
}
