//! Frame capture: pipes game window screenshots to FFmpeg rawvideo on stdin
//! for pixel-perfect frame dumps, activated by TOPDOWN_CAPTURE=1.

use bevy::{
    prelude::*, render::view::window::screenshot::ScreenshotManager, window::PrimaryWindow,
};

/// Frame capture configuration for saving game window screenshots to disk via FFmpeg.
#[derive(Resource)]
pub struct FrameCapture {
    pub output_dir: std::path::PathBuf,
    pub max_frames: u32,
    pub frame_count: u32,
    pub active: bool,
    pub sender: Option<std::sync::mpsc::Sender<Vec<u8>>>,
}

impl Default for FrameCapture {
    fn default() -> Self {
        let max_frames = std::env::var("TOPDOWN_CAPTURE_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600);
        Self {
            output_dir: std::path::PathBuf::from("/tmp/topdown_frames"),
            max_frames,
            frame_count: 0,
            active: false,
            sender: None,
        }
    }
}

/// Captures game window frames and pipes them to FFmpeg rawvideo on stdin,
/// producing pixel-perfect PNG screenshots of the game window.
pub fn frame_capture_system(
    mut capture: ResMut<FrameCapture>,
    mut screenshot_manager: ResMut<ScreenshotManager>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
) {
    if !capture.active {
        return;
    }
    if capture.frame_count >= capture.max_frames {
        capture.active = false;
        capture.sender = None;
        info!(
            "frame capture complete: {} frames piped to ffmpeg in {}",
            capture.frame_count,
            capture.output_dir.display()
        );
        return;
    }

    let Ok((window_entity, window)) = windows.get_single() else {
        return;
    };

    if std::fs::create_dir_all(&capture.output_dir).is_err() {
        capture.active = false;
        return;
    }

    // Lazy initialization of the FFmpeg rawvideo child process
    if capture.sender.is_none() {
        let width = window.physical_width();
        let height = window.physical_height();
        let out_pattern = capture.output_dir.join("frame_%04d.png");
        let out_str = out_pattern.to_string_lossy().to_string();

        let child = std::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "rawvideo",
                "-pixel_format",
                "bgra",
                "-video_size",
                &format!("{width}x{height}"),
                "-framerate",
                "60",
                "-i",
                "pipe:0",
                "-c:v",
                "png",
                &out_str,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        match child {
            Ok(mut proc) => {
                let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
                std::thread::spawn(move || {
                    use std::io::Write;
                    if let Some(mut stdin) = proc.stdin.take() {
                        while let Ok(frame_data) = rx.recv() {
                            if stdin.write_all(&frame_data).is_err() {
                                break;
                            }
                        }
                    }
                    let _ = proc.wait();
                });
                capture.sender = Some(tx);
            }
            Err(e) => {
                warn!("failed to spawn ffmpeg: {e}; falling back to direct save");
            }
        }
    }

    capture.frame_count += 1;

    if let Some(tx) = capture.sender.as_ref().cloned() {
        if screenshot_manager
            .take_screenshot(window_entity, move |image| {
                let _ = tx.send(image.data);
            })
            .is_err()
        {
            capture.frame_count -= 1;
        }
    } else {
        let path = capture
            .output_dir
            .join(format!("frame_{:04}.png", capture.frame_count));
        if screenshot_manager
            .save_screenshot_to_disk(window_entity, &path)
            .is_err()
        {
            capture.frame_count -= 1;
        }
    }
}
