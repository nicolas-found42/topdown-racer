use glam::Vec2;
use topdown_racer_core::{
    practice::{CornerChallenge, PracticeAttempt, PracticeStatus},
    simulation::{CarSnapshot, GridCar, Sim},
    track::{DirectionalGate, Track, HILLSIDE_CIRCUIT},
};
fn fixture() -> (CornerChallenge, CarSnapshot) {
    let car = Sim::new(Track::parse(HILLSIDE_CIRCUIT).unwrap(), 1).snapshots()[0];
    let challenge = CornerChallenge {
        gates: [10.0, 20.0, 30.0]
            .map(|x| DirectionalGate {
                center: Vec2::new(x, 0.0),
                direction: Vec2::X,
                half_width: 5.0,
            })
            .to_vec(),
        initial: GridCar {
            pose: Vec2::ZERO,
            heading: 0.0,
            velocity: Vec2::X * 20.0,
        },
    };
    (challenge, car)
}
#[test]
fn practice_gate_order_reverse_and_recovery_are_ineligible() {
    let (challenge, car) = fixture();
    for (from, to, invalidated, expected) in [
        (19.0, 21.0, false, "Skipped a section gate"),
        (11.0, 9.0, false, "Reverse gate crossing"),
        (0.0, 1.0, true, "Car recovered"),
    ] {
        let mut attempt = PracticeAttempt::new(challenge.clone());
        let mut previous = car;
        previous.pose = Vec2::new(from, 0.0);
        let mut current = car;
        current.pose = Vec2::new(to, 0.0);
        current.current_lap_invalidated = invalidated;
        attempt.observe(&previous, &current);
        assert_eq!(attempt.status(), PracticeStatus::Invalid(expected));
    }
}
#[test]
fn practice_times_only_the_section_and_freezes_at_exit() {
    let (challenge, car) = fixture();
    let mut attempt = PracticeAttempt::new(challenge);
    let mut previous = car;
    previous.pose = Vec2::new(9.0, 0.0);
    for x in 10..=31 {
        let mut current = car;
        current.pose = Vec2::new(x as f32, 0.0);
        current.velocity = Vec2::new(24.0, 0.0);
        attempt.observe(&previous, &current);
        previous = current;
    }
    assert_eq!(
        attempt.status(),
        PracticeStatus::Finished {
            seconds: 20.0 / 64.0,
            exit_speed: 24.0
        }
    );
    let mut invalid = previous;
    invalid.current_lap_invalidated = true;
    attempt.observe(&previous, &invalid);
    assert_eq!(
        attempt.status(),
        PracticeStatus::Finished {
            seconds: 20.0 / 64.0,
            exit_speed: 24.0
        }
    );
}
