//! Reproducible handling-envelope and clean-field measurements for issue #62.
use glam::Vec2;
use topdown_racer_core::{
    simulation::{CarInput, DrivingMode, FinishStatus, GridCar, RacePhase, Sim, FIXED_HZ},
    track::{Surface, Track, HILLSIDE_CIRCUIT, SAMPLE_CIRCUIT},
};
fn main() {
    let kind = std::env::args().nth(1).unwrap_or("field".into());
    if kind == "field" {
        for assist in [false, true] {
            for data in [SAMPLE_CIRCUIT, HILLSIDE_CIRCUIT] {
                let track = Track::parse(data).unwrap();
                let name = track.name.clone();
                let mut sim = Sim::new_race(track, 4);
                sim.set_stability_assist(assist);
                sim.request_player_mode(DrivingMode::Autopilot);
                let (mut walls, mut grass, mut contacts, mut ticks) = (0, 0, 0, 0);
                let started = std::time::Instant::now();
                for t in 0..30000 {
                    let s = sim.tick(&[]);
                    ticks = t + 1;
                    walls += s.iter().filter(|s| s.wall_contact).count();
                    grass += s.iter().filter(|s| s.surface == Surface::Grass).count();
                    contacts += s.iter().filter(|s| s.car_contact).count();
                    if sim.phase() == RacePhase::Finished {
                        break;
                    }
                }
                let micros = started.elapsed().as_secs_f64() * 1e6 / ticks as f64;
                let snapshots = sim.snapshots();
                println!("{name} player_assist={assist} ticks={ticks} seconds={:.3} walls={walls} grass={grass} contacts={contacts} mean_tick_us={micros:.2} laps={:?}",ticks as f32/FIXED_HZ,snapshots.iter().map(|s|s.lap_times).collect::<Vec<_>>());
                assert_eq!((walls, grass, contacts), (0, 0, 0));
                assert!(snapshots
                    .iter()
                    .all(|s| matches!(s.finish_status, FinishStatus::Finished { .. })
                        && s.completed_laps == 3));
            }
        }
        return;
    }
    assert_eq!(kind, "straight");
    let track=Track::parse(r#"{"name":"Envelope","width":100,"points":[[0,0],[10000,0],[10000,1000],[0,1000],[0,0]],"surfaces":[]}"#).unwrap();
    let mut sim = Sim::from_grid(
        track,
        &[GridCar {
            pose: Vec2::new(100., 0.),
            heading: 0.,
            velocity: Vec2::ZERO,
        }],
    );
    for t in 0..4800 {
        let s = sim.tick(&[CarInput {
            throttle: 1.,
            ..Default::default()
        }])[0];
        if (t + 1) % 120 == 0 {
            println!(
                "t={:.0}s speed={:.4} gear={} rpm={:.1}",
                (t + 1) as f32 / FIXED_HZ,
                s.forward_speed,
                s.gear,
                s.engine_rpm
            );
        }
    }
}
