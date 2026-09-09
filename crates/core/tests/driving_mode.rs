use topdown_racer_core::{simulation::{CarInput, DrivingMode, Sim}, track::{Track, SAMPLE_CIRCUIT}};

#[test]
fn explicit_ownership_applies_at_tick_boundary_and_autopilot_ignores_keys() {
    let mut sim = Sim::new(Track::parse(SAMPLE_CIRCUIT).unwrap(), 1);
    let initial = sim.snapshots()[0].pose;
    for _ in 0..120 { assert_eq!(sim.tick(&[])[0].pose, initial); }
    sim.request_player_mode(DrivingMode::Autopilot);
    assert_eq!(sim.snapshots()[0].driving_mode, DrivingMode::Manual);
    let keys = CarInput { brake: 1.0, steer: 1.0, handbrake: true, ..Default::default() };
    for _ in 0..120 {
        let snap = sim.tick(&[keys])[0];
        assert_eq!(snap.driving_mode, DrivingMode::Autopilot);
        assert!(!snap.handbrake);
    }
    assert!(sim.snapshots()[0].forward_speed > 5.0);
    sim.request_player_mode(DrivingMode::Manual);
    let snap = sim.tick(&[CarInput { throttle: 1.0, ..Default::default() }])[0];
    assert_eq!(snap.throttle, 0.0, "switch tick discards stale input");
    assert_eq!(snap.steer, 0.0);
    assert!(snap.current_lap_assisted);
    let coast = sim.tick(&[])[0];
    assert_eq!(coast.throttle, 0.0);
    assert!(coast.forward_speed < snap.forward_speed);
}

#[test]
fn assistance_sticks_to_the_lap_but_next_manual_flying_lap_can_set_a_record() {
    use topdown_racer_core::ai::{AiDriver, AiView};
    let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
    let mut sim = Sim::new(track.clone(), 1);
    sim.request_player_mode(DrivingMode::Autopilot);
    for _ in 0..250 { sim.tick(&[]); }
    sim.request_player_mode(DrivingMode::Manual);
    sim.tick(&[]);
    let mut scripted_driver = AiDriver::new(0);
    for _ in 0..6000 {
        let snap = sim.snapshots()[0];
        let input = scripted_driver.compute_input(AiView { car_index: 0, pose: snap.pose,
            heading: snap.heading, velocity: snap.velocity, track: &track,
            field: &[(snap.pose, snap.velocity)] });
        let snap = sim.tick(&[input])[0];
        if snap.completed_laps == 1 {
            assert!(snap.lap_assisted[0]);
            assert_eq!(snap.best_manual_lap_time, None);
            assert!(!snap.current_lap_assisted);
        }
        if snap.completed_laps == 2 {
            assert!(!snap.lap_assisted[1]);
            assert_eq!(snap.best_manual_lap_time, snap.lap_times[1]);
            return;
        }
    }
    panic!("scripted Car must complete two laps");
}

#[test]
fn identical_preset_and_mode_input_streams_replay_identical_snapshots() {
    use topdown_racer_core::ai::OpponentPace;
    for pace in [OpponentPace::Touring, OpponentPace::Club, OpponentPace::Race] {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut a = Sim::new_race_with_pace(track.clone(), 4, pace);
        let mut b = Sim::new_race_with_pace(track, 4, pace);
        for tick in 0..4500 {
            if let Some(mode) = match tick { 0 | 1800 => Some(DrivingMode::Autopilot), 1600 | 3300 => Some(DrivingMode::Manual), _ => None } {
                a.request_player_mode(mode);
                b.request_player_mode(mode);
            }
            let input = CarInput { throttle: if tick % 200 < 100 { 1.0 } else { 0.0 }, steer: 0.1, ..Default::default() };
            assert_eq!(a.tick(&[input]), b.tick(&[input]), "{pace:?} tick {tick}");
            for car in 1..4 { assert!(a.ai_enabled(car)); }
        }
    }
}
