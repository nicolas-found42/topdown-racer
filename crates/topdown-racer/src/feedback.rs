//! Bounded, snapshot-driven load, tire, surface and impact feedback.
use bevy::prelude::*;
use topdown_racer_core::{simulation::CarSnapshot, track::Surface};
#[derive(Clone, Copy, Default)]
pub struct FeedbackFrame {
    pub engine_volume: f32,
    pub tire_volume: f32,
    pub surface_volume: f32,
    pub impact_volume: f32,
}
pub struct FeedbackMixer {
    contact: bool,
    quiet_ticks: u32,
    envelope: f32,
    previous_speed: f32,
    tire: f32,
}
impl Default for FeedbackMixer {
    fn default() -> Self {
        Self {
            contact: false,
            quiet_ticks: 8,
            envelope: 0.0,
            previous_speed: 0.0,
            tire: 0.0,
        }
    }
}
impl FeedbackMixer {
    pub fn step(&mut self, car: &CarSnapshot) -> FeedbackFrame {
        let speed = car.velocity.length();
        let contact = car.wall_contact || car.car_contact;
        if !contact {
            self.quiet_ticks = self.quiet_ticks.saturating_add(1);
        }
        if contact && !self.contact && (self.quiet_ticks >= 8 || self.previous_speed == 0.0) {
            let severity = car
                .car_contact_speed
                .max((self.previous_speed - speed).abs());
            self.envelope = (severity / 20.0).clamp(0.1, 0.8);
        }
        if contact {
            self.quiet_ticks = 0;
        }
        self.contact = contact;
        self.previous_speed = speed;
        let left = Vec2::new(-car.heading.sin(), car.heading.cos());
        let slip = car.velocity.dot(left).abs();
        let tire_target = if car.drifting {
            (slip / 10.0).clamp(0.1, 0.6)
        } else {
            0.0
        };
        self.tire += (tire_target - self.tire) * 0.2;
        let result = FeedbackFrame {
            engine_volume: 0.12 + car.throttle.clamp(0.0, 1.0) * 0.18,
            tire_volume: self.tire,
            surface_volume: if car.surface == Surface::Road {
                0.0
            } else {
                (speed / 40.0).min(0.4)
            },
            impact_volume: self.envelope,
        };
        self.envelope = (self.envelope - 0.06).max(0.0);
        result
    }
}
#[derive(Resource, Default)]
pub struct FeedbackState {
    mixer: FeedbackMixer,
    pub frame: FeedbackFrame,
    generation: u64,
}
#[derive(Resource)]
pub struct EffectsVolume(pub f32);
impl Default for EffectsVolume {
    fn default() -> Self {
        Self(1.0)
    }
}
impl EffectsVolume {
    pub fn label(&self) -> &'static str {
        if self.0 == 0.0 {
            "MUTED"
        } else if self.0 < 1.0 {
            "50%"
        } else {
            "100%"
        }
    }
}
#[derive(Component)]
struct SurfaceAudio;
#[derive(Component)]
struct ImpactAudio;
pub(crate) struct FeedbackPlugin;
impl Plugin for FeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FeedbackState>()
            .init_resource::<EffectsVolume>()
            .add_systems(Startup, setup)
            .add_systems(
                FixedUpdate,
                advance
                    .after(crate::step_simulation)
                    .run_if(in_state(crate::AppState::Race)),
            )
            .add_systems(Update, (volume_control, apply).chain());
    }
}
fn setup(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let rough = sources.add(AudioSource {
        bytes: crate::generate_skid_loop_wav().into(),
    });
    let samples: Vec<i16> = (0..4410)
        .map(|i| {
            let t = i as f32 / 44100.0;
            ((t * std::f32::consts::TAU * 72.0).sin() * (-t * 32.0).exp() * 24000.0) as i16
        })
        .collect();
    let impact = sources.add(AudioSource {
        bytes: crate::create_pcm_wav(44100, &samples).into(),
    });
    commands.spawn((
        AudioBundle {
            source: rough,
            settings: PlaybackSettings::LOOP
                .with_speed(0.45)
                .with_volume(bevy::audio::Volume::ZERO),
        },
        SurfaceAudio,
    ));
    commands.spawn((
        AudioBundle {
            source: impact,
            settings: PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::ZERO),
        },
        ImpactAudio,
    ));
}
fn advance(shell: Res<crate::ShellSimulation>, mut feedback: ResMut<FeedbackState>) {
    if feedback.generation != shell.generation {
        *feedback = FeedbackState {
            generation: shell.generation,
            ..default()
        };
    }
    if shell.sim.is_paused() || shell.sim.preparation_ticks() > 0 {
        return;
    }
    if let Some(car) = shell.curr_snapshots.first() {
        feedback.frame = feedback.mixer.step(car);
    }
}
fn volume_control(keys: Res<ButtonInput<KeyCode>>, mut volume: ResMut<EffectsVolume>) {
    if keys.just_pressed(KeyCode::KeyB) {
        volume.0 = if volume.0 == 1.0 {
            0.5
        } else if volume.0 == 0.5 {
            0.0
        } else {
            1.0
        };
    }
}
fn apply(
    shell: Res<crate::ShellSimulation>,
    state: Res<State<crate::AppState>>,
    feedback: Res<FeedbackState>,
    volume: Res<EffectsVolume>,
    surface: Query<&AudioSink, With<SurfaceAudio>>,
    impact: Query<&AudioSink, With<ImpactAudio>>,
    mut cars: Query<(&crate::CarSprite, &mut Sprite)>,
) {
    let active = *state.get() == crate::AppState::Race
        && !shell.sim.is_paused()
        && shell.sim.preparation_ticks() == 0;
    let level = if active { volume.0 } else { 0.0 };
    for sink in &surface {
        sink.set_volume(feedback.frame.surface_volume * level);
    }
    for sink in &impact {
        sink.set_volume(feedback.frame.impact_volume * level);
    }
    for (car, mut sprite) in &mut cars {
        sprite.color = if car.car_index == 0
            && feedback.frame.impact_volume > 0.0
            && *state.get() == crate::AppState::Race
        {
            crate::palette::color(crate::palette::RIVAL_ORANGE)
        } else {
            Color::WHITE
        };
    }
}
