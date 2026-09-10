use topdown_racer_core::{
    simulation::{CarInput, Sim},
    track::{Track, SAMPLE_CIRCUIT},
};

#[test]
fn snapshots_echo_scripted_controls_for_player_and_ai_cars_each_tick() {
    let mut sim = Sim::new(Track::parse(SAMPLE_CIRCUIT).unwrap(), 4);
    sim.enable_ai_opponents();
    let script = [
        [
            CarInput {
                throttle: 0.25,
                brake: 0.0,
                steer: 0.1,
                handbrake: false,
            },
            CarInput {
                throttle: 0.5,
                brake: 0.25,
                steer: -0.2,
                handbrake: true,
            },
            CarInput {
                throttle: 0.75,
                brake: 0.5,
                steer: 0.3,
                handbrake: false,
            },
            CarInput {
                throttle: 1.0,
                brake: 0.75,
                steer: -0.4,
                handbrake: true,
            },
        ],
        [
            CarInput {
                throttle: 0.0,
                brake: 1.0,
                steer: -0.5,
                handbrake: true,
            },
            CarInput {
                throttle: 0.75,
                brake: 0.0,
                steer: 0.6,
                handbrake: false,
            },
            CarInput {
                throttle: 0.5,
                brake: 0.25,
                steer: -0.7,
                handbrake: true,
            },
            CarInput {
                throttle: 0.25,
                brake: 0.5,
                steer: 0.8,
                handbrake: false,
            },
        ],
    ];
    for inputs in script {
        let stepped = sim.tick(&inputs);
        for snapshots in [stepped, sim.snapshots()] {
            for (car, expected) in snapshots.iter().zip(inputs) {
                assert_eq!(
                    (car.throttle, car.brake, car.steer, car.handbrake),
                    (
                        expected.throttle,
                        expected.brake,
                        expected.steer,
                        expected.handbrake
                    ),
                );
            }
        }
    }
}

#[test]
fn fresh_and_countdown_snapshots_echo_neutral_then_applied_controls_at_green() {
    let mut sim = Sim::new_race(Track::parse(SAMPLE_CIRCUIT).unwrap(), 4);
    let assert_neutral = |snapshots: Vec<topdown_racer_core::simulation::CarSnapshot>| {
        for car in snapshots {
            assert_eq!((car.throttle, car.brake, car.handbrake), (0.0, 0.0, false));
        }
    };
    assert_neutral(sim.snapshots());
    sim.start_countdown(2);
    let inputs = [CarInput {
        throttle: 0.5,
        brake: 0.25,
        steer: 0.0,
        handbrake: true,
    }; 4];
    assert_neutral(sim.tick(&inputs));
    assert_neutral(sim.snapshots());
    for car in sim.tick(&inputs) {
        assert_eq!((car.throttle, car.brake, car.handbrake), (0.5, 0.25, true));
    }
    // Missing player input releases the controls instead of retaining the echo.
    let player = sim.tick(&[])[0];
    assert_eq!(
        (player.throttle, player.brake, player.handbrake),
        (0.0, 0.0, false)
    );
}

#[test]
fn ai_control_echo_replays_the_same_car_motion_through_scripted_inputs() {
    let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
    let mut driven = Sim::new(track.clone(), 4);
    driven.enable_ai_opponents();
    let mut replay = Sim::new(track, 4);
    let mut saw_throttle = [false; 4];
    let mut saw_brake = [false; 4];
    for tick in 0..4000 {
        let player = CarInput {
            throttle: if tick % 200 < 100 { 0.5 } else { 0.0 },
            brake: if tick % 200 >= 100 { 0.25 } else { 0.0 },
            steer: 0.1,
            handbrake: tick % 200 >= 150,
        };
        let actual = driven.tick(&[player]);
        let echoed: Vec<_> = actual
            .iter()
            .map(|car| CarInput {
                throttle: car.throttle,
                brake: car.brake,
                steer: car.steer,
                handbrake: car.handbrake,
            })
            .collect();
        // Replaying the exact echoed controls reproduces motion and race timing.
        let replayed = replay.tick(&echoed);
        for (driven, mut replayed) in actual.iter().copied().zip(replayed) {
            // Controller ownership is provenance, not an echoed physics input.
            // A scripted Manual replay intentionally has different record
            // eligibility; compare all motion/timing fields independently of it.
            replayed.driving_mode = driven.driving_mode;
            replayed.current_lap_assisted = driven.current_lap_assisted;
            replayed.lap_assisted = driven.lap_assisted;
            replayed.best_manual_lap_time = driven.best_manual_lap_time;
            assert_eq!(
                driven, replayed,
                "echo must reproduce motion at tick {tick}"
            );
        }
        for (i, car) in actual.iter().enumerate() {
            saw_throttle[i] |= car.throttle > 0.0;
            saw_brake[i] |= car.brake > 0.0;
        }
    }
    assert!(saw_throttle.into_iter().all(|seen| seen));
    assert!(saw_brake.into_iter().all(|seen| seen));
}
