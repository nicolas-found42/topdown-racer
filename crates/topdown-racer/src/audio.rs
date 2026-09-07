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

/// Pure function mapping forward speed to engine audio playback speed (pitch).
/// Idle speed (forward_speed <= 0.0) plays at 0.8x.
/// As forward speed increases to top speed (~28.0 u/s), pitch scales smoothly up to ~2.4x.
pub fn engine_pitch_from_speed(forward_speed: f32) -> f32 {
    let speed = forward_speed.abs();
    0.8 + (speed / 28.0) * 1.6
}

/// Pure function computing skid sound volume from drift state and speed.
/// Returns 0.0 when not drifting or stopped.
/// When drifting at speed, volume is non-zero (0.6).
pub fn skid_volume_from_drift(drifting: bool, forward_speed: f32) -> f32 {
    if drifting && forward_speed.abs() > 2.0 {
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
        skid_volume: skid_volume_from_drift(snap.drifting, snap.forward_speed),
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
