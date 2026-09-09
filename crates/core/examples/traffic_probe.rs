use topdown_racer_core::{simulation::Sim, track::{Track,SAMPLE_CIRCUIT}};
fn main() {
 let mut sim=Sim::new_race(Track::parse(SAMPLE_CIRCUIT).unwrap(),4);
 for tick in 0..10000 {
 let s=sim.tick(&[]);
 if tick%1024==0 { for (i,c) in s.iter().enumerate().skip(1) { eprintln!("{tick} car{i}: pose={:?} v={} steer={} throttle={} brake={} laps={} stats={:?}",c.pose,c.forward_speed,c.steer,c.throttle,c.brake,c.completed_laps,sim.pass_stats(i)); } }
 }
}
