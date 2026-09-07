# FFmpeg Game-Window Frame Capture: Deep-Dive (Round 2)

## TL;DR & Refined Recommendation (What Changed vs Round 1)

Round 1 concluded that external `avfoundation` is screen-only (cannot target windows without fragile cropping) and `screencapture -l` cannot sustain 60 FPS, recommending in-game raw dumps.

**Round 2 Refinements:**
1. **Bevy 0.14.2 Internal Readback is the Winner:** Analysis of `bevy_render-0.14.2` (`src/view/window/screenshot.rs`) confirms `ScreenshotManager::take_screenshot` already handles `wgpu` staging copies and row-padding stripping (`COPY_BYTES_PER_ROW_ALIGNMENT = 256`). Pumping `Image.data` directly into an FFmpeg child stdin pipe requires **zero new dependencies**, eliminates all macOS TCC gates, and maintains deterministic 1:1 simulation frame sync.
2. **Metal Endianness Trap:** macOS Metal swapchains default to `TextureFormat::Bgra8UnormSrgb`. Feeding raw bytes to FFmpeg requires `-pixel_format bgra` (not `rgba`), or red/blue channels swap.
3. **ScreenCaptureKit is Unusable in CI:** While macOS 14+ `SCScreenshotManager` / `SCContentFilter` supports native window capture, Apple hard-gates it behind `kTCCServiceScreenCapture`. It fails on headless GitHub Actions runners.
4. **Rust Crates Fall Short:** `xcap` uses deprecated `CGWindowListCreateImage` (synchronous 20–30ms latency, TCC required); `screenshots`, `scrap`, and `captrs` lack window-targeting on macOS.

---

## 1. macOS ScreenCaptureKit (`SCStream` & `SCScreenshotManager`)

macOS 12.3+ introduced ScreenCaptureKit; macOS 14.0 added `SCScreenshotManager` and desktop-independent window filters (`SCContentFilter(desktopIndependentWindow:)`).

- **Rust Bindings:**
  - `screencapturekit` (v1.4.x, [github.com/doom-fish/screencapturekit-rs](https://github.com/doom-fish/screencapturekit-rs)): provides `SCScreenshotManager::capture_image(&filter, &config)` and `SCStream`.
  - `cidre` (v0.5.x, [github.com/yury/cidre](https://github.com/yury/cidre)): zero-cost Apple bindings used by Cap ([github.com/CapSoftware/Cap](https://github.com/CapSoftware/Cap), `crates/recording/src/lib.rs`).
- **FFmpeg Integration Pipeline:**
  Acquire frame buffer via `SCStreamOutput` delegate or `capture_image()`, lock `CVPixelBuffer` base address, and write raw bytes to FFmpeg stdin:
  ```bash
  ffmpeg -f rawvideo -pixel_format bgra -video_size 1280x720 -framerate 60 -i pipe:0 -c:v png frames/frame_%04d.png
  ```
- **Primary Sources:**
  - Apple Docs: [developer.apple.com/documentation/screencapturekit](https://developer.apple.com/documentation/screencapturekit)
  - Real-World Code: `QwenLM/qwen-code` (`packages/cua-driver/rust/crates/platform-macos/src/capture.rs`), `obsproject/obs-studio` (`plugins/mac-capture/mac-sck-video-capture.m`).

---

## 2. Rust Screen-Capture Crates

| Crate | Window Capture on macOS? | Underlying macOS API | 60 FPS Real-time? | Verdict |
| :--- | :--- | :--- | :--- | :--- |
| **`xcap`** (v0.0.14+) | Yes (`Window::all()`) | `CGWindowListCreateImage` | No (~30–50 FPS max) | Synchronous IPC bottleneck; TCC-gated |
| **`screenshots`** (v0.8+) | No (monitors only) | `CGDisplayCreateImage` | No | Display-level only (`Screen::all()`) |
| **`scrap`** (v0.5) | No | `CGDisplayStream` | No | Deprecated/unmaintained (2017) |
| **`captrs`** (v0.1) | No | CoreGraphics Display | No | Display-level only |

- **Primary Sources:** `nashaofu/xcap` (`src/macos/impl_window.rs`), `nashaofu/screenshots-rs` (`src/lib.rs`).

---

## 3. Bevy 0.14.2 Render-World Readback to FFmpeg Pipe

`bevy_render-0.14.2` implements swapchain readback in `src/view/window/screenshot.rs`:
1. **GPU Copy (`screenshot.rs:222`):** `encoder.copy_texture_to_buffer` copies the swapchain texture into a staging buffer aligned to `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT` (256 bytes).
2. **Padding Strip (`screenshot.rs:317`):** `collect_screenshots` maps the buffer and truncates row padding, producing tightly packed pixel bytes in `Image::data`.
3. **Application Hook:**
   ```rust
   // In Update system:
   screenshot_manager.take_screenshot(window_entity, move |image: Image| {
       // image.data is tightly packed unpadded BGRA bytes
       ffmpeg_stdin.write_all(&image.data).unwrap();
   }).unwrap();
   ```
- **Primary Sources:** `bevy_render-0.14.2/src/view/window/screenshot.rs`, `gfx-rs/wgpu` (`wgpu-types/src/lib.rs:118`).

---

## 4. FFmpeg `rawvideo` Demuxer & `image2` Details

- **Demuxer Options:** Raw video has no container header. Must specify:
  `-f rawvideo -pixel_format bgra -video_size 1280x720 -framerate 60 -i pipe:0`
  - *Pixel Format:* Metal produces `bgra` (little-endian ARGB). Using `rgba` swaps R and B.
  - *Orientation:* Metal readback is top-down; no `-vf vflip` needed.
- **`image2` Muxer Patterns:**
  - Multi-frame sequence: `ffmpeg -f rawvideo ... -i pipe:0 -f image2 -c:v png frames/frame_%04d.png`
  - Atomic single-frame overwrite (`-update 1`):
    `ffmpeg -f rawvideo ... -i pipe:0 -f image2 -update 1 -atomic_writing 1 latest.png`
- **Primary Sources:** FFmpeg Formats Docs ([ffmpeg.org/ffmpeg-formats.html#image2-1](https://ffmpeg.org/ffmpeg-formats.html#image2-1), [ffmpeg.org/ffmpeg-formats.html#rawvideo](https://ffmpeg.org/ffmpeg-formats.html#rawvideo)).

---

## 5. macOS TCC Permissions: External vs In-Engine

- **`kTCCServiceScreenCapture` Requirement:**
  - `ScreenCaptureKit`: Without permission, throws `SCStreamErrorDomain code -3801 (userDeclined)`.
  - `CGWindowListCreateImage`: Fails silently, returning only desktop wallpaper and window frame shadows without app contents.
  - Headless CI: GitHub Actions macOS runners lack an interactive GUI to grant TCC, failing external captures unconditionally.
- **In-Engine Exemption:** Reading GPU swapchain buffers in-process via Metal/wgpu requires **zero OS permissions** and runs fully headlessly.

---

## 6. Comparison of New Approaches

| Criterion | Bevy `ScreenshotManager` + Pipe | ScreenCaptureKit (`cidre`/`screencapturekit`) | `xcap` (`CGWindowListCreateImage`) |
| :--- | :--- | :--- | :--- |
| **Window Isolation** | Exact viewport framebuffer | Window-only (OS compositor) | Window-only (with window chrome) |
| **Frame Sync** | Exact 1:1 sim tick | Asynchronous stream | Asynchronous polling |
| **macOS Permissions** | None (in-process GPU read) | Required (`kTCCServiceScreenCapture`) | Required (`kTCCServiceScreenCapture`) |
| **Headless CI Support**| Yes (software Metal/Vulkan) | Fails (no TCC prompt) | Fails (returns wallpaper only) |
| **Throughput** | Uncapped 60+ FPS | 60 FPS hardware accelerated | 20–40 FPS (sync bottleneck) |
