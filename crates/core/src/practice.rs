//! One authored Hillside section, timed independently of three-lap Races.
use crate::{
    simulation::{CarSnapshot, DrivingMode, GridCar, Sim, FIXED_DT},
    track::{DirectionalGate, Surface, Track},
};
use glam::Vec2;

#[derive(Clone)]
pub struct CornerChallenge {
    pub gates: Vec<DirectionalGate>,
    pub initial: GridCar,
}
impl CornerChallenge {
    pub const ID: &'static str = "hillside-braking-transition-v1";
    pub const HINT: &'static str = "Brake before the first bend. Leave room for the right-left transition; compare exit speed, not just entry speed.";
    pub fn hillside(track: &Track) -> Self {
        let gate = |arc: f32| {
            let center = track.point_at_arc(arc);
            let direction = (track.point_at_arc(arc + 0.1) - center).normalize_or_zero();
            DirectionalGate {
                center,
                direction,
                half_width: track.road_half_width_at_arc(arc),
            }
        };
        let mut gates = vec![gate(50.0)];
        let mut arc = 0.0;
        // Mid-segment gates require the entire braking/right-left sequence.
        for (i, pair) in track.points.windows(2).enumerate().take(10) {
            let length = pair[0].distance(pair[1]);
            if i > 0 {
                gates.push(gate(arc + length * 0.5));
            }
            arc += length;
        }
        Self {
            gates,
            initial: GridCar {
                pose: track.point_at_arc(25.0),
                heading: 0.0,
                velocity: Vec2::new(22.0, 0.0),
            },
        }
    }
    pub fn start(&self, track: Track) -> Sim {
        let mut sim = Sim::from_grid(track, &[self.initial]);
        sim.pause();
        sim.resume();
        sim
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PracticeStatus {
    Approach,
    Running,
    Finished { seconds: f32, exit_speed: f32 },
    Invalid(&'static str),
}
pub struct PracticeAttempt {
    pub challenge: CornerChallenge,
    next_gate: usize,
    ticks: u32,
    status: PracticeStatus,
}
impl PracticeAttempt {
    pub fn new(challenge: CornerChallenge) -> Self {
        Self {
            challenge,
            next_gate: 0,
            ticks: 0,
            status: PracticeStatus::Approach,
        }
    }
    pub fn status(&self) -> PracticeStatus {
        self.status
    }
    pub fn elapsed(&self) -> f32 {
        self.ticks as f32 * FIXED_DT
    }
    pub fn observe(&mut self, previous: &CarSnapshot, current: &CarSnapshot) {
        if matches!(
            self.status,
            PracticeStatus::Finished { .. } | PracticeStatus::Invalid(_)
        ) {
            return;
        }
        let invalid = if current.driving_mode == DrivingMode::Autopilot {
            Some("Autopilot used")
        } else if current.current_lap_invalidated {
            Some("Car recovered")
        } else if current.surface == Surface::Grass {
            Some("Left the Track")
        } else {
            None
        };
        if let Some(reason) = invalid {
            self.status = PracticeStatus::Invalid(reason);
            return;
        }
        if self.status == PracticeStatus::Running {
            self.ticks += 1;
        }
        for (i, gate) in self.challenge.gates.iter().enumerate() {
            if gate.crossed(current.pose, previous.pose) {
                self.status = PracticeStatus::Invalid("Reverse gate crossing");
                return;
            }
            if gate.crossed(previous.pose, current.pose) {
                if i != self.next_gate {
                    self.status = PracticeStatus::Invalid("Skipped a section gate");
                    return;
                }
                self.next_gate += 1;
                if self.next_gate == self.challenge.gates.len() {
                    self.status = PracticeStatus::Finished {
                        seconds: self.elapsed(),
                        exit_speed: current.velocity.length(),
                    };
                    return;
                }
                self.status = PracticeStatus::Running;
            }
        }
    }
}
