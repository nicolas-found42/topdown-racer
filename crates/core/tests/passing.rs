use glam::Vec2;
use topdown_racer_core::{
    ai::{AiDriver, AiView},
    track::{Track, SAMPLE_CIRCUIT},
};

#[test]
fn committed_pass_does_not_swap_sides_when_the_rival_moves_across_center() {
    let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
    let mut driver = AiDriver::new(1);
    let me = (Vec2::new(40.0, 0.0), Vec2::new(20.0, 0.0));
    for lateral in [0.6, -0.6, 0.6, -0.6] {
        for _ in 0..24 {
            let input = driver.compute_input(AiView {
                active: None,
                car_index: 0,
                pose: me.0,
                heading: 0.0,
                velocity: me.1,
                track: &track,
                field: &[me, (Vec2::new(56.0, lateral), Vec2::new(12.0, 0.0))],
            });
            assert!(
                input.steer < 0.0,
                "committed right pass must not switch left: {input:?}"
            );
        }
    }
}

#[test]
fn blocked_committed_route_aborts_and_follows_without_switching_sides() {
    let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
    let mut driver = AiDriver::new(1);
    let me = (Vec2::new(40.0, 0.0), Vec2::new(20.0, 0.0));
    let lead = (Vec2::new(56.0, 0.0), Vec2::new(10.0, 0.0));
    for _ in 0..20 {
        driver.compute_input(AiView {
            active: None,
            car_index: 0,
            pose: me.0,
            heading: 0.0,
            velocity: me.1,
            track: &track,
            field: &[me, lead],
        });
    }
    let blocker = (Vec2::new(48.0, -4.0), Vec2::new(5.0, 0.0));
    for _ in 0..60 {
        let input = driver.compute_input(AiView {
            active: None,
            car_index: 0,
            pose: me.0,
            heading: 0.0,
            velocity: me.1,
            track: &track,
            field: &[me, lead, blocker],
        });
        assert!(input.brake > 0.5, "blocked pass must brake: {input:?}");
        assert!(
            input.steer <= 0.0,
            "abort must not switch directly to the other side"
        );
    }
    assert_eq!(driver.pass_stats().attempted, 1);
    assert_eq!(driver.pass_stats().aborted, 1);
}

fn passing_track(width: f32) -> Track {
    Track::parse(&format!(r#"{{"name":"Passing straight", "width":{width}, "points":[[0,0],[220,0],[260,40],[260,100],[220,140],[-180,140],[-220,100],[-220,40],[-180,0],[0,0]], "surfaces":[]}}"#)).unwrap()
}

#[test]
fn faster_car_completes_clean_pass_across_the_track_seam() {
    use topdown_racer_core::{ai::OpponentPace, simulation::Sim};
    let track = passing_track(20.0);
    let mut sim = Sim::new(track, 2);
    sim.set_ai(0, Some(AiDriver::with_pace(0, OpponentPace::Touring)));
    sim.set_ai(1, Some(AiDriver::with_pace(1, OpponentPace::Race)));
    let mut near_ticks = 0;
    let mut contacts = 0;
    let mut severity = 0.0_f32;
    for tick in 0..1800 {
        let snaps = sim.tick(&[]);
        if snaps[0].pose.distance(snaps[1].pose) < 28.0 {
            near_ticks += 1;
        }
        contacts += snaps.iter().filter(|s| s.car_contact).count();
        severity = severity.max(
            snaps
                .iter()
                .map(|s| s.car_contact_speed)
                .fold(0.0, f32::max),
        );
        assert!(!snaps[1].wall_contact);
        if sim.pass_stats(1).unwrap().completed > 0 {
            eprintln!("seam pass: tick={tick}, stats={:?}, contacts={contacts}, severity={severity}, near_ticks={near_ticks}", sim.pass_stats(1));
            assert_eq!(contacts, 0);
            assert_eq!(snaps[1].position, 1);
            return;
        }
    }
    panic!(
        "no pass completed: {:?}; contacts={contacts}, severity={severity}",
        sim.pass_stats(1)
    );
}

#[test]
fn established_inside_overlap_prevents_turning_across_the_other_car() {
    let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
    for ahead in [-4.4_f32, 0.0, 4.4] {
        let mut driver = AiDriver::new(1);
        let me = (Vec2::new(106.0, -2.0), Vec2::new(12.0, 0.0));
        let rival = (Vec2::new(106.0 + ahead, 1.2), Vec2::new(12.0, 0.0));
        for _ in 0..40 {
            let input = driver.compute_input(AiView {
                active: None,
                car_index: 0,
                pose: me.0,
                heading: 0.0,
                velocity: me.1,
                track: &track,
                field: &[me, rival],
            });
            assert!(
                input.steer <= 0.02,
                "overlap at {ahead} must keep the outside lane: {input:?}"
            );
        }
    }
}

#[test]
fn no_safe_pass_corridor_waits_without_reverse_or_repeated_attempts() {
    let track = passing_track(6.0);
    let mut driver = AiDriver::new(1);
    let me = (Vec2::new(40.0, 0.0), Vec2::ZERO);
    let lead = (Vec2::new(47.0, 0.0), Vec2::ZERO);
    for _ in 0..300 {
        let input = driver.compute_input(AiView {
            active: None,
            car_index: 0,
            pose: me.0,
            heading: 0.0,
            velocity: me.1,
            track: &track,
            field: &[me, lead],
        });
        assert_eq!(input.throttle, 0.0, "wait behind the stopped Car");
        assert_eq!(input.brake, 0.0, "do not reverse into following traffic");
        assert_eq!(input.steer, 0.0);
    }
    assert_eq!(driver.pass_stats().attempted, 0);
    let clear = driver.compute_input(AiView {
        active: None,
        car_index: 0,
        pose: me.0,
        heading: 0.0,
        velocity: me.1,
        track: &track,
        field: &[me],
    });
    assert!(
        clear.throttle > 0.0,
        "drive as soon as the obstruction leaves"
    );
}

#[test]
fn late_braking_approach_slows_before_reaching_the_minimum_gap() {
    let track = passing_track(6.0);
    let mut driver = AiDriver::new(1);
    let me = (Vec2::new(40.0, 0.0), Vec2::new(28.0, 0.0));
    let input = driver.compute_input(AiView {
        active: None,
        car_index: 0,
        pose: me.0,
        heading: 0.0,
        velocity: me.1,
        track: &track,
        field: &[me, (Vec2::new(60.0, 0.0), Vec2::new(5.0, 0.0))],
    });
    assert!(
        input.brake > 0.5,
        "brake while there is still stopping distance: {input:?}"
    );
    assert_eq!(input.throttle, 0.0);
    assert_eq!(driver.pass_stats().attempted, 0);
}

#[test]
fn inside_blockage_selects_the_clear_outside_route() {
    let track = passing_track(20.0);
    let mut driver = AiDriver::new(1);
    let me = (Vec2::new(40.0, 0.0), Vec2::new(20.0, 0.0));
    let field = [
        me,
        (Vec2::new(56.0, 0.0), Vec2::new(10.0, 0.0)),
        (Vec2::new(55.0, 4.0), Vec2::new(10.0, 0.0)),
    ];
    for _ in 0..30 {
        let input = driver.compute_input(AiView {
            active: None,
            car_index: 0,
            pose: me.0,
            heading: 0.0,
            velocity: me.1,
            track: &track,
            field: &field,
        });
        assert!(input.steer < 0.0);
    }
    assert_eq!(driver.pass_stats().attempted, 1);
    assert_eq!(driver.pass_stats().aborted, 0);
}

#[test]
fn slower_rejoin_into_a_committed_route_aborts_before_overlap() {
    let track = passing_track(20.0);
    let mut driver = AiDriver::new(1);
    let me = (Vec2::new(40.0, 0.0), Vec2::new(20.0, 0.0));
    let lead = (Vec2::new(56.0, 0.0), Vec2::new(10.0, 0.0));
    driver.compute_input(AiView {
        active: None,
        car_index: 0,
        pose: me.0,
        heading: 0.0,
        velocity: me.1,
        track: &track,
        field: &[me, lead],
    });
    let rejoin = (Vec2::new(52.0, -8.0), Vec2::new(5.0, 4.0));
    let input = driver.compute_input(AiView {
        active: None,
        car_index: 0,
        pose: me.0,
        heading: 0.0,
        velocity: me.1,
        track: &track,
        field: &[me, lead, rejoin],
    });
    assert!(input.brake > 0.5);
    assert_eq!(driver.pass_stats().aborted, 1);
}
