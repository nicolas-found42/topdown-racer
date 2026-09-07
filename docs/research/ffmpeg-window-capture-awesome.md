# FFmpeg Game-Window Frame Capture: macOS & Cross-Platform

## TL;DR & Recommendation

**Recommendation: In-Game Frame Dump Piped to FFmpeg (Rawvideo / PNG).**
For a Bevy 0.14.2 (Metal) game on macOS requiring per-frame PNGs of *only* the game window:
1. **Best Pipeline:** Have the Bevy app copy its render texture / swapchain buffer to stdout/pipe as raw RGBA frames (`pipe:0`), and consume with `ffmpeg -f rawvideo -pixel_format rgba -video_size WxH -framerate 60 -i pipe:0 -c:v png frame_%04d.png` (or save PNGs via Bevy `ScreenshotManager` and run FFmpeg for post-processing/filtering).
2. **Why not external `avfoundation`?** FFmpeg's `avfoundation` demuxer does **not** support window capture—it only captures physical cameras or whole displays (`Capture screen 0`). Cropping via `crop=w:h:x:y` using `CGWindowListCopyWindowInfo` bounds captures overlapping windows and breaks when the window moves.
3. **Why not `screencapture -l`?** Running `/usr/sbin/screencapture -l <wid>` spawns a new process per frame, dropping frames at 60 FPS.
4. **TCC Gate:** External captures require macOS Screen Recording permissions (`kTCCServiceScreenCapture`), failing on headless CI. In-process capture needs zero permissions.

---

## 1. In-Game Frame Dump + FFmpeg (Recommended)

- **Architecture:** Bevy reads its Metal swapchain/render target in-process and writes raw frames to `stdout` or disk. FFmpeg encodes or formats per-frame PNGs.
- **FFmpeg Command:**
  ```bash
  cargo run -- --record-raw | ffmpeg -f rawvideo -pixel_format rgba \
    -video_size 1280x720 -framerate 60 -i pipe:0 -c:v png frames/frame_%04d.png
  ```
- **Primary Sources:**
  - FFmpeg rawvideo demuxer: [ffmpeg.org/ffmpeg-formats.html#rawvideo](https://ffmpeg.org/ffmpeg-formats.html#rawvideo)
  - Pipe pattern: `pytorch/executorch` (`examples/models/voxtral_realtime/main.cpp`), `elizaOS/eliza` (`packages/app-core/scripts/voice-interactive.mjs`)
  - Awesome-list endorsement: `jaywcjlove/awesome-swift-macos-apps` (`BetterCapture`, https://github.com/jsattler/BetterCapture)

---

## 2. macOS FFmpeg `avfoundation` (Full-Screen + Crop)

- **Behavior:** `avfoundation` enumerates monitors (`[2] Capture screen 0`), not windows. Capturing a window requires pre-querying window coordinates via `CGWindowListCopyWindowInfo` and applying FFmpeg's `crop` filter.
- **FFmpeg Command:**
  ```bash
  ffmpeg -f avfoundation -framerate 60 -capture_cursor 0 -i "2:none" \
    -vf "crop=1280:720:100:200" -c:v png frames/frame_%04d.png
  ```
- **Verified Demuxer Options (`ffmpeg -h demuxer=avfoundation`):**
  `-list_devices`, `-video_device_index`, `-audio_device_index`, `-pixel_format` (default `yuv420p`), `-framerate` (default `ntsc`), `-video_size`, `-capture_cursor`, `-capture_mouse_clicks`, `-capture_raw_data`, `-drop_late_frames`.
- **Primary Sources:**
  - FFmpeg devices doc: [ffmpeg.org/ffmpeg-devices.html#avfoundation](https://ffmpeg.org/ffmpeg-devices.html#avfoundation)
  - Code examples: `lobehub/lobehub` (`record-electron-demo.sh`), `cytopia/ffscreencast` (`bin/ffscreencast`), `screenpipe/screenpipe` (`capture.sh`)
  - Window bounds lookup: `sweetpad-dev/sweetpad` (`sweetpad-cli/src/cli/commands/app/macwin.rs`), `nashaofu/xcap` (`src/macos/impl_window.rs`)

---

## 3. macOS `screencapture -l` + FFmpeg Post-Process

- **Behavior:** macOS utility `/usr/sbin/screencapture` can target a single window by `kCGWindowNumber` (`-l <id>`). Single static frames are captured to disk and encoded with FFmpeg.
- **Commands:**
  ```bash
  /usr/sbin/screencapture -l <WINDOW_ID> -x -o -t png /tmp/shot.png
  ffmpeg -framerate 60 -i /tmp/frames_%04d.png -c:v png out/%04d.png
  ```
- **Limitations:** Spawning `screencapture` per frame cannot sustain 60 FPS (15–50ms process latency).
- **Primary Sources:**
  - Real-world code: `ammaarreshi/Generals-Mac-iOS-iPad` (`Screenshot_macos.cpp`), `QwenLM/qwen-code` (`capture.rs`), `trycua/cua` (`capture.rs`), `lobehub/lobehub` (`capture-app-window.sh`)
  - Awesome-list entries: `jaywcjlove/awesome-swift-macos-apps` (`macosrec`, https://github.com/xenodium/macosrec; `capcap`, https://github.com/realskyrin/capcap)

---

## 4. Cross-Platform Reference: `gdigrab` (Win) & `x11grab` (Linux)

Unlike macOS, Windows and Linux support native window targeting in FFmpeg:
- **Windows (`gdigrab`):**
  ```bash
  ffmpeg -f gdigrab -framerate 60 -i title="topdown-racer" -c:v png frames/%04d.png
  ```
  *Docs:* [ffmpeg.org/ffmpeg-devices.html#gdigrab](https://ffmpeg.org/ffmpeg-devices.html#gdigrab) (`-draw_mouse 0`, `title=*name*`, `hwnd=*id*`). Code: `robotstreamer/robotstreamer` (`send_video_windows.py`).
- **Linux (`x11grab`):**
  ```bash
  ffmpeg -f x11grab -window_id 0x3e00001 -framerate 60 -i :0.0 -c:v png frames/%04d.png
  ```
  *Docs:* [ffmpeg.org/ffmpeg-devices.html#x11grab](https://ffmpeg.org/ffmpeg-devices.html#x11grab) (`-window_id <wid>`). Code: `warpdotdev/warp` (`recording.rs`), `notune/velo` (`CLAUDE.md`).

---

## 5. macOS TCC Screen-Recording Permission

- **External Capture Gate:** Any tool reading screen pixels (`avfoundation`, `screencapture -l`, `ScreenCaptureKit`) requires `kTCCServiceScreenCapture`. If denied, `avfoundation` captures blank desktop wallpaper and `screencapture` returns empty images. Headless CI lacks GUI dialogs to authorize TCC.
- **In-Process Immunity:** The Bevy process reading its own swapchain/Metal textures requires zero TCC permissions.

---

## 6. Comparison Table

| Approach | Platform | Window-Only? | 60 FPS Real-Time? | TCC Permission? | Complexity & Fragility |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **In-Game + FFmpeg Pipe** | Any (macOS/Metal) | **Yes (exact)** | **Yes (sync 60fps)** | **None** | Low; zero occlusion or DPI scaling issues |
| **`avfoundation` + Crop** | macOS | No (crops screen) | Yes (drops frames) | Required | High; occlusions leak, window movement breaks crop |
| **`screencapture -l`** | macOS | **Yes (exact)** | No (<20 fps) | Required | Medium; process-spawn bottleneck |
| **`gdigrab`** | Windows | **Yes (`title=`/`hwnd=`)** | Yes | UAC/OS dependent | Low; native OS demuxer |
| **`x11grab`** | Linux (X11) | **Yes (`-window_id`)** | Yes | None (X11 auth) | Low; native OS demuxer |

*Note: round 2 (`ffmpeg-window-capture-round2.md`) refines this with one correction — on Metal the swapchain is `Bgra8UnormSrgb`, so the pipe uses `-pixel_format bgra`, not `rgba` as sketched in §1 above.*
