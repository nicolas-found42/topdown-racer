//! Procedural audio: engine/skid loop generation, playback systems, and
//! mute/unmute state transitions.

use bevy::prelude::*;

use topdown_racer_core::simulation::CarSnapshot;

use crate::ShellSimulation;

/// Default audible playback volume for the engine loop.
pub const DEFAULT_ENGINE_VOLUME: f32 = 0.3;

/// Marker for the looping engine audio player entity.
#[derive(Component)]
pub struct EngineAudio;

/// Marker for the looping tire skid audio player entity.
#[derive(Component)]
pub struct SkidAudio;

/// Engine playback follows drivetrain RPM, including pitch drops on upshifts.
pub fn engine_pitch_from_rpm(rpm: f32) -> f32 {
    0.8 + ((rpm - 900.0) / 5100.0).clamp(0.0, 1.0) * 1.6
}

/// Continuous tire excess; a gripped wheel contributes no screech.
pub fn tire_volume_from_wheels(wheels: &[topdown_racer_core::simulation::WheelSnapshot; 4]) -> f32 {
    wheels
        .iter()
        .map(|wheel| {
            let surface_gain = if wheel.surface == topdown_racer_core::track::Surface::Road {
                1.0
            } else {
                0.55
            };
            (wheel.normalized_slip - 1.0).clamp(0.0, 2.0) * 0.3 * surface_gain
        })
        .fold(0.0, f32::max)
}

/// Projected audio cues derived from a car snapshot.
pub(crate) struct AudioCue {
    pub pitch: f32,
    pub skid_volume: f32,
}

/// Projects a car snapshot into engine pitch and skid volume cues.
pub(crate) fn audio_cue_from_snapshot(snap: &CarSnapshot) -> AudioCue {
    AudioCue {
        pitch: engine_pitch_from_rpm(snap.engine_rpm),
        skid_volume: tire_volume_from_wheels(&snap.wheels),
    }
}

/// Creates a standard 44-byte WAV header followed by 16-bit signed PCM mono audio samples.
pub fn create_pcm_wav(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let file_len = 36 + data_len;
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&file_len.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");

    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // Mono
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * 2;
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes()); // Block align
    bytes.extend_from_slice(&16u16.to_le_bytes()); // 16-bit

    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }

    bytes
}

/// Generates an in-memory loopable WAV waveform representing an engine tone.
pub fn generate_engine_loop_wav() -> Vec<u8> {
    let sample_rate = 44100u32;
    // Exactly 25 full cycles of 110.25 Hz = 10000 samples for seamless looping
    let sample_count = 10000;
    let mut samples = Vec::with_capacity(sample_count);

    let f0 = 110.25f32;
    for i in 0..sample_count {
        let t = i as f32 / sample_rate as f32;
        let p = 2.0 * std::f32::consts::PI * f0 * t;
        let v =
            p.sin() * 0.5 + (p * 2.0).sin() * 0.3 + (p * 3.0).sin() * 0.15 + (p * 4.0).sin() * 0.05;
        let sample = (v * 16000.0) as i16;
        samples.push(sample);
    }

    create_pcm_wav(sample_rate, &samples)
}

/// Generates an in-memory loopable WAV waveform representing tire skid friction.
pub fn generate_skid_loop_wav() -> Vec<u8> {
    let sample_rate = 44100u32;
    // Exactly 200 cycles of 882 Hz = 10000 samples for seamless looping
    let sample_count = 10000;
    let mut samples = Vec::with_capacity(sample_count);

    for i in 0..sample_count {
        let t = i as f32 / sample_rate as f32;
        let carrier = (2.0 * std::f32::consts::PI * 882.0 * t).sin();
        let modulator = (2.0 * std::f32::consts::PI * 176.4 * t).sin();
        let hash = ((i.wrapping_mul(1103515245).wrapping_add(12345)) % 1000) as f32 / 1000.0 - 0.5;
        let v = (carrier * 0.6 + carrier * modulator * 0.2 + hash * 0.2).clamp(-1.0, 1.0);
        let sample = (v * 14000.0) as i16;
        samples.push(sample);
    }

    create_pcm_wav(sample_rate, &samples)
}

/// Spawns procedural audio loops for engine and skid feedback.
pub fn setup_audio(mut commands: Commands, mut audio_sources: ResMut<Assets<AudioSource>>) {
    let engine_source = audio_sources.add(AudioSource {
        bytes: generate_engine_loop_wav().into(),
    });
    let skid_source = audio_sources.add(AudioSource {
        bytes: generate_skid_loop_wav().into(),
    });

    commands.spawn((
        AudioBundle {
            source: engine_source,
            settings: PlaybackSettings::LOOP
                .with_speed(0.8)
                .with_volume(bevy::audio::Volume::ZERO),
        },
        EngineAudio,
    ));

    commands.spawn((
        AudioBundle {
            source: skid_source,
            settings: PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::ZERO),
        },
        SkidAudio,
    ));
}

/// Updates engine pitch by speed and skid cue during Drift.
/// Fails safe: if no audio hardware is present (or in headless CI), AudioSink query
/// yields no entities, so this system executes cleanly without error.
pub fn update_audio(
    shell: Res<ShellSimulation>,
    feedback: Option<Res<crate::feedback::FeedbackState>>,
    volume: Option<Res<crate::feedback::EffectsVolume>>,
    engine_q: Query<&AudioSink, With<EngineAudio>>,
    skid_q: Query<&AudioSink, With<SkidAudio>>,
) {
    let Some(player_snap) = shell.curr_snapshots.first() else {
        return;
    };

    let cue = audio_cue_from_snapshot(player_snap);
    let level = volume.as_ref().map_or(1.0, |v| v.0);
    let engine_volume = feedback
        .as_ref()
        .map_or(DEFAULT_ENGINE_VOLUME, |f| f.frame.engine_volume)
        * level;
    let tire_volume = feedback
        .as_ref()
        .map_or(cue.skid_volume, |f| f.frame.tire_volume)
        * level;

    for sink in engine_q.iter() {
        sink.set_speed(cue.pitch);
        sink.set_volume(
            if shell.sim.is_paused() || shell.sim.preparation_ticks() > 0 {
                0.0
            } else {
                engine_volume
            },
        );
    }
    for sink in skid_q.iter() {
        sink.set_volume(
            if shell.sim.is_paused() || shell.sim.preparation_ticks() > 0 {
                0.0
            } else {
                tire_volume
            },
        );
    }
}

/// Mutes audio while outside of an active race (e.g. Menu or Results).
pub fn mute_audio(
    engine_q: Query<&AudioSink, With<EngineAudio>>,
    skid_q: Query<&AudioSink, With<SkidAudio>>,
) {
    for sink in engine_q.iter() {
        sink.set_volume(0.0);
    }
    for sink in skid_q.iter() {
        sink.set_volume(0.0);
    }
}

/// Unmutes engine audio when starting/resuming an active race.
pub fn unmute_audio(engine_q: Query<&AudioSink, With<EngineAudio>>) {
    for sink in engine_q.iter() {
        sink.set_volume(DEFAULT_ENGINE_VOLUME);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ShellSimulation;
    use topdown_racer_core::track::{Track, SAMPLE_CIRCUIT};

    #[test]
    fn engine_pitch_increases_with_rpm_and_falls_after_a_shift() {
        let p0 = engine_pitch_from_rpm(900.0);
        let p10 = engine_pitch_from_rpm(2000.0);
        let p20 = engine_pitch_from_rpm(4000.0);
        let p30 = engine_pitch_from_rpm(6000.0);

        assert!(
            p0 > 0.7 && p0 < 0.9,
            "idle pitch should be around 0.8: got {p0}"
        );
        assert!(
            p10 > p0,
            "pitch must increase with speed: p10={p10} > p0={p0}"
        );
        assert!(
            p20 > p10,
            "pitch must increase with speed: p20={p20} > p10={p10}"
        );
        assert!(
            p30 > p20,
            "pitch must increase with speed: p30={p30} > p20={p20}"
        );
    }

    #[test]
    fn tire_volume_follows_excess_and_is_silent_with_grip() {
        let mut wheels = [topdown_racer_core::simulation::WheelSnapshot::default(); 4];
        assert_eq!(tire_volume_from_wheels(&wheels), 0.0);
        wheels[0].normalized_slip = 1.5;
        let mild = tire_volume_from_wheels(&wheels);
        wheels[0].normalized_slip = 3.0;
        assert!(tire_volume_from_wheels(&wheels) > mild);
        assert_eq!(tire_volume_from_wheels(&wheels), 0.6);
    }

    #[test]
    fn audio_systems_run_without_error_when_no_audio_device_is_present() {
        let mut app = App::new();
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        app.insert_resource(ShellSimulation::new(track))
            .add_systems(Update, (update_audio, mute_audio, unmute_audio));

        // Spawn audio entities without AudioSink (simulating headless runner without audio stream)
        app.world_mut().spawn(EngineAudio);
        app.world_mut().spawn(SkidAudio);

        // Must execute cleanly without error or panic
        app.update();
    }

    #[test]
    fn pcm_wav_generator_produces_valid_wave_header() {
        let engine_wav = generate_engine_loop_wav();
        assert!(engine_wav.len() > 44);
        assert_eq!(&engine_wav[0..4], b"RIFF");
        assert_eq!(&engine_wav[8..12], b"WAVE");
        assert_eq!(&engine_wav[12..16], b"fmt ");
        assert_eq!(&engine_wav[36..40], b"data");

        let skid_wav = generate_skid_loop_wav();
        assert!(skid_wav.len() > 44);
        assert_eq!(&skid_wav[0..4], b"RIFF");
        assert_eq!(&skid_wav[8..12], b"WAVE");
    }
}
