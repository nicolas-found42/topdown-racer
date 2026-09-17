use glam::Vec2;
use topdown_racer_core::{
    ai::AiDriver,
    simulation::{
        CarInput, DrivingMode, FinishStatus, GridCar, RacePhase, Sim, CAR_HALF_LENGTH,
        CAR_HALF_WIDTH,
    },
    track::{PropKind, Surface, Track, HILLSIDE_CIRCUIT},
};

#[test]
fn hillside_grid_races_cleanly_with_deterministic_results() {
    let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
    assert_eq!(track.name, "Hillside Circuit");
    let mut a = Sim::new_race(track.clone(), 4);
    let mut b = Sim::new_race(track, 4);
    a.request_player_mode(DrivingMode::Autopilot);
    b.request_player_mode(DrivingMode::Autopilot);
    let mut walls = 0;
    let mut off_road = 0;
    for tick in 0..16000 {
        let snapshots = a.tick(&[]);
        assert_eq!(snapshots, b.tick(&[]));
        walls += snapshots.iter().filter(|car| car.wall_contact).count();
        off_road += snapshots
            .iter()
            .filter(|car| car.surface == Surface::Grass)
            .count();
        if a.phase() == RacePhase::Finished {
            eprintln!(
                "Hillside: {tick} ticks, {walls} wall ticks, {off_road} off-road ticks; laps {:?}",
                snapshots.iter().map(|c| c.lap_times).collect::<Vec<_>>()
            );
            assert_eq!((walls, off_road), (0, 0));
            assert!(snapshots
                .iter()
                .all(|c| matches!(c.finish_status, FinishStatus::Finished { .. })));
            // Exclude the standing-start lap: grid position and acceleration
            // make it slower than the circuit's intended ~30-second pace.
            for car in &snapshots {
                assert!(car.lap_times.iter().all(Option::is_some));
                for lap in car.lap_times.iter().skip(1).flatten() {
                    assert!(
                        (27.0..=33.0).contains(lap),
                        "Hillside flying lap must stay within 10% of 30 s, got {lap} s"
                    );
                }
            }
            return;
        }
    }
    panic!("the field must finish");
}

#[test]
fn hillside_grid_and_scenery_leave_the_usable_road_clear() {
    let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
    let cars = Sim::new_race(track.clone(), 4).snapshots();
    for car in &cars {
        assert_eq!(
            car.heading, 0.0,
            "the entire grid faces east on the straight"
        );
        assert!(car.pose.x - CAR_HALF_LENGTH > 28.0);
        assert!(car.pose.x + CAR_HALF_LENGTH < track.finish_gate().center.x);
        assert_eq!(car.pose.y, 0.0);
        assert!(track.road_half_width_at(car.pose) > CAR_HALF_WIDTH);
    }
    for pair in cars.windows(2) {
        assert!(pair[0].pose.x - pair[1].pose.x > 2.0 * CAR_HALF_LENGTH);
    }
    for (segment, points) in track.points.windows(2).enumerate() {
        let slope = (track.widths[segment + 1] - track.widths[segment]).abs()
            / points[0].distance(points[1]);
        assert!(slope <= 0.1, "road width ramps must not pinch abruptly");
    }
    for prop in &track.props {
        // Bounding-circle clearance includes the full rotated sprite and kerb.
        let radius = match prop.kind {
            PropKind::Tree => 2.5,
            PropKind::TireStack => 2.0_f32.sqrt(),
            PropKind::BrakeBoard => 1.25,
        };
        assert!(
            track.nearest_segment(prop.position).distance
                > track.road_half_width_at(prop.position) + radius + 0.9,
            "scenery must not cover road limits: {prop:?}"
        );
    }
}

#[test]
fn hillside_braking_approach_allows_inside_and_outside_passes() {
    for blocked_side in [-1.0, 1.0] {
        let track = Track::parse(HILLSIDE_CIRCUIT).unwrap();
        let car = |x, y, speed| GridCar {
            pose: Vec2::new(x, y),
            heading: 0.0,
            velocity: Vec2::new(speed, 0.0),
        };
        let grid = [
            car(50.0, 0.0, 24.0),
            car(75.0, 0.0, 8.0),
            car(80.0, blocked_side * 4.0, 8.0),
        ];
        let mut a = Sim::from_grid(track.clone(), &grid);
        let mut b = Sim::from_grid(track, &grid);
        a.set_ai(0, Some(AiDriver::new(0)));
        b.set_ai(0, Some(AiDriver::new(0)));
        let mut overlap_ticks = 0;
        let mut completed = false;
        for tick in 0..320 {
            let inputs: Vec<_> = a
                .snapshots()
                .iter()
                .map(|s| CarInput {
                    throttle: ((8.0 - s.forward_speed) / 4.0).clamp(0.0, 1.0),
                    ..Default::default()
                })
                .collect();
            let snaps = a.tick(&inputs);
            assert_eq!(snaps, b.tick(&inputs));
            assert!(snaps
                .iter()
                .all(|s| !s.car_contact && !s.wall_contact && s.surface == Surface::Road));
            if (snaps[0].pose.x - snaps[1].pose.x).abs() < 2.0 * CAR_HALF_LENGTH {
                overlap_ticks += 1;
                assert!(snaps[0].pose.y * blocked_side < -CAR_HALF_WIDTH);
            }
            if a.pass_stats(0).unwrap().completed > 0 {
                assert!(snaps[0].pose.x > snaps[1].pose.x + 2.0 * CAR_HALF_LENGTH);
                assert!(snaps[0].pose.x < 120.0, "complete before first turn-in");
                assert!(overlap_ticks > 0, "pass must include side-by-side running");
                eprintln!("Hillside blocked_side={blocked_side}: completed at tick {tick}, overlap={overlap_ticks}, stats={:?}; zero contact/off-Track ticks", a.pass_stats(0));
                completed = true;
                break;
            }
        }
        assert!(
            completed,
            "a clear alternative corridor must permit the pass"
        );
    }
}
