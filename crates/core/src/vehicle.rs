//! Private planar four-wheel solver. The simulation tick remains the public seam.
use crate::{
    simulation::{CarInput, FIXED_DT},
    track::{Surface, Track},
};
use glam::Vec2;

const MASS: f32 = 1000.0;
const INERTIA: f32 = 1700.0;
const GRAVITY: f32 = 9.81;
const COM_HEIGHT: f32 = 0.42;
const RADIUS: f32 = 0.32;
const WHEEL_INERTIA: f32 = 2.0;
pub(crate) const WHEELBASE: f32 = 2.6;
pub(crate) const MAX_STEER: f32 = 0.55;
const TRACK_WIDTH: f32 = 2.1;
const RATIOS: [f32; 4] = [4.0, 3.0, 2.25, 1.67];
const FINAL_DRIVE: f32 = 4.0;
const SUB_DT: f32 = FIXED_DT / 4.0;
pub const ASSIST_MOMENT_LIMIT: f32 = 3500.0;

/// Front-left, front-right, rear-left, rear-right; matches the Car Sprite axles.
pub const WHEEL_POSITIONS: [Vec2; 4] = [
    Vec2::new(1.3, 1.05),
    Vec2::new(1.3, -1.05),
    Vec2::new(-1.3, 1.05),
    Vec2::new(-1.3, -1.05),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelSnapshot {
    pub local_position: Vec2,
    pub surface: Surface,
    pub normal_load: f32,
    pub slip_angle: f32,
    pub slip_ratio: f32,
    /// Maximum absolute lateral/longitudinal slip relative to this tire's peak.
    pub normalized_slip: f32,
    pub normalized_lateral_slip: f32,
    pub beyond_limit: bool,
}
impl Default for WheelSnapshot {
    fn default() -> Self {
        Self {
            local_position: Vec2::ZERO,
            surface: Surface::Road,
            normal_load: MASS * GRAVITY / 4.0,
            slip_angle: 0.0,
            slip_ratio: 0.0,
            normalized_slip: 0.0,
            normalized_lateral_slip: 0.0,
            beyond_limit: false,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Vehicle {
    pub yaw_rate: f32,
    pub steering_angle: f32,
    pub rpm: f32,
    pub gear: usize,
    pub assist_moment: f32,
    pub wheels: [WheelSnapshot; 4],
    omega: [f32; 4],
    acceleration: Vec2,
    shift_remaining: f32,
    initialised: bool,
}
impl Default for Vehicle {
    fn default() -> Self {
        Self {
            yaw_rate: 0.0,
            steering_angle: 0.0,
            rpm: 900.0,
            gear: 0,
            assist_moment: 0.0,
            wheels: std::array::from_fn(|i| WheelSnapshot {
                local_position: WHEEL_POSITIONS[i],
                ..Default::default()
            }),
            omega: [0.0; 4],
            acceleration: Vec2::ZERO,
            shift_remaining: 0.0,
            initialised: false,
        }
    }
}

fn surface_parameters(surface: Surface) -> (f32, f32, f32, f32) {
    match surface {
        Surface::Road => (1.25, 0.13, 1.0, 1.0),
        Surface::Gravel => (0.83, 0.23, 1.3, 1.6),
        Surface::Grass => (0.50, 0.30, 1.8, 3.0),
    }
}
pub(crate) fn lateral_limit(surface: Surface) -> f32 {
    let mu = surface_parameters(surface).0;
    let planned = loads(Vec2::new(0.0, mu * GRAVITY * 0.55));
    // Front axle peak at its planned lateral load, divided by its supported mass.
    let peak = |load: f32| mu * load * (load / (MASS * GRAVITY / 4.0)).powf(-0.08);
    (peak(planned[0]) + peak(planned[1])) / (MASS * 0.5)
}

/// Peak-normalized Pacejka curve: rises to one at normalized slip one, then falls.
fn tire_curve(normalized_slip: f32) -> f32 {
    (1.6 * (normalized_slip * (std::f32::consts::PI / 3.2).tan()).atan()).sin()
}
fn forces(slip: Vec2, capacity: f32) -> Vec2 {
    let force = Vec2::new(tire_curve(slip.x), tire_curve(slip.y)) * capacity;
    force / (force.length() / capacity).max(1.0)
}
fn loads(acceleration: Vec2) -> [f32; 4] {
    let longitudinal = MASS * acceleration.x.clamp(-15.0, 15.0) * COM_HEIGHT / WHEELBASE / 2.0;
    let lateral = MASS * acceleration.y.clamp(-15.0, 15.0) * COM_HEIGHT / TRACK_WIDTH / 2.0;
    let mut result = [0.0; 4];
    for (i, load) in result.iter_mut().enumerate() {
        *load = (MASS * GRAVITY / 4.0
            + if i < 2 { -longitudinal } else { longitudinal }
            + if i % 2 == 0 { -lateral } else { lateral })
        .max(50.0);
    }
    let scale = MASS * GRAVITY / result.iter().sum::<f32>();
    result.map(|load| load * scale)
}

impl Vehicle {
    pub fn wall_impulse(&mut self, velocity: &mut Vec2, offset: Vec2, normal: Vec2) {
        let at_contact = *velocity + offset.perp() * self.yaw_rate;
        let normal_speed = at_contact.dot(normal);
        if normal_speed < 0.0 {
            let lever = offset.perp_dot(normal);
            let impulse = -(1.0 + 0.4) * normal_speed / (1.0 / MASS + lever * lever / INERTIA);
            *velocity += normal * impulse / MASS;
            self.yaw_rate += lever * impulse / INERTIA;
            let tangent = normal.perp();
            *velocity -= tangent * velocity.dot(tangent) * 0.25;
        }
    }

    pub fn step(
        &mut self,
        motion: (&mut Vec2, &mut f32, &mut Vec2),
        input: CarInput,
        reverse: bool,
        assist: bool,
        track: &Track,
    ) {
        let (pose, heading, velocity) = motion;
        if !self.initialised {
            self.omega = [velocity.dot(Vec2::from_angle(*heading)) / RADIUS; 4];
            if !reverse {
                while self.gear < 3
                    && self.omega[2].abs() * RATIOS[self.gear] * FINAL_DRIVE * 60.0
                        / std::f32::consts::TAU
                        > 4500.0
                {
                    self.gear += 1;
                }
            }
            self.initialised = true;
        }
        let speed = velocity.length();
        let ratio = if reverse { 5.0 } else { RATIOS[self.gear] } * FINAL_DRIVE;
        self.rpm = ((self.omega[2].abs() + self.omega[3].abs()) * 0.5 * ratio * 60.0
            / std::f32::consts::TAU)
            .max(900.0);
        if !reverse && self.shift_remaining <= 0.0 {
            if self.rpm > 5300.0 && self.gear < 3 {
                self.gear += 1;
                self.shift_remaining = 0.12;
            } else if self.rpm < 2500.0 && self.gear > 0 {
                self.gear -= 1;
                self.shift_remaining = 0.12;
            }
        }
        self.assist_moment = 0.0;
        for _ in 0..4 {
            let desired = input.steer.clamp(-1.0, 1.0) * MAX_STEER;
            let returning =
                desired.abs() < self.steering_angle.abs() || desired * self.steering_angle < 0.0;
            let rate = if returning { 4.0 } else { 2.4 };
            self.steering_angle +=
                (desired - self.steering_angle).clamp(-rate * SUB_DT, rate * SUB_DT);
            let forward = Vec2::from_angle(*heading);
            let left = forward.perp();
            let local_velocity = Vec2::new(velocity.dot(forward), velocity.dot(left));
            let normal_loads = loads(self.acceleration);
            let ratio = if reverse { 5.0 } else { RATIOS[self.gear] } * FINAL_DRIVE;
            let demand = if reverse { input.brake } else { input.throttle }.clamp(0.0, 1.0);
            let torque = if self.shift_remaining > 0.0 {
                0.0
            } else {
                180.0 * ((6000.0 - self.rpm) / 1000.0).clamp(0.0, 1.0) * demand * ratio
            } * if reverse { -1.0 } else { 1.0 };
            self.shift_remaining = (self.shift_remaining - SUB_DT).max(0.0);
            // A bounded clutch-type limited-slip coupling opposes rear wheel-speed difference.
            let differential = ((self.omega[3] - self.omega[2]) * 12.0)
                .clamp(-torque.abs() * 0.3, torque.abs() * 0.3);
            let mut force = Vec2::ZERO;
            let mut moment = 0.0;
            for i in 0..4 {
                let position = WHEEL_POSITIONS[i];
                let wheel_world = *pose + forward * position.x + left * position.y;
                let surface = track.sample_surface(wheel_world);
                let (mu, peak_angle, _, _) = surface_parameters(surface);
                let load_ratio = normal_loads[i] / (MASS * GRAVITY / 4.0);
                let peak_angle = peak_angle * load_ratio.powf(0.10);
                let peak_ratio = 0.12 * (peak_angle / 0.13);
                let capacity = mu * normal_loads[i] * load_ratio.powf(-0.08);
                let angle = if i < 2 { self.steering_angle } else { 0.0 };
                let wheel_forward = Vec2::from_angle(angle);
                let wheel_left = wheel_forward.perp();
                let at_wheel = local_velocity + position.perp() * self.yaw_rate;
                let longitudinal = at_wheel.dot(wheel_forward);
                let lateral = at_wheel.dot(wheel_left);
                let slip_angle = (-lateral).atan2(longitudinal.abs().max(0.5));
                let brake_demand =
                    if reverse { input.throttle } else { input.brake }.clamp(0.0, 1.0);
                let brake = brake_demand * if i < 2 { 1400.0 } else { 600.0 }
                    + if i >= 2 && input.handbrake {
                        4000.0
                    } else {
                        0.0
                    };
                let drive = if i == 2 {
                    torque * 0.5 + differential
                } else if i == 3 {
                    torque * 0.5 - differential
                } else {
                    0.0
                };
                let braking_sign = if self.omega[i].abs() > 0.1 {
                    self.omega[i].signum()
                } else {
                    longitudinal.signum()
                };
                // Blend into the no-slip rolling constraint near zero speed. This avoids
                // dividing by zero and the stiff wheel-spin singularity at rest.
                if speed < 3.0 {
                    self.omega[i] = longitudinal / RADIUS;
                }
                let slip_ratio =
                    (self.omega[i] * RADIUS - longitudinal) / longitudinal.abs().max(1.0);
                let mut slip = Vec2::new(slip_ratio / peak_ratio, slip_angle / peak_angle);
                let mut tire_force = forces(slip, capacity);
                if speed < 3.0 {
                    let requested = (drive - brake * braking_sign) / RADIUS;
                    let rolling_force = requested.clamp(-capacity, capacity);
                    tire_force.x = rolling_force;
                    if requested.abs() > capacity {
                        slip.x = requested / capacity;
                    }
                }
                // Once brake torque holds a wheel stopped, kinetic friction opposes
                // contact-patch travel. A locked tire has no rolling cornering stiffness.
                // This applies to service-brake lockup and handbrake lockup alike.
                if brake > drive.abs() && self.omega[i].abs() < 0.1 && speed > 1.0 {
                    tire_force =
                        -Vec2::new(longitudinal, lateral).normalize_or_zero() * capacity * 0.8;
                }
                let combined = (tire_force.length() / capacity).max(1.0);
                tire_force /= combined;
                let wheel_force = wheel_forward * tire_force.x + wheel_left * tire_force.y;
                force += wheel_force;
                moment += position.perp_dot(wheel_force);
                self.omega[i] +=
                    (drive - brake * braking_sign - tire_force.x * RADIUS) / WHEEL_INERTIA * SUB_DT;
                if brake > drive.abs() && self.omega[i] * braking_sign < 0.0 {
                    self.omega[i] = 0.0;
                }
                let normalized_slip = slip.abs().max_element();
                self.wheels[i] = WheelSnapshot {
                    local_position: position,
                    surface,
                    normal_load: normal_loads[i],
                    slip_angle,
                    slip_ratio: slip.x * peak_ratio,
                    normalized_slip,
                    normalized_lateral_slip: slip.y,
                    beyond_limit: normalized_slip > 1.0 && speed > 1.0,
                };
            }
            let (_, _, drag_factor, rolling_factor) =
                surface_parameters(track.sample_surface(*pose));
            force.x -= MASS
                * (0.0003 * drag_factor * local_velocity.x * local_velocity.x.abs()
                    + 0.01 * rolling_factor * local_velocity.x);
            let front_slip = self.wheels[..2]
                .iter()
                .map(|w| w.slip_angle.abs())
                .fold(0.0, f32::max);
            let rear_slip = self.wheels[2..]
                .iter()
                .map(|w| w.slip_angle.abs())
                .fold(0.0, f32::max);
            let rear_past_peak = self.wheels[2..]
                .iter()
                .any(|w| w.normalized_lateral_slip.abs() > 1.0);
            let correction = if assist && rear_past_peak && rear_slip > front_slip {
                -(self.yaw_rate * (rear_slip - front_slip) * 18000.0)
                    .clamp(-ASSIST_MOMENT_LIMIT, ASSIST_MOMENT_LIMIT)
            } else {
                0.0
            };
            self.assist_moment = correction;
            self.acceleration = force / MASS;
            *velocity += (forward * force.x + left * force.y) / MASS * SUB_DT;
            self.yaw_rate += (moment + correction) / INERTIA * SUB_DT;
            if speed < 3.0 {
                let blend = (1.0 - speed / 3.0) * 0.25;
                let rolling_yaw = local_velocity.x * self.steering_angle.tan() / WHEELBASE;
                self.yaw_rate += (rolling_yaw - self.yaw_rate) * blend;
                *velocity -= left * velocity.dot(left) * blend;
            }
            if demand == 0.0
                && (speed < 0.05
                    || (input.brake > 0.0 && velocity.dot(forward) * local_velocity.x < 0.0))
            {
                *velocity = Vec2::ZERO;
                self.omega = [0.0; 4];
                self.yaw_rate = 0.0;
            }
            *heading += self.yaw_rate * SUB_DT;
            *pose += *velocity * SUB_DT;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tire_force_has_a_peak_and_a_combined_grip_ceiling() {
        assert!(tire_curve(0.5) < tire_curve(1.0));
        assert!(tire_curve(3.0) < tire_curve(1.0));
        assert!(forces(Vec2::new(1.0, 0.8), 2000.0).y < forces(Vec2::new(0.0, 0.8), 2000.0).y);
        for x in [0.1, 0.8, 1.0, 4.0] {
            for y in [0.0, 0.8, 3.0] {
                assert!(forces(Vec2::new(x, y), 2000.0).length() <= 2000.001);
            }
        }
    }
    #[test]
    fn transfer_preserves_weight_and_moves_load_to_the_outer_and_front_tires() {
        let braking = loads(Vec2::new(-5.0, 4.0));
        assert!((braking.iter().sum::<f32>() - MASS * GRAVITY).abs() < 0.01);
        assert!(braking[0] + braking[1] > braking[2] + braking[3]);
        assert!(braking[1] > braking[0]);
    }
}
