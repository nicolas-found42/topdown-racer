//! Bounded, snapshot-driven load, tire, surface and impact feedback.
use bevy::prelude::*;
use topdown_racer_core::{simulation::CarSnapshot, track::Surface};
#[derive(Clone, Copy, Default)]
pub struct FeedbackFrame {
    pub engine_volume: f32,
    pub tire_volume: f32,
    pub surface_volume: f32,
    pub impact_volume: f32,
    /// True only on an armed contact onset; drives a single playback.
    pub impact_started: bool,
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
        let impact_started = contact && !self.contact && self.quiet_ticks >= 8;
        if impact_started {
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
            impact_started,
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
    pending_impact: Option<f32>,
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
struct ImpactAudio {
    gain: f32,
}
/// Small nose-local highlight, above the Car but never across the road.
#[derive(Component)]
pub(crate) struct ImpactFlash;
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
            settings: PlaybackSettings::ONCE.with_volume(bevy::audio::Volume::ZERO),
        },
        ImpactAudio { gain: 0.0 },
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
        feedback.pending_impact = None;
        feedback.mixer.envelope = 0.0;
        feedback.frame.impact_volume = 0.0;
        return;
    }
    if let Some(car) = shell.curr_snapshots.first() {
        feedback.frame = feedback.mixer.step(car);
        if feedback.frame.impact_started {
            feedback.pending_impact = Some(feedback.frame.impact_volume);
        }
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
    mut commands: Commands,
    lifecycle: (Res<crate::ShellSimulation>, Res<State<crate::AppState>>),
    mut feedback: ResMut<FeedbackState>,
    volume: Res<EffectsVolume>,
    surface: Query<&AudioSink, With<SurfaceAudio>>,
    mut impact: Query<(
        Entity,
        Option<&AudioSink>,
        &mut PlaybackSettings,
        &mut ImpactAudio,
    )>,
    mut flashes: Query<&mut Visibility, With<ImpactFlash>>,
) {
    let (shell, state) = lifecycle;
    let active = *state.get() == crate::AppState::Race
        && !shell.sim.is_paused()
        && shell.sim.preparation_ticks() == 0;
    let level = if active { volume.0 } else { 0.0 };
    for sink in &surface {
        sink.set_volume(feedback.frame.surface_volume * level);
    }
    let onset = feedback.pending_impact.take().filter(|_| active);
    for (entity, sink, mut settings, mut voice) in &mut impact {
        if !active {
            voice.gain = 0.0;
            if let Some(sink) = sink {
                sink.stop();
            }
        } else if let Some(gain) = onset {
            voice.gain = gain;
            settings.volume = bevy::audio::Volume::new(gain * level);
            if let Some(sink) = sink {
                sink.stop();
                commands.entity(entity).remove::<AudioSink>();
            }
        } else if let Some(sink) = sink {
            sink.set_volume(voice.gain * level);
        }
    }
    for mut visibility in &mut flashes {
        *visibility = if feedback.frame.impact_volume > 0.0 && active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppState, ShellSimulation};
    use topdown_racer_core::track::{Track, HILLSIDE_CIRCUIT};

    #[test]
    fn impact_is_car_local_consumed_once_and_cleared_on_pause_and_restart() {
        let mut app = App::new();
        app.insert_resource(ShellSimulation::new(
            Track::parse(HILLSIDE_CIRCUIT).unwrap(),
        ))
        .insert_resource(State::new(AppState::Race))
        .init_resource::<Assets<AudioSource>>()
        .init_resource::<FeedbackState>()
        .init_resource::<EffectsVolume>()
        .add_systems(Startup, (setup, crate::audio::setup_audio))
        .add_systems(Update, (advance, apply).chain());
        let flash = app
            .world_mut()
            .spawn((ImpactFlash, Visibility::Hidden))
            .id();
        app.update();
        let baseline = app.world().entities().len();
        for _ in 0..10 {
            {
                let mut shell = app.world_mut().resource_mut::<ShellSimulation>();
                shell.curr_snapshots[0].wall_contact = true;
                shell.curr_snapshots[0].velocity = Vec2::new(20.0, 0.0);
            }
            app.update();
            assert_eq!(
                *app.world().get::<Visibility>(flash).unwrap(),
                Visibility::Inherited
            );
            assert!(app
                .world()
                .resource::<FeedbackState>()
                .pending_impact
                .is_none());
            for _ in 0..20 {
                app.update();
            }
            assert_eq!(
                *app.world().get::<Visibility>(flash).unwrap(),
                Visibility::Hidden
            );
            app.world_mut()
                .resource_mut::<ShellSimulation>()
                .sim
                .pause();
            app.update();
            assert_eq!(
                *app.world().get::<Visibility>(flash).unwrap(),
                Visibility::Hidden
            );
            app.world_mut()
                .resource_mut::<ShellSimulation>()
                .reset_to_fresh_race();
            app.update();
            assert_eq!(
                app.world().resource::<FeedbackState>().frame.impact_volume,
                0.0
            );
            assert_eq!(app.world().entities().len(), baseline);
            assert_eq!(
                app.world_mut()
                    .query::<&Handle<AudioSource>>()
                    .iter(app.world())
                    .count(),
                4
            );
        }
    }
}
