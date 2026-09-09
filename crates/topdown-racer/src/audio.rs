//! Procedural audio: engine/skid loop generation, playback systems, and
//! mute/unmute state transitions.

use topdown_racer_core::simulation::TOP_SPEED;

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

/// Pure function mapping forward speed to engine audio playback speed (pitch).
/// Idle speed (forward_speed <= 0.0) plays at 0.8x.
/// As forward speed increases to core's TOP_SPEED, pitch scales smoothly up to ~2.4x.
pub fn engine_pitch_from_speed(forward_speed: f32) -> f32 {
    let speed = forward_speed.abs();
    0.8 + (speed / TOP_SPEED) * 1.6
}

/// Pure function computing skid sound volume from drift state.
/// Returns 0.0 when not drifting; 0.6 while drifting.
/// Speed eligibility is the simulation's own Drift contract (DRIFT_SPEED_MIN),
/// so no shell-side speed gate is re-encoded here.
pub fn skid_volume_from_drift(drifting: bool) -> f32 {
    if drifting {
        0.6
    } else {
        0.0
    }
}

/// Projected audio cues derived from a car snapshot.
pub(crate) struct AudioCue {
    pub pitch: f32,
    pub skid_volume: f32,
}

/// Projects a car snapshot into engine pitch and skid volume cues.
pub(crate) fn audio_cue_from_snapshot(snap: &CarSnapshot) -> AudioCue {
    AudioCue {
        pitch: engine_pitch_from_speed(snap.forward_speed),
        skid_volume: skid_volume_from_drift(snap.drifting),
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
    engine_q: Query<&AudioSink, With<EngineAudio>>,
    skid_q: Query<&AudioSink, With<SkidAudio>>,
) {
    let Some(player_snap) = shell.curr_snapshots.first() else {
        return;
    };

    let cue = audio_cue_from_snapshot(player_snap);

    for sink in engine_q.iter() {
        sink.set_speed(cue.pitch);
    }
    for sink in skid_q.iter() {
        sink.set_volume(cue.skid_volume);
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
    fn engine_pitch_strictly_increases_with_forward_speed() {
        let p0 = engine_pitch_from_speed(0.0);
        let p10 = engine_pitch_from_speed(10.0);
        let p20 = engine_pitch_from_speed(20.0);
        let p30 = engine_pitch_from_speed(30.0);

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
    fn skid_volume_is_active_during_drift_and_silent_when_grip_recovers() {
        // Grip recovers (drifting becomes false): silent
        assert_eq!(skid_volume_from_drift(false), 0.0);

        // Drifting: audible skid cue
        let vol_drift = skid_volume_from_drift(true);
        assert!(
            vol_drift > 0.5,
            "skid volume must be audible while drifting: got {vol_drift}"
        );
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
