use topdown_racer_core::{
    ai::{AiDriver, AiView},
    practice::{CornerChallenge, PracticeAttempt, PracticeStatus},
    simulation::CarInput,
    track::{Track, HILLSIDE_CIRCUIT},
};
fn main() {
    for kind in ["tidy", "late"] {
        let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
        let challenge = CornerChallenge::hillside(&track);
        let mut sim = challenge.start(track.clone());
        let mut attempt = PracticeAttempt::new(challenge);
        let mut driver = AiDriver::new(0);
        for tick in 0..4000 {
            let prev = sim.snapshots()[0];
            let field = [(prev.pose, prev.velocity)];
            let mut input = driver.compute_input(AiView {
                active: None,
                car_index: 0,
                pose: prev.pose,
                heading: prev.heading,
                velocity: prev.velocity,
                track: &track,
                field: &field,
            });
            input.steer = if input.steer.abs() < 0.05 {
                0.0
            } else {
                input.steer.signum()
            };
            input.throttle = if input.throttle > 0.0 { 1.0 } else { 0.0 };
            input.brake = if input.brake > 0.0 { 1.0 } else { 0.0 };
            if kind == "late" && prev.pose.x < 136.0 && prev.pose.y < 8.0 {
                input.throttle = 1.0;
                input.brake = 0.0;
            }
            if sim.preparation_ticks() > 0 {
                input = CarInput::default();
            }
            let cur = sim.tick(&[input])[0];
            attempt.observe(&prev, &cur);
            match attempt.status() {
                PracticeStatus::Finished {
                    seconds,
                    exit_speed,
                } => {
                    println!("{kind} tick={tick} seconds={seconds} speed={exit_speed}");
                    break;
                }
                PracticeStatus::Invalid(reason) => {
                    println!("{kind} tick={tick} invalid={reason} pose={:?}", cur.pose);
                    break;
                }
                _ => {}
            }
        }
    }
}
