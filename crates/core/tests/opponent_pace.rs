use topdown_racer_core::{
    ai::{AiDriver, OpponentPace},
    simulation::{RacePhase, Sim},
    track::{Track, SAMPLE_CIRCUIT},
};

#[test]
fn repeated_clean_solo_runs_order_the_three_paces() {
    let mut times = Vec::new();
    for pace in [
        OpponentPace::Touring,
        OpponentPace::Club,
        OpponentPace::Race,
    ] {
        let mut runs = Vec::new();
        for _ in 0..2 {
            let mut sim = Sim::new(Track::parse(SAMPLE_CIRCUIT).unwrap(), 1);
            sim.set_ai(0, Some(AiDriver::with_pace(0, pace)));
            for _ in 0..10000 {
                let snap = sim.tick(&[])[0];
                assert!(!snap.wall_contact, "{pace:?} wall contact");
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
    eprintln!("Touring / Club / Race flying laps: {times:?}");
    assert!(times[0] > times[1] && times[1] > times[2]);
}
