use topdown_racer_core::{
    simulation::{DrivingMode, FinishStatus, RacePhase, Sim},
    track::{Surface, Track, HILLSIDE_CIRCUIT},
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
            return;
        }
    }
    panic!("the field must finish");
}
