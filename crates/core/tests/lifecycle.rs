use topdown_racer_core::{
    simulation::{CarInput, Sim},
    track::{Track, HILLSIDE_CIRCUIT},
};

#[test]
fn pause_and_deliberate_resume_preserve_the_fixed_tick_sequence() {
    let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
    let mut baseline = Sim::new_race(track.clone(), 4);
    let mut paused = Sim::new_race(track, 4);
    let input = CarInput {
        throttle: 1.0,
        ..Default::default()
    };
    for _ in 0..240 {
        baseline.tick(&[input]);
        paused.tick(&[input]);
    }
    paused.pause();
    let frozen = paused.snapshots();
    for _ in 0..200 {
        assert_eq!(paused.tick(&[input]), frozen);
    }
    paused.resume();
    for _ in 0..64 {
        paused.tick(&[input]);
    }
    for _ in 0..120 {
        assert_eq!(baseline.tick(&[input]), paused.tick(&[input]));
    }
}

#[test]
fn recovery_is_slow_safe_rate_limited_and_invalidates_the_lap() {
    use glam::Vec2;
    use topdown_racer_core::simulation::{GridCar, RecoveryError};
    let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
    let mut sim = Sim::from_grid(
        track.clone(),
        &[GridCar {
            pose: Vec2::new(50.0, 40.0),
            heading: 2.0,
            velocity: Vec2::ZERO,
        }],
    );
    sim.recover_player().unwrap();
    let snapshot = sim.snapshots()[0];
    assert!(snapshot.current_lap_invalidated);
    assert_eq!(snapshot.completed_laps, 0);
    assert_eq!(snapshot.velocity, Vec2::ZERO);
    assert_eq!(snapshot.surface, topdown_racer_core::track::Surface::Road);
    assert_eq!(sim.recover_player(), Err(RecoveryError::Cooldown));
    let blocker = track.spawn_pose(5.0);
    let mut occupied = Sim::from_grid(
        track,
        &[
            GridCar {
                pose: Vec2::new(50.0, 40.0),
                heading: 0.0,
                velocity: Vec2::ZERO,
            },
            GridCar {
                pose: blocker,
                heading: 0.0,
                velocity: Vec2::ZERO,
            },
        ],
    );
    assert_eq!(occupied.recover_player(), Err(RecoveryError::Occupied));
}

#[test]
fn recovery_cannot_gain_a_position_by_shortening_distance_to_next_checkpoint() {
    use glam::Vec2;
    use topdown_racer_core::simulation::GridCar;
    let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
    let mut sim = Sim::from_grid(
        track,
        &[
            GridCar {
                pose: Vec2::new(50.0, 40.0),
                heading: 0.0,
                velocity: Vec2::ZERO,
            },
            GridCar {
                pose: Vec2::new(44.0, 0.0),
                heading: 0.0,
                velocity: Vec2::ZERO,
            },
        ],
    );
    let before = sim.snapshots()[0].position;
    assert_eq!(before, 2);
    sim.recover_player().unwrap();
    assert_eq!(sim.snapshots()[0].position, before);
}

#[test]
fn practice_starts_repeatably_and_rejects_assistance() {
    use topdown_racer_core::{
        practice::{CornerChallenge, PracticeAttempt, PracticeStatus},
        simulation::DrivingMode,
    };
    let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
    let challenge = CornerChallenge::hillside(&track);
    let mut a = challenge.start(track.clone());
    let b = challenge.start(track);
    assert_eq!(a.snapshots(), b.snapshots());
    assert_eq!(a.snapshots().len(), 1);
    let mut attempt = PracticeAttempt::new(challenge);
    for _ in 0..64 {
        a.tick(&[]);
    }
    let previous = a.snapshots()[0];
    a.request_player_mode(DrivingMode::Autopilot);
    let current = a.tick(&[])[0];
    attempt.observe(&previous, &current);
    assert_eq!(attempt.status(), PracticeStatus::Invalid("Autopilot used"));
}

#[test]
fn practice_scripted_manual_commands_finish_and_retry_identically() {
    use topdown_racer_core::{
        ai::{AiDriver, AiView},
        practice::{CornerChallenge, PracticeAttempt, PracticeStatus},
    };
    let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
    let challenge = CornerChallenge::hillside(&track);
    let mut sim = challenge.start(track.clone());
    let mut retry = challenge.start(track.clone());
    let mut attempt = PracticeAttempt::new(challenge.clone());
    let mut repeated = PracticeAttempt::new(challenge);
    let mut driver = AiDriver::new(0);
    for _ in 0..64 {
        sim.tick(&[]);
        retry.tick(&[]);
    }
    for _ in 0..4000 {
        let previous = sim.snapshots()[0];
        let field = [(previous.pose, previous.velocity)];
        let input = driver.compute_input(AiView {
            active: None,
            car_index: 0,
            pose: previous.pose,
            heading: previous.heading,
            velocity: previous.velocity,
            track: &track,
            field: &field,
        });
        let current = sim.tick(&[input])[0];
        assert_eq!(current, retry.tick(&[input])[0]);
        attempt.observe(&previous, &current);
        repeated.observe(&previous, &current);
        assert_eq!(attempt.status(), repeated.status());
        match attempt.status() {
            PracticeStatus::Finished {
                seconds,
                exit_speed,
            } => {
                assert!(seconds > 5.0 && seconds < 30.0);
                assert!(exit_speed > 10.0);
                return;
            }
            PracticeStatus::Invalid(reason) => panic!("clean driver invalidated: {reason}"),
            _ => {}
        }
    }
    panic!("section never finished");
}
