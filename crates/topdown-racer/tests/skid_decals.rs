use glam::Vec2;
use topdown_racer::effects::{DriftSample, SkidDecals};

#[test]
fn drift_stream_leaves_bounded_marks_and_drops_oldest_first() {
    let mut decals = SkidDecals::new(4);
    let sample = |x, drifting| DriftSample {
        pose: Vec2::new(x, 0.0),
        heading: 0.0,
        drifting,
    };
    decals.step(&[sample(0.0, false)]);
    decals.step(&[sample(1.0, false)]);
    assert_eq!(decals.marks().count(), 0);
    decals.step(&[sample(2.0, true)]);
    assert_eq!(decals.marks().count(), 2);
    let oldest = *decals.marks().next().unwrap();
    decals.step(&[sample(3.0, true)]);
    decals.step(&[sample(4.0, true)]);
    assert_eq!(decals.marks().count(), 4);
    assert!(!decals.marks().any(|mark| *mark == oldest));
    decals.clear();
    assert_eq!(decals.marks().count(), 0);
}
