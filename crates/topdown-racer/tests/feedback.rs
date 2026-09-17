use topdown_racer::{feedback::FeedbackMixer, ShellSimulation};
use topdown_racer_core::track::{Surface, Track, HILLSIDE_CIRCUIT};
#[test]
fn feedback_distinguishes_load_surface_and_bounded_contact_onset() {
    let shell = ShellSimulation::new(Track::parse(HILLSIDE_CIRCUIT).unwrap());
    let mut car = shell.curr_snapshots[0];
    car.forward_speed = 20.0;
    car.velocity.x = 20.0;
    let mut mixer = FeedbackMixer::default();
    let coast = mixer.step(&car);
    car.throttle = 1.0;
    assert!(mixer.step(&car).engine_volume > coast.engine_volume);
    car.surface = Surface::Gravel;
    assert!(mixer.step(&car).surface_volume > 0.0);
    car.car_contact = true;
    car.car_contact_speed = 15.0;
    let impact = mixer.step(&car).impact_volume;
    assert!(impact > 0.0 && impact <= 0.8);
    for _ in 0..64 {
        mixer.step(&car);
    }
    assert_eq!(mixer.step(&car).impact_volume, 0.0);
    car.car_contact = false;
    for _ in 0..16 {
        mixer.step(&car);
    }
    car.car_contact = true;
    assert!(mixer.step(&car).impact_volume > 0.0);
}

#[test]
fn stationary_contact_chatter_requires_eight_clear_ticks_to_rearm() {
    let shell = ShellSimulation::new(Track::parse(HILLSIDE_CIRCUIT).unwrap());
    let mut car = shell.curr_snapshots[0];
    let mut mixer = FeedbackMixer::default();
    car.wall_contact = true;
    assert!(mixer.step(&car).impact_started);
    for _ in 0..64 {
        car.wall_contact = false;
        assert!(!mixer.step(&car).impact_started);
        car.wall_contact = true;
        assert!(!mixer.step(&car).impact_started);
    }
    car.wall_contact = false;
    for _ in 0..8 {
        mixer.step(&car);
    }
    car.wall_contact = true;
    assert!(mixer.step(&car).impact_started);
}

#[test]
fn slip_severity_rises_and_recovers_smoothly_without_braking_proxies() {
    let shell = ShellSimulation::new(Track::parse(HILLSIDE_CIRCUIT).unwrap());
    let mut car = shell.curr_snapshots[0];
    car.heading = 0.0;
    car.velocity = glam::Vec2::new(20.0, 2.0);
    car.drifting = true;
    let mut mixer = FeedbackMixer::default();
    let initial = mixer.step(&car).tire_volume;
    let mut low = initial;
    for _ in 0..64 {
        low = mixer.step(&car).tire_volume;
    }
    assert!(initial > 0.0 && low > initial);
    car.velocity.y = 6.0;
    let next = mixer.step(&car).tire_volume;
    assert!(next > low && next < 0.6);
    car.drifting = false;
    car.brake = 1.0;
    let release = mixer.step(&car).tire_volume;
    assert!(release > 0.0 && release < next);
    for _ in 0..128 {
        mixer.step(&car);
    }
    assert!(mixer.step(&car).tire_volume < 0.0001);
    car.velocity = glam::Vec2::ZERO;
    car.surface = Surface::Grass;
    let stopped = mixer.step(&car);
    assert_eq!(stopped.surface_volume, 0.0);
    assert_eq!(stopped.impact_volume, 0.0);
}

#[test]
fn presentation_mixing_does_not_change_deterministic_race_outcomes() {
    use topdown_racer_core::simulation::{CarInput, DrivingMode, RacePhase, Sim};
    let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
    let mut with_feedback = Sim::new_race(track.clone(), 4);
    let mut without_feedback = Sim::new_race(track, 4);
    with_feedback.request_player_mode(DrivingMode::Autopilot);
    without_feedback.request_player_mode(DrivingMode::Autopilot);
    let mut mixer = FeedbackMixer::default();
    for tick in 0..20000 {
        let snapshots = with_feedback.tick(&[CarInput::default(); 4]);
        assert_eq!(snapshots, without_feedback.tick(&[CarInput::default(); 4]));
        let frame = mixer.step(&snapshots[0]);
        assert!((0.0..=0.6).contains(&frame.tire_volume));
        assert!((0.0..=0.4).contains(&frame.surface_volume));
        assert!((0.0..=0.8).contains(&frame.impact_volume));
        if snapshots[0].phase == RacePhase::Finished {
            println!("identical four-Car Race finished at tick {tick}");
            return;
        }
    }
    panic!("Race did not finish");
}

#[test]
fn stronger_impacts_scale_but_never_exceed_the_ceiling() {
    let shell = ShellSimulation::new(Track::parse(HILLSIDE_CIRCUIT).unwrap());
    let mut car = shell.curr_snapshots[0];
    car.car_contact = true;
    let mut volumes = Vec::new();
    for speed in [0.0, 5.0, 10.0, 100.0] {
        car.car_contact_speed = speed;
        let mut mixer = FeedbackMixer::default();
        let frame = mixer.step(&car);
        assert!(frame.impact_started);
        volumes.push(frame.impact_volume);
        for _ in 0..14 {
            assert!(!mixer.step(&car).impact_started);
        }
        assert_eq!(mixer.step(&car).impact_volume, 0.0);
    }
    assert!(volumes.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(volumes[3], 0.8);
}
