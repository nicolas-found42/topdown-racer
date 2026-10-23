use glam::Vec2;
use topdown_racer_core::{
    simulation::{CarInput, GridCar, Sim, ASSIST_MOMENT_LIMIT, FIXED_HZ},
    track::{Surface, Track},
};
fn track(surface: &str) -> Track {
    Track::parse(&format!(r#"{{"name":"Handling fixture","width":200,"points":[[0,0],[10000,0],[10000,1000],[0,1000],[0,0]],"surfaces":[{{"start":0,"end":1,"surface":"{surface}"}}]}}"#)).unwrap()
}
fn rolling(speed: f32, surface: &str, assist: bool) -> Sim {
    let mut sim = Sim::from_grid(
        track(surface),
        &[GridCar {
            pose: Vec2::new(100., 0.),
            heading: 0.,
            velocity: Vec2::new(speed, 0.),
        }],
    );
    sim.set_stability_assist(assist);
    sim
}
#[test]
fn ordinary_corners_are_gripped_silent_telemetry_without_assist_intervention() {
    for speed in [5., 10., 20.] {
        for steer in [0.025, 0.05, 0.1] {
            for assist in [false, true] {
                let mut sim = rolling(speed, "road", assist);
                for _ in 0..(FIXED_HZ * 2.) as u32 {
                    let s = sim.tick(&[CarInput {
                        steer,
                        throttle: 0.2,
                        ..Default::default()
                    }])[0];
                    assert!(!s.drifting);
                    assert!(
                        s.wheels.iter().all(|w| !w.beyond_limit),
                        "speed {speed} steer {steer}: {:?}",
                        s.wheels
                    );
                    assert_eq!(s.assist_moment, 0.);
                }
            }
        }
    }
}
#[test]
fn overspeed_entry_exceeds_front_grip_without_becoming_drift() {
    let mut sim = rolling(25., "road", true);
    let (mut front, mut rear) = (0, 0);
    for _ in 0..120 {
        let s = sim.tick(&[CarInput {
            steer: 1.,
            ..Default::default()
        }])[0];
        front += s.wheels[..2].iter().filter(|w| w.beyond_limit).count();
        rear += s.wheels[2..].iter().filter(|w| w.beyond_limit).count();
        assert!(!s.drifting);
    }
    assert!(front > 100 && rear == 0, "front {front}, rear {rear}");
}
#[test]
fn handbrake_locks_rear_wheels_and_countersteer_can_hold_drift_with_assist() {
    let mut sim = rolling(25., "road", true);
    let (mut rear_marks, mut drift, mut streak, mut longest) = (0, 0, 0, 0);
    for t in 0..360 {
        let s = sim.tick(&[CarInput {
            steer: if t < 45 { 0.4 } else { -0.1 },
            handbrake: t < 45,
            throttle: 0.0,
            ..Default::default()
        }])[0];
        rear_marks += s.wheels[2..].iter().filter(|w| w.beyond_limit).count();
        drift += s.drifting as u32;
        streak = if s.drifting { streak + 1 } else { 0 };
        longest = longest.max(streak);
    }
    println!("rear marks {rear_marks} drift ticks {drift} longest {longest}");
    assert!(rear_marks > 20 && longest >= 240);
}
#[test]
fn stationary_cars_and_reverse_are_stable_at_120hz() {
    let mut sim = rolling(0., "road", true);
    let initial = sim.snapshots()[0].pose;
    for _ in 0..1200 {
        let s = sim.tick(&[])[0];
        assert_eq!(s.pose, initial);
        assert_eq!(s.velocity, Vec2::ZERO);
        assert_eq!(s.yaw_rate, 0.);
    }
    for _ in 0..29 {
        assert!(
            sim.tick(&[CarInput {
                brake: 1.,
                ..Default::default()
            }])[0]
                .forward_speed
                >= 0.
        );
    }
    let mut last = sim.snapshots()[0];
    for _ in 0..120 {
        last = sim.tick(&[CarInput {
            brake: 1.,
            ..Default::default()
        }])[0];
    }
    assert!(last.forward_speed < -1. && last.gear == -1);
}
#[test]
fn assist_never_changes_steering_and_remains_bounded_and_dissipative() {
    let mut on = rolling(15., "road", true);
    let mut off = rolling(15., "road", false);
    let mut corrections = 0;
    let (mut on_rotation, mut off_rotation) = (0.0, 0.0);
    for t in 0..360 {
        let input = CarInput {
            steer: if t < 90 { 0.2 } else { -0.1 },
            handbrake: t < 90,
            ..Default::default()
        };
        let a = on.tick(&[input])[0];
        let b = off.tick(&[input])[0];
        assert_eq!(a.steering_angle, b.steering_angle);
        assert_eq!(a.steer, b.steer);
        assert!(a.assist_moment.abs() <= ASSIST_MOMENT_LIMIT);
        assert!(a.assist_moment * a.yaw_rate <= 0.01);
        if a.wheels[2..]
            .iter()
            .all(|wheel| wheel.normalized_lateral_slip.abs() <= 1.0)
        {
            assert_eq!(a.assist_moment, 0.0);
        }
        corrections += (a.assist_moment.abs() > 0.) as u32;
        on_rotation += a.yaw_rate.abs();
        off_rotation += b.yaw_rate.abs();
    }
    println!("assist ticks {corrections}; total yaw on {on_rotation} off {off_rotation}");
    assert!(corrections > 0 && on_rotation < off_rotation * 0.6);
    assert!(off_rotation / FIXED_HZ > std::f32::consts::PI);
    assert!(on_rotation / FIXED_HZ < std::f32::consts::PI);
}
#[test]
fn surface_grip_and_wheel_telemetry_are_deterministic() {
    let mut yaw = Vec::new();
    for surface in ["road", "gravel", "grass"] {
        let mut a = rolling(20., surface, false);
        let mut b = rolling(20., surface, false);
        let mut last = a.snapshots()[0];
        for _ in 0..120 {
            let input = CarInput {
                steer: 0.2,
                ..Default::default()
            };
            last = a.tick(&[input])[0];
            assert_eq!(last, b.tick(&[input])[0]);
        }
        yaw.push(last.heading.abs());
        assert_eq!(
            last.wheels[0].surface,
            match surface {
                "road" => Surface::Road,
                "gravel" => Surface::Gravel,
                _ => Surface::Grass,
            }
        );
    }
    println!("surface headings {yaw:?}");
    assert!(yaw[0] > yaw[1] && yaw[1] > yaw[2]);
}
#[test]
fn steering_self_alignment_is_faster_than_turn_in() {
    let mut sim = rolling(10., "road", false);
    for _ in 0..10 {
        sim.tick(&[CarInput {
            steer: 1.,
            ..Default::default()
        }]);
    }
    let angle = sim.snapshots()[0].steering_angle;
    for _ in 0..10 {
        sim.tick(&[]);
    }
    assert!(angle > 0. && sim.snapshots()[0].steering_angle.abs() < 0.001);
}

#[test]
fn a_straight_handbrake_locks_only_rear_wheels() {
    let mut sim = rolling(15., "road", true);
    let mut rear = 0;
    for _ in 0..30 {
        let s = sim.tick(&[CarInput {
            handbrake: true,
            ..Default::default()
        }])[0];
        assert!(s.wheels[..2].iter().all(|w| !w.beyond_limit));
        rear += s.wheels[2..].iter().filter(|w| w.beyond_limit).count();
    }
    assert!(rear > 20);
}
#[test]
fn drivetrain_shifts_through_four_gears_and_reaches_the_calibrated_envelope() {
    let mut sim = rolling(0., "road", false);
    let mut gears = [false; 4];
    let mut drops = 0;
    let mut previous = sim.snapshots()[0];
    let mut shift_rpm = None;
    for _ in 0..4800 {
        let s = sim.tick(&[CarInput {
            throttle: 1.,
            ..Default::default()
        }])[0];
        gears[s.gear as usize - 1] = true;
        if let Some(before) = shift_rpm.take() {
            if s.engine_rpm < before * 0.95 {
                drops += 1;
            }
        }
        if s.gear > previous.gear {
            shift_rpm = Some(s.engine_rpm);
        }
        previous = s;
    }
    println!(
        "terminal speed {} gear {} rpm {}",
        previous.forward_speed, previous.gear, previous.engine_rpm
    );
    assert!(gears.into_iter().all(|reached| reached));
    assert!(drops > 0);
    assert!((28.8..=29.8).contains(&previous.forward_speed));
}
#[test]
fn clipping_a_wall_loses_speed_and_applies_yaw_without_drift_credit() {
    let mut road = track("road");
    road.widths.fill(20.);
    let mut sim = Sim::from_grid(
        road,
        &[GridCar {
            pose: Vec2::new(100., 18.),
            heading: 0.35,
            velocity: Vec2::new(12., 8.),
        }],
    );
    let mut hit = false;
    let mut yaw = 0.0_f32;
    for _ in 0..20 {
        let s = sim.tick(&[])[0];
        if s.wall_contact {
            hit = true;
            assert!(!s.drifting);
            assert!(s.velocity.length() < Vec2::new(12., 8.).length());
            yaw = yaw.max(s.yaw_rate.abs());
        }
    }
    assert!(hit && yaw > 0.1);
}

#[test]
fn trail_braking_loads_the_front_and_tightens_the_line_relative_to_power() {
    let mut turns = Vec::new();
    let mut loads = Vec::new();
    for braking in [false, true] {
        let mut sim = rolling(15., "road", false);
        let mut last = sim.snapshots()[0];
        for _ in 0..60 {
            last = sim.tick(&[CarInput {
                steer: 0.15,
                brake: if braking { 0.2 } else { 0. },
                throttle: if braking { 0. } else { 0.5 },
                ..Default::default()
            }])[0];
        }
        let front = last.wheels[..2].iter().map(|w| w.normal_load).sum::<f32>();
        let rear = last.wheels[2..].iter().map(|w| w.normal_load).sum::<f32>();
        loads.push(front - rear);
        turns.push(last.heading / last.pose.distance(Vec2::new(100., 0.)));
    }
    println!("brake vs power curvature {turns:?}, axle load difference {loads:?}");
    assert!(loads[0] < 0. && loads[1] > 0.);
    assert!(turns[1] > turns[0]);
}
#[test]
fn hard_service_braking_marks_front_lockup_and_a_gripped_coast_loses_speed() {
    let mut braking = rolling(20., "road", true);
    let mut coast = rolling(20., "road", true);
    let mut front = 0;
    let mut last = coast.snapshots()[0];
    for _ in 0..60 {
        let s = braking.tick(&[CarInput {
            brake: 1.,
            ..Default::default()
        }])[0];
        front += s.wheels[..2].iter().filter(|w| w.beyond_limit).count();
        if s.forward_speed > 5.0 {
            assert!(s.wheels.iter().all(|wheel| wheel.slip_ratio >= -1.0001));
        }
        last = coast.tick(&[])[0];
    }
    assert!(front > 0);
    assert!(last.forward_speed < 20.);
    assert!(last.wheels.iter().all(|wheel| !wheel.beyond_limit));
}

#[test]
fn rear_wheel_drive_spins_only_driven_wheels_and_a_full_slide_marks_all_four() {
    let mut sim = rolling(0., "road", false);
    let mut rear = 0;
    for _ in 0..120 {
        let s = sim.tick(&[CarInput {
            throttle: 1.,
            ..Default::default()
        }])[0];
        assert!(s.wheels[..2].iter().all(|wheel| !wheel.beyond_limit));
        rear += s.wheels[2..]
            .iter()
            .filter(|wheel| wheel.beyond_limit)
            .count();
    }
    assert!(rear > 0);
    let mut sliding = Sim::from_grid(
        track("road"),
        &[GridCar {
            pose: Vec2::new(100., 0.),
            heading: 0.,
            velocity: Vec2::new(5., 15.),
        }],
    );
    let snapshot = sliding.tick(&[])[0];
    assert!(snapshot.wheels.iter().all(|wheel| wheel.beyond_limit));
    assert!(!snapshot.drifting);
}
