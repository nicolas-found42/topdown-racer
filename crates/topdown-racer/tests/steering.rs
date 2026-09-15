use topdown_racer::controls::{SteeringFilter, SteeringResponse};

#[test]
fn smooth_steering_rises_releases_reverses_and_resets_at_fixed_ticks() {
    let mut filter = SteeringFilter::default();
    let first = filter.step(1.0, SteeringResponse::Smooth);
    assert!(first > 0.0 && first < 0.25);
    for _ in 0..6 {
        filter.step(1.0, SteeringResponse::Smooth);
    }
    assert_eq!(filter.step(1.0, SteeringResponse::Smooth), 1.0);
    for _ in 0..4 {
        filter.step(-1.0, SteeringResponse::Smooth);
    }
    assert!(filter.step(-1.0, SteeringResponse::Smooth) < 0.0);
    for _ in 0..4 {
        filter.step(0.0, SteeringResponse::Smooth);
    }
    assert_eq!(filter.step(0.0, SteeringResponse::Smooth), 0.0);
    filter.reset();
    assert_eq!(filter.step(1.0, SteeringResponse::Smooth), first);
    assert_eq!(filter.step(-1.0, SteeringResponse::Raw), -1.0);
}
