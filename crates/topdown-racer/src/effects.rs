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
    if effects.generation != shell.generation {
        effects.decals.clear();
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
            *visibility = Visibility::Visible;
            sprite.custom_size = Some(Vec2::new(mark.length + 0.125, 0.25));
            *transform = Transform::from_xyz(mark.center.x, mark.center.y, world_z::SKID)
                .with_rotation(Quat::from_rotation_z(mark.heading));
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

/// Shared airborne budget for all four Cars, including player streaks.
pub const PARTICLE_BUDGET: usize = 128;

#[derive(Clone, Copy)]
enum ParticleKind {
    Smoke,
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
    effects.particles.retain(|p| {
        p.age
            < match p.kind {
                ParticleKind::Streak => 8,
                _ => 24,
            }
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
                    kind: ParticleKind::Smoke,
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
            ParticleKind::Smoke | ParticleKind::Dust => {
                *texture = match particle.kind {
                    ParticleKind::Smoke => textures.smoke.clone(),
                    _ => textures.dust.clone(),
                };
                sprite.custom_size = Some(Vec2::splat(2.0));
                sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
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
