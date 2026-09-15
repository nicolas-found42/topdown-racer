use glam::Vec2;
use topdown_racer_core::{
    simulation::{DrivingMode, Sim},
    track::{Track, SAMPLE_CIRCUIT},
};

#[test]
fn completed_lap_requires_crossing_the_painted_checker() {
    let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
    // The shipping checker is centered ten units along the first straight.
    let mut sim = Sim::new(track, 1);
    sim.request_player_mode(DrivingMode::Autopilot);
    let mut previous = sim.snapshots()[0];
    for _ in 0..5000 {
        let current = sim.tick(&[])[0];
        if current.completed_laps > previous.completed_laps {
            assert!(
                previous.pose.x <= 10.0 && current.pose.x > 10.0,
                "lap credited away from checker: {:?} -> {:?}",
                previous.pose,
                current.pose
            );
            assert!(current.velocity.dot(Vec2::X) > 0.0);
            return;
        }
        previous = current;
    }
    panic!("Autopilot must complete a lap");
}

#[test]
fn rolling_grid_near_finish_cannot_claim_an_untravelled_lap() {
    use topdown_racer_core::simulation::GridCar;
    let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
    let mut sim = Sim::from_grid(
        track,
        &[GridCar {
            pose: Vec2::new(0.0, 5.0),
            heading: -std::f32::consts::FRAC_PI_2,
            velocity: Vec2::new(0.0, -10.0),
        }],
    );
    sim.request_player_mode(DrivingMode::Autopilot);
    for _ in 0..160 {
        assert_eq!(
            sim.tick(&[])[0].completed_laps,
            0,
            "starting near the finish grants no route progress"
        );
    }
}

#[test]
fn winner_leaves_the_manual_player_a_finish_window() {
    use topdown_racer_core::simulation::{CarInput, RacePhase};
    let mut sim = Sim::new_race(Track::parse(SAMPLE_CIRCUIT).unwrap(), 4);
    for _ in 0..12000 {
        let snapshots = sim.tick(&[]);
        if snapshots.iter().any(|car| car.completed_laps == 3) {
            assert_eq!(sim.phase(), RacePhase::Racing);
            let player = sim.tick(&[CarInput {
                throttle: 1.0,
                ..Default::default()
            }])[0];
            assert!(player.forward_speed > 0.0);
            let frozen_winner = snapshots
                .iter()
                .position(|car| car.completed_laps == 3)
                .unwrap();
            for _ in 0..topdown_racer_core::simulation::FINISH_WINDOW_TICKS - 1 {
                sim.tick(&[]);
            }
            let final_state = sim.snapshots();
            assert_eq!(sim.phase(), RacePhase::Finished);
            assert_eq!(
                final_state[0].finish_status,
                topdown_racer_core::simulation::FinishStatus::Dnf
            );
            assert_eq!(final_state[frozen_winner].position, 1);
            assert_eq!(final_state[frozen_winner].completed_laps, 3);
            assert_eq!(sim.tick(&[]), final_state, "a finished Race cannot advance");
            return;
        }
    }
    panic!("opponents must finish");
}

#[test]
fn directional_gate_accepts_swept_forward_crossings_only_inside_its_width() {
    let gate = topdown_racer_core::track::DirectionalGate {
        center: Vec2::new(10.0, 0.0),
        direction: Vec2::X,
        half_width: 5.0,
    };
    assert!(gate.crossed(Vec2::new(-100.0, 0.0), Vec2::new(100.0, 0.0)));
    assert!(!gate.crossed(Vec2::new(100.0, 0.0), Vec2::new(-100.0, 0.0)));
    assert!(!gate.crossed(Vec2::new(10.0, 0.0), Vec2::new(10.0, 0.0)));
    assert!(!gate.crossed(Vec2::new(9.0, 6.0), Vec2::new(11.0, 6.0)));
    assert!(!gate.crossed(Vec2::new(9.0, -6.0), Vec2::new(9.0, 6.0)));
}

#[test]
fn equal_tick_finish_uses_stable_grid_order() {
    use topdown_racer_core::{
        ai::{AiDriver, AiView},
        simulation::{FinishStatus, GridCar, RacePhase},
    };
    let track=Track::parse(r#"{"name":"Wide tie fixture","width":200,"points":[[0,0],[200,0],[200,200],[0,200],[0,0]],"surfaces":[]}"#).unwrap();
    let mut sim = Sim::from_grid(
        track.clone(),
        &[
            GridCar {
                pose: Vec2::new(10.0, 0.0),
                heading: 0.0,
                velocity: Vec2::ZERO,
            },
            GridCar {
                pose: Vec2::new(10.0, 8.0),
                heading: 0.0,
                velocity: Vec2::ZERO,
            },
        ],
    );
    let mut driver = AiDriver::new(0);
    for _ in 0..16000 {
        let previous = sim.snapshots();
        let car = previous[0];
        let field = [(car.pose, car.velocity)];
        let input = driver.compute_input(AiView {
            active: None,
            car_index: 0,
            pose: car.pose,
            heading: car.heading,
            velocity: car.velocity,
            track: &track,
            field: &field,
        });
        let current = sim.tick(&[input, input]);
        if sim.phase() == RacePhase::Finished {
            assert_eq!(current[0].completed_laps, 3);
            assert_eq!(current[1].completed_laps, 3);
            assert_eq!(current[0].lap_times, current[1].lap_times);
            assert_eq!(
                current[0].finish_status,
                FinishStatus::Finished { place: 1 }
            );
            assert_eq!(
                current[1].finish_status,
                FinishStatus::Finished { place: 2 }
            );
            return;
        }
    }
    panic!("equal-input field must finish");
}

#[test]
fn lapped_player_takes_flag_on_next_valid_crossing_after_winner() {
    use topdown_racer_core::{
        ai::{AiDriver, AiView},
        simulation::{FinishStatus, RacePhase},
    };
    let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
    let mut sim = Sim::new_race(track.clone(), 2);
    let mut driver = AiDriver::new(0);
    let mut winner = false;
    for _ in 0..14000 {
        let previous = sim.snapshots();
        let car = previous[0];
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
        input.throttle = input.throttle.min(0.2);
        let current = sim.tick(&[input]);
        winner |= matches!(
            current[1].finish_status,
            FinishStatus::Finished { place: 1 }
        );
        if matches!(current[0].finish_status, FinishStatus::Finished { .. }) {
            assert!(winner);
            assert!(current[0].completed_laps < 3);
            assert_eq!(current[0].completed_laps, previous[0].completed_laps + 1);
            assert_eq!(
                current[0].finish_status,
                FinishStatus::Finished { place: 2 }
            );
            return;
        }
        if sim.phase() == RacePhase::Finished {
            panic!("fixture player must reach gate before DNF expiry");
        }
    }
    panic!("lapped player never classified");
}

#[test]
fn skipped_checkpoints_cannot_credit_a_lap_at_the_gate() {
    use topdown_racer_core::simulation::GridCar;
    let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
    // Placed on the grid facing the checker with rolling speed: the car
    // sweeps through the gate within the first second of the Race, but it
    // never cleared a single earlier checkpoint on the way.
    let mut sim = Sim::from_grid(
        track,
        &[GridCar {
            pose: Vec2::new(0.0, 0.0),
            heading: 0.0,
            velocity: Vec2::new(50.0, 0.0),
        }],
    );

    let mut previous = sim.snapshots()[0];
    let mut swept_the_gate = false;
    for tick in 0..100 {
        let snap = sim.tick(&[])[0];
        swept_the_gate |= previous.pose.x <= 10.0 && snap.pose.x > 10.0;
        assert_eq!(
            snap.completed_laps, 0,
            "tick {tick}: a gate crossing without the ordered route cannot credit a lap"
        );
        assert_eq!(snap.lap_times, [None; 3], "tick {tick}");
        previous = snap;
    }
    assert!(
        swept_the_gate,
        "fixture must actually sweep the unchecked car through the gate"
    );
}

#[test]
fn autopilot_lap_is_ineligible_for_a_manual_record() {
    use topdown_racer_core::ai::AiDriver;
    let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
    let mut sim = Sim::new(track, 1);
    sim.set_ai(0, Some(AiDriver::new(0)));

    let mut ticks = 0;
    while sim.snapshots()[0].completed_laps < 1 {
        sim.tick(&[]);
        ticks += 1;
        assert!(
            ticks < 4000,
            "autopilot must complete a lap; gave up after {ticks} ticks"
        );
    }
    let first = sim.snapshots()[0];
    assert!(
        first.best_lap_time.is_some(),
        "an autopilot lap still records a lap time"
    );
    assert!(
        first.best_manual_lap_time.is_none(),
        "an autopilot lap is never eligible for the manual record"
    );

    // The first completed lap can never be a manual record (no completed
    // lap precedes it), so drive a second assisted lap to prove the
    // eligibility rule itself rejects autopilot laps.
    while sim.snapshots()[0].completed_laps < 2 {
        sim.tick(&[]);
        ticks += 1;
        assert!(
            ticks < 8000,
            "autopilot must complete a second lap; gave up after {ticks} ticks"
        );
    }
    let second = sim.snapshots()[0];
    assert!(second.lap_assisted[1], "lap 2 must be flagged assisted");
    assert!(
        second.best_manual_lap_time.is_none(),
        "autopilot laps never earn the manual record"
    );
    assert!(second.best_lap_time.is_some());
}
