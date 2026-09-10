use glam::Vec2;
use topdown_racer_core::{
    ai::AiDriver,
    simulation::{CarInput, GridCar, Sim},
    track::Track,
};

fn track() -> Track {
    Track::parse(r#"{"name":"Encounters", "width":20, "points":[[0,0],[220,0],[260,40],[260,100],[220,140],[-180,140],[-220,100],[-220,40],[-180,0],[0,0]], "surfaces":[]}"#).unwrap()
}
fn car(x: f32, y: f32, speed: f32) -> GridCar {
    GridCar {
        pose: Vec2::new(x, y),
        heading: 0.0,
        velocity: Vec2::new(speed, 0.0),
    }
}

#[test]
fn inside_blockage_leaves_a_clean_outside_pass_in_motion() {
    let mut sim = Sim::from_grid(
        track(),
        &[
            car(35.0, 0.0, 24.0),
            car(60.0, 0.0, 12.0),
            car(65.0, 4.0, 12.0),
        ],
    );
    sim.set_ai(0, Some(AiDriver::new(0)));
    let mut contacts = 0;
    let mut severity = 0.0_f32;
    let mut near = 0;
    for tick in 0..600 {
        let before = sim.snapshots();
        let inputs: Vec<_> = before
            .iter()
            .map(|s| CarInput {
                throttle: ((12.0 - s.forward_speed) / 4.0).clamp(0.0, 1.0),
                ..Default::default()
            })
            .collect();
        let snaps = sim.tick(&inputs);
        contacts += snaps.iter().filter(|s| s.car_contact).count();
        severity = severity.max(
            snaps
                .iter()
                .map(|s| s.car_contact_speed)
                .fold(0.0, f32::max),
        );
        near += usize::from(snaps[0].pose.distance(snaps[1].pose) < 28.0);
        if sim.pass_stats(0).unwrap().completed > 0 {
            eprintln!("outside pass: {tick} ticks, {:?}, contacts={contacts}, severity={severity}, near={near}", sim.pass_stats(0));
            assert_eq!(contacts, 0);
            assert!(snaps[0].pose.x > snaps[1].pose.x + 4.4);
            return;
        }
    }
    panic!(
        "pass failed: {:?}, contacts={contacts}, severity={severity}",
        sim.pass_stats(0)
    );
}

#[test]
fn late_braking_in_a_narrow_corridor_follows_without_contact() {
    let mut narrow = track();
    narrow.widths.fill(6.0);
    let mut sim = Sim::from_grid(narrow, &[car(35.0, 0.0, 28.0), car(60.0, 0.0, 5.0)]);
    sim.set_ai(0, Some(AiDriver::new(0)));
    let mut min_gap = f32::INFINITY;
    let mut max_contact = 0.0_f32;
    for _ in 0..300 {
        let speed = sim.snapshots()[1].forward_speed;
        let snaps = sim.tick(&[
            CarInput::default(),
            CarInput {
                throttle: ((5.0 - speed) / 4.0).clamp(0.0, 1.0),
                ..Default::default()
            },
        ]);
        min_gap = min_gap.min(snaps[1].pose.x - snaps[0].pose.x);
        max_contact = max_contact.max(snaps[0].car_contact_speed);
        assert!(
            !snaps[0].car_contact,
            "late approach contacted at gap={min_gap}, severity={max_contact}"
        );
        assert!(snaps[0].forward_speed >= 0.0);
    }
    assert_eq!(sim.pass_stats(0).unwrap().attempted, 0);
    assert!(min_gap > 4.4);
    eprintln!("narrow / late braking: {:?}, contacts=0, severity={max_contact}, near_ticks=300, minimum_gap={min_gap}", sim.pass_stats(0));
}

#[test]
fn moving_rejoin_aborts_the_pass_without_contact() {
    let rejoin = GridCar {
        pose: Vec2::new(56.0, -12.0),
        heading: 0.65,
        velocity: Vec2::new(6.4, 4.8),
    };
    let mut sim = Sim::from_grid(
        track(),
        &[car(35.0, 0.0, 20.0), car(60.0, 0.0, 10.0), rejoin],
    );
    sim.set_ai(0, Some(AiDriver::new(0)));
    let mut near = 0;
    let mut max_contact = 0.0_f32;
    for _ in 0..150 {
        let before = sim.snapshots();
        let snaps = sim.tick(&[
            CarInput::default(),
            CarInput {
                throttle: ((10.0 - before[1].forward_speed) / 4.0).clamp(0.0, 1.0),
                ..Default::default()
            },
            CarInput {
                throttle: ((8.0 - before[2].forward_speed) / 4.0).clamp(0.0, 1.0),
                ..Default::default()
            },
        ]);
        near += usize::from(snaps[0].pose.distance(snaps[1].pose) < 28.0);
        max_contact = max_contact.max(snaps[0].car_contact_speed);
        assert!(
            !snaps[0].car_contact,
            "rejoin contact severity={max_contact}"
        );
    }
    let stats = sim.pass_stats(0).unwrap();
    eprintln!(
        "moving rejoin: {stats:?}, player contacts=0, severity={max_contact}, near_ticks={near}"
    );
    assert!(stats.attempted >= 1 && stats.aborted >= 1);
}
