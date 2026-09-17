//! Snapshot-driven driving effects. Geometry is independent of the engine.
use glam::Vec2;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug)]
pub struct DriftSample {
    pub pose: Vec2,
    pub heading: f32,
    pub drifting: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkidMark {
    pub center: Vec2,
    pub heading: f32,
    pub length: f32,
}

pub struct SkidDecals {
    capacity: usize,
    marks: VecDeque<SkidMark>,
    previous: Vec<Vec2>,
}

impl SkidDecals {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            marks: VecDeque::new(),
            previous: Vec::new(),
        }
    }

    pub fn marks(&self) -> impl Iterator<Item = &SkidMark> {
        self.marks.iter()
    }

    pub fn clear(&mut self) {
        self.marks.clear();
        self.previous.clear();
    }

    pub fn step(&mut self, samples: &[DriftSample]) {
        for (i, sample) in samples.iter().enumerate() {
            if i >= self.previous.len() {
                self.previous.push(sample.pose);
                continue;
            }
            let delta = sample.pose - self.previous[i];
            let length = delta.length();
            if sample.drifting && (0.25..3.0).contains(&length) {
                let forward = Vec2::from_angle(sample.heading);
                let left = Vec2::new(-forward.y, forward.x);
                for side in [-1.0, 1.0] {
                    self.marks.push_back(SkidMark {
                        center: (sample.pose + self.previous[i]) * 0.5 - forward * 1.1
                            + left * side * 0.85,
                        heading: delta.y.atan2(delta.x),
                        length,
                    });
                }
                while self.marks.len() > self.capacity {
                    self.marks.pop_front();
                }
            }
            if !sample.drifting || length >= 0.25 {
                self.previous[i] = sample.pose;
            }
        }
        self.previous.truncate(samples.len());
    }
}

use crate::{world_z, AppState, ShellSimulation};
use bevy::prelude::*;

/// One fixed pool per app: at most 2048 tire marks, retained until retry.
pub const SKID_BUDGET: usize = 2048;

#[derive(Resource)]
struct DrivingEffects {
    generation: u64,
    race_generation: u64,
    decals: SkidDecals,
    particles: VecDeque<Particle>,
    tick: u32,
}

#[derive(Component)]
struct DecalSlot(usize);

pub(crate) struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DrivingEffects {
            generation: 0,
            race_generation: 0,
            decals: SkidDecals::new(SKID_BUDGET),
            particles: VecDeque::new(),
            tick: 0,
        })
        .add_systems(Startup, (setup_decals, setup_particles))
        .add_systems(OnEnter(AppState::Race), reset_effects)
        .add_systems(
            FixedUpdate,
            advance_effects
                .after(crate::step_simulation)
                .run_if(in_state(AppState::Race)),
        )
        .add_systems(
            Update,
            (render_decals, render_particles, render_brake_lights),
        );
    }
}

fn setup_decals(mut commands: Commands) {
    for slot in 0..SKID_BUDGET {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: crate::palette::color(crate::palette::ASPHALT_DARKEST),
                    ..default()
                },
                visibility: Visibility::Hidden,
                ..default()
            },
            DecalSlot(slot),
        ));
    }
}

fn reset_effects(mut effects: ResMut<DrivingEffects>) {
    effects.decals.clear();
    effects.particles.clear();
    effects.tick = 0;
}

fn advance_effects(shell: Res<ShellSimulation>, mut effects: ResMut<DrivingEffects>) {
    if effects.race_generation != shell.race_generation {
        effects.decals.clear();
        effects.particles.clear();
        effects.tick = 0;
        effects.race_generation = shell.race_generation;
    }
    if effects.generation != shell.generation {
        effects.decals.previous.clear();
        effects.particles.clear();
        effects.tick = 0;
        effects.generation = shell.generation;
    }
    if shell.sim.is_paused() || shell.sim.preparation_ticks() > 0 {
        return;
    }
    let samples: Vec<_> = shell
        .curr_snapshots
        .iter()
        .map(|car| DriftSample {
            pose: car.pose,
            heading: car.heading,
            drifting: car.drifting,
        })
        .collect();
    effects.decals.step(&samples);
    step_particles(&shell.curr_snapshots, &mut effects);
}

fn render_decals(
    effects: Res<DrivingEffects>,
    mut decals: Query<(&DecalSlot, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    for (slot, mut sprite, mut transform, mut visibility) in &mut decals {
        if let Some(mark) = effects.decals.marks.get(slot.0) {
            if *visibility != Visibility::Visible {
                *visibility = Visibility::Visible;
            }
            sprite.custom_size = Some(Vec2::new(mark.length + 0.125, 0.25));
            *transform = Transform::from_xyz(mark.center.x, mark.center.y, world_z::SKID)
                .with_rotation(Quat::from_rotation_z(mark.heading));
        } else if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
    }
}

/// Shared airborne budget for all four Cars, including player streaks.
pub const PARTICLE_BUDGET: usize = 128;

#[derive(Clone, Copy)]
enum ParticleKind {
    Smoke(usize),
    Dust,
    Streak,
}

struct Particle {
    pose: Vec2,
    heading: f32,
    kind: ParticleKind,
    age: u32,
}

#[derive(Component)]
struct ParticleSlot(usize);

#[derive(Resource)]
struct ParticleTextures {
    smoke: Handle<Image>,
    dust: Handle<Image>,
}

fn setup_particles(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(ParticleTextures {
        smoke: assets.load("sprites/fx/smoke.png"),
        dust: assets.load("sprites/fx/dust.png"),
    });
    for i in 0..PARTICLE_BUDGET {
        commands.spawn((
            SpriteBundle {
                visibility: Visibility::Hidden,
                ..default()
            },
            ParticleSlot(i),
        ));
    }
}

fn step_particles(
    cars: &[topdown_racer_core::simulation::CarSnapshot],
    effects: &mut DrivingEffects,
) {
    effects.tick = effects.tick.wrapping_add(1);
    for particle in &mut effects.particles {
        particle.age += 1;
    }
    effects.particles.retain(|p| match p.kind {
        // Smoke must disappear on release, even between emission ticks.
        ParticleKind::Smoke(car_index) => {
            p.age < 24
                && cars.get(car_index).is_some_and(|car| {
                    car.handbrake
                        && car.phase == topdown_racer_core::simulation::RacePhase::Racing
                        && car.finish_status == topdown_racer_core::simulation::FinishStatus::Racing
                })
        }
        ParticleKind::Streak => p.age < 8,
        ParticleKind::Dust => p.age < 24,
    });
    if !effects.tick.is_multiple_of(4) {
        return;
    }
    for (i, car) in cars.iter().enumerate() {
        if car.phase != topdown_racer_core::simulation::RacePhase::Racing
            || car.finish_status != topdown_racer_core::simulation::FinishStatus::Racing
        {
            continue;
        }
        let forward = Vec2::from_angle(car.heading);
        let left = Vec2::new(-forward.y, forward.x);
        // Handbrake is the smoke trigger; Drift and deceleration are not proxies.
        if car.handbrake {
            for side in [-1.0, 1.0] {
                effects.particles.push_back(Particle {
                    pose: car.pose - forward * 1.5 + left * side,
                    heading: car.heading,
                    kind: ParticleKind::Smoke(i),
                    age: 0,
                });
            }
        }
        if car.surface != topdown_racer_core::track::Surface::Road && car.velocity.length() > 2.0 {
            effects.particles.push_back(Particle {
                pose: car.pose - forward * 1.6,
                heading: car.heading,
                kind: ParticleKind::Dust,
                age: 0,
            });
        }
        if i == 0 && car.velocity.length() > 26.0 {
            let heading = car.velocity.y.atan2(car.velocity.x);
            for side in [-1.0, 1.0] {
                effects.particles.push_back(Particle {
                    pose: car.pose - forward * 3.0 + left * side * 3.0,
                    heading,
                    kind: ParticleKind::Streak,
                    age: 0,
                });
            }
        }
    }
    while effects.particles.len() > PARTICLE_BUDGET {
        effects.particles.pop_front();
    }
}

fn render_particles(
    effects: Res<DrivingEffects>,
    textures: Res<ParticleTextures>,
    mut slots: Query<(
        &ParticleSlot,
        &mut Sprite,
        &mut Handle<Image>,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    for (slot, mut sprite, mut texture, mut transform, mut visibility) in &mut slots {
        let Some(particle) = effects.particles.get(slot.0) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        *visibility = Visibility::Visible;
        let lifetime = match particle.kind {
            ParticleKind::Streak => 8.0,
            _ => 24.0,
        };
        let alpha = 1.0 - particle.age as f32 / lifetime;
        match particle.kind {
            ParticleKind::Smoke(_) | ParticleKind::Dust => {
                *texture = match particle.kind {
                    ParticleKind::Smoke(_) => textures.smoke.clone(),
                    _ => textures.dust.clone(),
                };
                sprite.custom_size = Some(Vec2::splat(2.0));
                // Keep the road edge and a following rival readable even when
                // smoke and dust overlap. Trigger cadence/lifetimes are unchanged.
                sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha * 0.45);
            }
            ParticleKind::Streak => {
                *texture = Handle::default();
                sprite.custom_size = Some(Vec2::new(2.0, 0.125));
                sprite.color =
                    crate::palette::color(crate::palette::CREAM_HIGHLIGHT).with_alpha(alpha * 0.45);
            }
        }
        *transform = Transform::from_xyz(particle.pose.x, particle.pose.y, world_z::FX)
            .with_rotation(Quat::from_rotation_z(particle.heading));
    }
}

#[derive(Component)]
pub(crate) struct BrakeLight(pub usize);

fn render_brake_lights(
    shell: Res<ShellSimulation>,
    mut lights: Query<(&BrakeLight, &mut Visibility)>,
) {
    for (light, mut visibility) in &mut lights {
        *visibility = if shell
            .curr_snapshots
            .get(light.0)
            .is_some_and(|car| car.brake > 0.0)
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod skid_lifecycle_tests {
    use super::*;
    use topdown_racer_core::{simulation::Sim, track::Track};
    #[test]
    fn pause_and_preparation_preserve_visible_particles_until_active_ticks_resume() {
        let track = Track::parse(topdown_racer_core::track::HILLSIDE_CIRCUIT).unwrap();
        let mut app = App::new();
        app.insert_resource(ShellSimulation::from_sim(Sim::new(track, 1)))
            .insert_resource(DrivingEffects {
                generation: 0,
                race_generation: 0,
                decals: SkidDecals::new(SKID_BUDGET),
                particles: VecDeque::from([Particle {
                    pose: Vec2::new(2.0, 3.0),
                    heading: 0.0,
                    kind: ParticleKind::Dust,
                    age: 23,
                }]),
                tick: 0,
            })
            .add_systems(Update, advance_effects);
        app.world_mut()
            .resource_mut::<ShellSimulation>()
            .sim
            .pause();
        for _ in 0..200 {
            app.update();
            let effects = app.world().resource::<DrivingEffects>();
            assert_eq!(effects.particles.len(), 1);
            assert_eq!(effects.particles[0].age, 23);
            assert_eq!(effects.particles[0].pose, Vec2::new(2.0, 3.0));
        }
        app.world_mut()
            .resource_mut::<ShellSimulation>()
            .sim
            .resume();
        for _ in 0..64 {
            app.update();
            assert_eq!(
                app.world().resource::<DrivingEffects>().particles[0].age,
                23
            );
            app.world_mut()
                .resource_mut::<ShellSimulation>()
                .sim
                .tick(&[]);
        }
        app.update();
        assert!(app
            .world()
            .resource::<DrivingEffects>()
            .particles
            .is_empty());
    }

    #[test]
    fn recovery_preserves_marks_but_restart_clears_them() {
        let track = Track::parse(topdown_racer_core::track::HILLSIDE_CIRCUIT).unwrap();
        let mut app = App::new();
        app.insert_resource(ShellSimulation::from_sim(Sim::new(track, 1)))
            .insert_resource(DrivingEffects {
                generation: 0,
                decals: SkidDecals::new(SKID_BUDGET),
                race_generation: 0,
                particles: VecDeque::new(),
                tick: 0,
            })
            .add_systems(Update, advance_effects);
        app.update();
        {
            let mut shell = app.world_mut().resource_mut::<ShellSimulation>();
            shell.curr_snapshots[0].pose.x += 1.0;
            shell.curr_snapshots[0].drifting = true;
        }
        app.update();
        let marks: Vec<_> = app
            .world()
            .resource::<DrivingEffects>()
            .decals
            .marks()
            .copied()
            .collect();
        assert_eq!(marks.len(), 2);
        {
            let mut shell = app.world_mut().resource_mut::<ShellSimulation>();
            shell.sim.pause();
            shell.sim.recover_player().unwrap();
            shell.curr_snapshots = shell.sim.snapshots();
            shell.prev_snapshots = shell.curr_snapshots.clone();
            shell.generation += 1;
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<DrivingEffects>()
                .decals
                .marks()
                .copied()
                .collect::<Vec<_>>(),
            marks
        );
        app.world_mut()
            .resource_mut::<ShellSimulation>()
            .reset_to_fresh_race();
        app.update();
        assert_eq!(
            app.world()
                .resource::<DrivingEffects>()
                .decals
                .marks()
                .count(),
            0
        );
    }

    /// Pins the shell render half of the seam: after a drift step, decal
    /// slots 0..marks must be Visible at world_z::SKID and the rest Hidden.
    #[test]
    fn render_decals_shows_marks_at_skid_z_and_hides_the_rest() {
        let mut app = App::new();
        app.insert_resource(ShellSimulation::from_sim(Sim::new(
            Track::parse(topdown_racer_core::track::HILLSIDE_CIRCUIT).unwrap(),
            1,
        )))
        .insert_resource(DrivingEffects {
            generation: 0,
            race_generation: 0,
            decals: SkidDecals::new(SKID_BUDGET),
            particles: VecDeque::new(),
            tick: 0,
        })
        .add_systems(Update, (advance_effects, render_decals).chain());
        for slot in 0..8u32 {
            app.world_mut().spawn((
                DecalSlot(slot as usize),
                SpriteBundle {
                    visibility: Visibility::Hidden,
                    ..default()
                },
            ));
        }
        app.update();
        {
            let mut shell = app.world_mut().resource_mut::<ShellSimulation>();
            shell.curr_snapshots[0].pose.x += 1.0;
            shell.curr_snapshots[0].drifting = true;
        }
        app.update();
        let mark_count = app
            .world()
            .resource::<DrivingEffects>()
            .decals
            .marks()
            .count();
        assert_eq!(mark_count, 2);
        let marks: Vec<_> = app
            .world()
            .resource::<DrivingEffects>()
            .decals
            .marks()
            .copied()
            .collect();
        let mut shown = 0;
        for (slot, sprite, transform, visibility) in app
            .world_mut()
            .query::<(&DecalSlot, &Sprite, &Transform, &Visibility)>()
            .iter(app.world())
        {
            match marks.get(slot.0) {
                Some(mark) => {
                    assert_eq!(
                        *visibility,
                        Visibility::Visible,
                        "slot {} must show",
                        slot.0
                    );
                    assert_eq!(transform.translation, mark.center.extend(world_z::SKID));
                    let (_, angle) = transform.rotation.to_axis_angle();
                    assert!((angle - mark.heading).abs() < 1e-5);
                    assert_eq!(
                        sprite.custom_size,
                        Some(Vec2::new(mark.length + 0.125, 0.25))
                    );
                    shown += 1;
                }
                None => {
                    assert_eq!(*visibility, Visibility::Hidden, "slot {} must hide", slot.0);
                }
            }
        }
        assert_eq!(shown, marks.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use topdown_racer_core::{
        simulation::{CarInput, FinishStatus, Sim},
        track::{Track, SAMPLE_CIRCUIT},
    };

    #[test]
    fn brake_lights_follow_each_cars_echo_not_handbrake_or_motion() {
        let mut app = App::new();
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let mut shell = ShellSimulation::new(track.clone());
        shell.sim = Sim::new(track, 4);
        app.insert_resource(shell)
            .add_systems(Update, render_brake_lights);
        let lights: Vec<_> = (0..4)
            .map(|car| {
                app.world_mut()
                    .spawn((BrakeLight(car), Visibility::Hidden))
                    .id()
            })
            .collect();
        for braking_car in 0..4 {
            let mut inputs = [CarInput {
                handbrake: true,
                ..default()
            }; 4];
            inputs[braking_car].brake = 0.01;
            let mut shell = app.world_mut().resource_mut::<ShellSimulation>();
            shell.curr_snapshots = shell.sim.tick(&inputs);
            app.update();
            for (car, &light) in lights.iter().enumerate() {
                assert_eq!(
                    *app.world().get::<Visibility>(light).unwrap(),
                    if car == braking_car {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    }
                );
            }
        }
        let mut shell = app.world_mut().resource_mut::<ShellSimulation>();
        shell.curr_snapshots = shell.sim.tick(&[CarInput::default(); 4]);
        app.update();
        for light in lights {
            assert_eq!(
                *app.world().get::<Visibility>(light).unwrap(),
                Visibility::Hidden
            );
        }
    }

    #[test]
    fn smoke_disappears_on_release_without_waiting_for_emission_cadence() {
        let mut sim = Sim::new(Track::parse(SAMPLE_CIRCUIT).unwrap(), 4);
        let mut effects = DrivingEffects {
            generation: 0,
            decals: SkidDecals::new(0),
            race_generation: 0,
            particles: VecDeque::new(),
            tick: 0,
        };
        let mut inputs = [CarInput {
            brake: 1.0,
            ..default()
        }; 4];
        for _ in 0..4 {
            step_particles(&sim.tick(&inputs), &mut effects);
        }
        assert!(effects.particles.is_empty(), "braking alone must not smoke");
        for input in &mut inputs {
            input.handbrake = true;
        }
        for _ in 0..4 {
            step_particles(&sim.tick(&inputs), &mut effects);
        }
        for car in 0..4 {
            assert_eq!(
                effects
                    .particles
                    .iter()
                    .filter(|p| matches!(p.kind, ParticleKind::Smoke(i) if i == car))
                    .count(),
                2
            );
        }
        inputs[0].handbrake = false;
        inputs[2].handbrake = false;
        step_particles(&sim.tick(&inputs), &mut effects);
        assert!(effects
            .particles
            .iter()
            .all(|p| matches!(p.kind, ParticleKind::Smoke(1 | 3))));
        assert_eq!(effects.particles.len(), 4, "other Cars keep their smoke");
        let mut snapshots = sim.tick(&inputs);
        snapshots[1].finish_status = FinishStatus::Finished { place: 1 };
        snapshots.truncate(2);
        step_particles(&snapshots, &mut effects);
        assert!(
            effects.particles.is_empty(),
            "finished and removed Cars leave no smoke"
        );
    }
}

#[cfg(test)]
mod dust_streak_tests {
    use super::*;
    use topdown_racer_core::{
        simulation::{CarSnapshot, GridCar, Sim},
        track::{Surface, Track, SAMPLE_CIRCUIT},
    };

    fn effects() -> DrivingEffects {
        DrivingEffects {
            generation: 0,
            decals: SkidDecals::new(0),
            race_generation: 0,
            particles: VecDeque::new(),
            tick: 0,
        }
    }

    fn snapshots(surface: Surface, speed: f32) -> Vec<CarSnapshot> {
        let mut track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        track.surfaces.fill(surface);
        Sim::from_grid(
            track,
            &[GridCar {
                pose: Vec2::new(20.0, 0.0),
                heading: 0.0,
                velocity: Vec2::new(speed, 0.0),
            }; 4],
        )
        .snapshots()
    }

    fn advance(cars: &[CarSnapshot], effects: &mut DrivingEffects, ticks: usize) {
        for _ in 0..ticks {
            step_particles(cars, effects);
        }
    }

    #[test]
    fn moving_cars_raise_dust_on_gravel_and_grass_but_not_road() {
        for surface in [Surface::Road, Surface::Gravel, Surface::Grass] {
            let cars = snapshots(surface, 10.0);
            assert!(cars.iter().all(|car| car.surface == surface));
            let mut effects = effects();
            advance(&cars, &mut effects, 4);
            assert_eq!(
                effects
                    .particles
                    .iter()
                    .filter(|p| matches!(p.kind, ParticleKind::Dust))
                    .count(),
                if surface == Surface::Road { 0 } else { 4 },
                "{surface:?}"
            );
            assert!(!effects
                .particles
                .iter()
                .any(|p| matches!(p.kind, ParticleKind::Streak)));
        }
        // Off-track grass comes from the Track query, not an authored Terrain Zone.
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let cars = Sim::from_grid(
            track,
            &[GridCar {
                pose: Vec2::new(20.0, 8.0),
                heading: 0.0,
                velocity: Vec2::new(10.0, 0.0),
            }],
        )
        .snapshots();
        assert_eq!(cars[0].surface, Surface::Grass);
        let mut effects = effects();
        advance(&cars, &mut effects, 4);
        assert!(effects
            .particles
            .iter()
            .any(|p| matches!(p.kind, ParticleKind::Dust)));
    }

    #[test]
    fn only_the_player_leaves_streaks_above_the_speed_threshold() {
        let mut cars = snapshots(Surface::Road, 30.0);
        for (i, car) in cars.iter_mut().enumerate() {
            car.pose += Vec2::new(0.0, i as f32 * 20.0);
        }
        let mut effects = effects();
        advance(&cars, &mut effects, 4);
        assert_eq!(effects.particles.len(), 2);
        assert!(
            effects
                .particles
                .iter()
                .all(|p| matches!(p.kind, ParticleKind::Streak)
                    && p.pose.distance(cars[0].pose) < 5.0)
        );

        // Fast rivals cannot trigger player streaks at or below the cutoff.
        for speed in [0.0, 10.0, 26.0] {
            cars[0].velocity = Vec2::new(speed, 0.0);
            effects.particles.clear();
            advance(&cars, &mut effects, 4);
            assert!(effects.particles.is_empty(), "player speed {speed}");
        }
    }

    #[test]
    fn stopping_emission_expires_dust_and_streaks() {
        let mut cars = snapshots(Surface::Gravel, 30.0);
        let mut effects = effects();
        advance(&cars, &mut effects, 4);
        assert!(effects
            .particles
            .iter()
            .any(|p| matches!(p.kind, ParticleKind::Dust)));
        assert!(effects
            .particles
            .iter()
            .any(|p| matches!(p.kind, ParticleKind::Streak)));
        for car in &mut cars {
            car.velocity = Vec2::ZERO;
        }
        advance(&cars, &mut effects, 8);
        assert!(!effects
            .particles
            .iter()
            .any(|p| matches!(p.kind, ParticleKind::Streak)));
        assert!(effects
            .particles
            .iter()
            .any(|p| matches!(p.kind, ParticleKind::Dust)));
        advance(&cars, &mut effects, 16);
        assert!(effects.particles.is_empty());
    }
}
