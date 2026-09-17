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
    decals.step(&[sample(3.0, true)]);
    let retained: Vec<_> = decals.marks().copied().skip(2).collect();
    decals.step(&[sample(4.0, true)]);
    let marks: Vec<_> = decals.marks().copied().collect();
    assert_eq!(marks.len(), 4);
    assert_eq!(&marks[..2], retained.as_slice());
    assert_eq!(marks[2].center, Vec2::new(2.4, -0.85));
    assert_eq!(marks[3].center, Vec2::new(2.4, 0.85));
    decals.clear();
    assert_eq!(decals.marks().count(), 0);
}

#[test]
fn marks_stay_pinned_at_the_real_shell_budget() {
    use topdown_racer::effects::SKID_BUDGET;
    let mut decals = SkidDecals::new(SKID_BUDGET);
    let sample = |x, drifting| DriftSample {
        pose: Vec2::new(x, 0.0),
        heading: 0.0,
        drifting,
    };
    for step in 0..=(SKID_BUDGET * 2) {
        decals.step(&[sample(step as f32, true)]);
        assert!(
            decals.marks().count() <= SKID_BUDGET,
            "mark count must never exceed the budget"
        );
    }
    assert_eq!(decals.marks().count(), SKID_BUDGET);
}

#[test]
fn capacity_never_spawns_marks_without_drift() {
    let mut decals = SkidDecals::new(8);
    let sample = |x, drifting| DriftSample {
        pose: Vec2::new(x, 0.0),
        heading: 0.0,
        drifting,
    };
    for step in 0..64 {
        decals.step(&[sample(step as f32, false)]);
        assert_eq!(decals.marks().count(), 0);
    }
}

#[test]
fn zero_budget_never_holds_marks() {
    let mut decals = SkidDecals::new(0);
    let sample = |x, drifting| DriftSample {
        pose: Vec2::new(x, 0.0),
        heading: 0.0,
        drifting,
    };
    for step in 0..8 {
        decals.step(&[sample(step as f32, true)]);
        assert_eq!(decals.marks().count(), 0);
    }
}
