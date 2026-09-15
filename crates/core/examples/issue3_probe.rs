use topdown_racer_core::{
    simulation::Sim,
    track::{Track, SAMPLE_CIRCUIT},
};
fn main() {
    for count in 2..=4 {
        let mut sim = Sim::new(Track::parse(SAMPLE_CIRCUIT).unwrap(), count);
        sim.enable_ai_opponents();
        for tick in 0..2500 {
            let snaps = sim.tick(&[]);
            if tick == 2499 {
                for (i, s) in snaps.iter().enumerate().skip(1) {
                    eprintln!(
                        "[DEBUG-issue3] count={count} car={i} laps={} pose={:?} stats={:?}",
                        s.completed_laps,
                        s.pose,
                        sim.pass_stats(i)
                    );
                }
            }
        }
    }
}
