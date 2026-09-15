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
