use glam::Vec2;
use topdown_racer::effects::{SkidDecals, TireSample};

#[test]
fn drift_stream_leaves_bounded_marks_and_drops_oldest_first() {
    let mut decals = SkidDecals::new(4);
    let sample = |x, drifting| TireSample {
        pose: Vec2::new(x, 0.0),
        heading: 0.0,
        wheels: std::array::from_fn(|i| topdown_racer_core::simulation::WheelSnapshot {
            local_position: topdown_racer_core::simulation::WHEEL_POSITIONS[i],
            beyond_limit: drifting && i >= 2,
            ..Default::default()
        }),
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
    assert_eq!(marks[2].center, Vec2::new(2.2, 1.05));
    assert_eq!(marks[3].center, Vec2::new(2.2, -1.05));
    decals.clear();
    assert_eq!(decals.marks().count(), 0);
}

#[test]
fn marks_stay_pinned_at_the_real_shell_budget() {
    use topdown_racer::effects::SKID_BUDGET;
    let mut decals = SkidDecals::new(SKID_BUDGET);
    let sample = |x, drifting| TireSample {
        pose: Vec2::new(x, 0.0),
        heading: 0.0,
        wheels: std::array::from_fn(|i| topdown_racer_core::simulation::WheelSnapshot {
            local_position: topdown_racer_core::simulation::WHEEL_POSITIONS[i],
            beyond_limit: drifting && i >= 2,
            ..Default::default()
        }),
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
    let sample = |x, drifting| TireSample {
        pose: Vec2::new(x, 0.0),
        heading: 0.0,
        wheels: std::array::from_fn(|i| topdown_racer_core::simulation::WheelSnapshot {
            local_position: topdown_racer_core::simulation::WHEEL_POSITIONS[i],
            beyond_limit: drifting && i >= 2,
            ..Default::default()
        }),
    };
    for step in 0..64 {
        decals.step(&[sample(step as f32, false)]);
        assert_eq!(decals.marks().count(), 0);
    }
}

#[test]
fn zero_budget_never_holds_marks() {
    let mut decals = SkidDecals::new(0);
    let sample = |x, drifting| TireSample {
        pose: Vec2::new(x, 0.0),
        heading: 0.0,
        wheels: std::array::from_fn(|i| topdown_racer_core::simulation::WheelSnapshot {
            local_position: topdown_racer_core::simulation::WHEEL_POSITIONS[i],
            beyond_limit: drifting && i >= 2,
            ..Default::default()
        }),
    };
    for step in 0..8 {
        decals.step(&[sample(step as f32, true)]);
        assert_eq!(decals.marks().count(), 0);
    }
}

#[test]
fn a_single_slipping_front_wheel_marks_its_own_rotated_contact_patch() {
    use topdown_racer_core::simulation::{WheelSnapshot, WHEEL_POSITIONS};
    let mut wheels = std::array::from_fn(|i| WheelSnapshot {
        local_position: WHEEL_POSITIONS[i],
        ..Default::default()
    });
    wheels[0].beyond_limit = true;
    let mut decals = SkidDecals::new(8);
    for y in [0., 1.] {
        decals.step(&[TireSample {
            pose: Vec2::new(10., y),
            heading: std::f32::consts::FRAC_PI_2,
            wheels,
        }]);
    }
    let marks: Vec<_> = decals.marks().collect();
    assert_eq!(marks.len(), 1);
    assert!(marks[0].center.distance(Vec2::new(8.95, 1.8)) < 0.001);
}

#[test]
fn real_gripped_corner_is_clean_while_real_handbrake_input_leaves_wheel_marks() {
    use topdown_racer_core::{
        simulation::{CarInput, GridCar, Sim},
        track::Track,
    };
    let track=Track::parse(r#"{"name":"Effects fixture","width":200,"points":[[0,0],[10000,0],[10000,1000],[0,1000],[0,0]],"surfaces":[]}"#).unwrap();
    let mut counts = Vec::new();
    for handbrake in [false, true] {
        let mut sim = Sim::from_grid(
            track.clone(),
            &[GridCar {
                pose: Vec2::new(100., 0.),
                heading: 0.,
                velocity: Vec2::new(20., 0.),
            }],
        );
        let mut decals = SkidDecals::new(2048);
        for _ in 0..120 {
            let s = sim.tick(&[CarInput {
                steer: 0.1,
                handbrake,
                ..Default::default()
            }])[0];
            decals.step(&[TireSample {
                pose: s.pose,
                heading: s.heading,
                wheels: s.wheels,
            }]);
        }
        counts.push(decals.marks().count());
    }
    assert_eq!(counts[0], 0);
    assert!(counts[1] > 0);
}
