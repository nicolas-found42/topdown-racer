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
