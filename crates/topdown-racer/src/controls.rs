//! Fixed-tick human steering adaptation. AI controls bypass this boundary.
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SteeringResponse {
    #[default]
    Raw,
    Smooth,
}

impl SteeringResponse {
    pub fn label(self) -> &'static str {
        match self {
            Self::Raw => "RAW",
            Self::Smooth => "SMOOTH (100ms rise)",
        }
    }
}

#[derive(Default, Resource)]
pub struct SteeringFilter {
    applied: f32,
}

impl SteeringFilter {
    pub fn reset(&mut self) {
        self.applied = 0.0;
    }
    pub fn step(&mut self, target: f32, response: SteeringResponse) -> f32 {
        let target = target.clamp(-1.0, 1.0);
        if response == SteeringResponse::Raw {
            self.applied = target;
        } else {
            let rate = if target == 0.0 || target * self.applied < 0.0 {
                20.0
            } else {
                10.0
            };
            self.applied += (target - self.applied).clamp(-rate / 64.0, rate / 64.0);
        }
        self.applied
    }
}
