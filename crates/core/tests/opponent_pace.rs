use topdown_racer_core::{
    ai::{AiDriver, OpponentPace},
    simulation::{RacePhase, Sim},
    track::{Surface, Track, HILLSIDE_CIRCUIT, SAMPLE_CIRCUIT},
};

#[test]
fn repeated_clean_solo_runs_order_the_three_paces() {
    for circuit in [SAMPLE_CIRCUIT, HILLSIDE_CIRCUIT] {
        let track = Track::parse(circuit).unwrap();
        let mut times = Vec::new();
        for pace in [
            OpponentPace::Touring,
            OpponentPace::Club,
            OpponentPace::Race,
        ] {
            let mut runs = Vec::new();
            for _ in 0..2 {
                let mut sim = Sim::new(track.clone(), 1);
                sim.set_ai(0, Some(AiDriver::with_pace(0, pace)));
                for _ in 0..20000 {
                    let snap = sim.tick(&[])[0];
                    assert!(!snap.wall_contact, "{pace:?} wall contact");
                    assert_ne!(snap.surface, Surface::Grass, "{pace:?} left the road");
                    if snap.phase == RacePhase::Finished {
                        runs.push(snap.lap_times[1].unwrap());
                        break;
                    }
                }
            }
            assert_eq!(runs.len(), 2);
            assert_eq!(runs[0], runs[1]);
            times.push(runs[0]);
        }
        eprintln!(
            "{} Touring / Club / Race flying laps: {times:?}",
            track.name
        );
        assert!(times[0] > times[1] && times[1] > times[2]);
    }
}
