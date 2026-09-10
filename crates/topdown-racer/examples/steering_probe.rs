//! Synthetic digital-controller comparison; does not measure human preference.
use glam::Vec2;
use topdown_racer_core::{
    ai::{AiDriver, AiView},
    simulation::{CarInput, GridCar, RacePhase, Sim},
    track::{Track, HILLSIDE_CIRCUIT},
};
fn adapt(applied: &mut f32, target: f32, rise: f32) -> f32 {
    if rise == 0.0 {
        *applied = target;
    } else {
        let rate = if target == 0.0 || target * *applied < 0.0 {
            20.0
        } else {
            1.0 / rise
        };
        *applied += (target - *applied).clamp(-rate / 64.0, rate / 64.0);
    }
    *applied
}
fn main() {
    for rise in [0.0, 0.05, 0.1, 0.15] {
        let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
        let mut pulse = Sim::from_grid(
            track.clone(),
            &[GridCar {
                pose: Vec2::new(65.0, 0.0),
                heading: 0.0,
                velocity: Vec2::new(29.28, 0.0),
            }],
        );
        let mut filter = 0.0;
        let mut heading = 0.0;
        for _ in 0..8 {
            heading = pulse.tick(&[CarInput {
                steer: adapt(&mut filter, 1.0, rise),
                ..Default::default()
            }])[0]
                .heading;
        }
        for _ in 0..4 {
            adapt(&mut filter, -1.0, rise);
        }
        let reversal = filter;
        let mut sim = Sim::new(track.clone(), 1);
        let mut driver = AiDriver::new(0);
        let mut filter = 0.0;
        let mut walls = 0;
        let mut result = None;
        for _ in 0..16000 {
            let car = sim.snapshots()[0];
            let field = [(car.pose, car.velocity)];
            let mut input = driver.compute_input(AiView {
                active: None,
                car_index: 0,
                pose: car.pose,
                heading: car.heading,
                velocity: car.velocity,
                track: &track,
                field: &field,
            });
            let target = if input.steer.abs() < 0.05 {
                0.0
            } else {
                input.steer.signum()
            };
            input.steer = adapt(&mut filter, target, rise);
            let snapshot = sim.tick(&[input])[0];
            walls += usize::from(snapshot.wall_contact);
            if sim.phase() == RacePhase::Finished {
                result = Some(snapshot.lap_times);
                break;
            }
        }
        println!("rise_ms={:.0}, pulse_degrees={:.3}, reversal_after_4ticks={:.3}, wall_ticks={}, laps={:?}",rise*1000.0,heading.to_degrees(),reversal,walls,result);
    }
}
