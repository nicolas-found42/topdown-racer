# Rust stack for a top-down racer: docs-first comparison

Scope: Bevy vs macroquad vs ggez / Tetra / Fyrox, judged on 2D top-down needs
(2D rendering + camera, fixed timestep, input, audio, 2D physics, WASM story,
binary size / complexity). Every claim below is cited to the primary doc that
owns it — official sites, docs.rs rustdoc, or first-party GitHub repos. No
secondary write-ups used as authority.

## Verdict

- **If the Rust pilot must live inside the existing Vite app (Rust→WASM
  module): macroquad (on miniquad) is the lowest-friction spike.** One
  `cargo build --target wasm32-unknown-unknown`, one JS loader line, same code
  on native and web, smallest dependency/binary story
  ([macroquad WASM build](https://github.com/not-fl3/macroquad),
  [miniquad JS-interop article](https://macroquad.rs/articles/wasm/),
  [macroquad crate docs](https://docs.rs/macroquad)). Physics for a top-down
  arcade car is hand-rolled anyway (custom drift/grip model, not rigid-body
  stacking), and macroquad's `physics-platformer` helper is explicitly
  rect-only platformer WIP — i.e. don't expect a car physics crate from this
  stack ([physics-platformer README](https://github.com/not-fl3/macroquad/tree/master/physics-platformer)).
- **If greenfield native-first with room to grow (editor, replays, netplay):
  Bevy + Avian2d is the deep end worth swimming in.** First-party fixed
  timestep (`Time<Fixed>`, `FixedUpdate`), 2D render graph (`Camera2d`,
  `Sprite`), full input/audio subsystems, and a native-ECS physics engine
  (Avian) with interpolation, CCD, and determinism hooks
  ([Bevy fixed-timestep example](https://bevy.org/examples/movement/physics-in-fixed-timestep/),
  [`Time<Fixed>` rustdoc](https://docs.rs/bevy/latest/bevy/time/struct.Fixed.html),
  [`Camera2d` rustdoc](https://docs.rs/bevy/latest/bevy/prelude/struct.Camera2d.html),
  [Avian docs](https://docs.rs/avian2d),
  [Avian repo](https://github.com/avianphysics/avian)). Cost: real compile
  times (mitigated by `dynamic_linking`/LLD per the setup guide) and a heavier
  WASM story (`web`/`webgl2`/`webgpu` features, `wasm-release` profile,
  `wasm-opt`) ([Bevy setup guide](https://bevyengine.org/learn/quick-start/getting-started/setup/),
  [Bevy crate features](https://docs.rs/bevy/latest/bevy/index.html)).
- **ggez / Tetra are out for WASM; Fyrox is the native-editor dark horse.**
  ggez has no direct WASM support (only the `good-web-game` subset port)
  ([ggez issue #71](https://github.com/ggez/ggez/issues/71),
  [`good-web-game` docs](https://docs.rs/good-web-game)); Tetra is SDL-based
  and explicitly unmaintained since Jan 2022 ([Tetra docs](https://docs.rs/tetra));
  Fyrox 1.0 ships a Unity-like editor plus a `wasm32-unknown-unknown` export
  CLI, but it is a full scene-editor engine, not a lightweight canvas module
  ([Fyrox 1.0.0 post](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/)).
- **The tiebreaker is architectural, not ideological:** embedding in Vite
  wants a *library that renders into a `<canvas>` and exposes a few extern
  functions* (the miniquad plugin model,
  [article](https://macroquad.rs/articles/wasm/)); greenfield wants a
  *framework that owns the loop, schedule, and assets* (Bevy's `App` +
  schedules, [crate docs](https://docs.rs/bevy/latest/bevy/index.html)). MDN
  frames exactly these two WASM postures — whole app in Rust vs Rust as part
  of an existing JS frontend — and notes the Rust team focuses on the latter
  ([MDN Rust→WASM](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Rust_to_Wasm)).

## Comparison table

| Axis | Bevy (0.19-era docs) | macroquad (0.4.x) | Alternatives | What to steal for a top-down racer |
|---|---|---|---|---|
| 2D rendering + camera | `Camera2d` component "enables the 2D render graph for a `Camera`" ([rustdoc](https://docs.rs/bevy/latest/bevy/prelude/struct.Camera2d.html)); `Sprite` in prelude ([prelude](https://docs.rs/bevy/latest/bevy/prelude/index.html)); built-in 2D renderer is a named feature collection `2d_bevy_render` = `bevy_render` + `bevy_sprite_render` + pipelines ([features](https://docs.rs/bevy/latest/bevy/index.html)); 2D-only profile `bevy = { default-features = false, features = ["2d"] }` ([same](https://docs.rs/bevy/latest/bevy/index.html)) | "Efficient 2D rendering with automatic geometry batching", same code all platforms ([crate](https://docs.rs/macroquad)); `camera` module: `Camera2D`, `set_camera`, `set_default_camera`, push/pop state ([module](https://docs.rs/macroquad/latest/macroquad/camera/index.html)); per-frame `clear_background` + `draw_*` + `next_frame().await` loop ([README](https://github.com/not-fl3/macroquad)) | ggez: "portable 2D drawing" on `wgpu`, sprite batches/instancing, render targets ([ggez.rs](https://ggez.rs/)); Tetra: "efficient 2D rendering, draw call batching by default", built-in cameras + screen scaling ([docs](https://docs.rs/tetra)); Fyrox: "create 2D or 3D games, or even mix", PBR renderer + native scene editor ([Fyrox 1.0.0](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/)) | Top-down camera = follow + rotate-to-heading. Bevy: camera-after-physics pattern from the fixed-timestep example (below). macroquad: `set_camera(Camera2D{…})` per frame — trivial follow cam. Tetra's screen-scaling helper is the prior art for letterboxed track views. |
| Fixed timestep | First-class: `Time<Fixed>` default 64 Hz (power-of-two reasoning documented), `FixedUpdate` runs 0..n times/frame between `PreUpdate` and `Update`, follows virtual time (pause/speed) ([rustdoc](https://docs.rs/bevy/latest/bevy/time/struct.Fixed.html)); official example prescribes accumulate-input → `FixedUpdate` physics → interpolate `Transform` → variable-step camera in `RunFixedMainLoop::{Before,After}FixedMainLoop` ([example](https://bevy.org/examples/movement/physics-in-fixed-timestep/)) | No fixed-step scheduler: `time` module only offers `get_fps`, `get_frame_time` (last-frame seconds), `get_time` (wall clock) ([module](https://docs.rs/macroquad/latest/macroquad/time/index.html)); frame pacing is `next_frame().await` ([window](https://docs.rs/macroquad/latest/macroquad/window/index.html)) | Tetra: "deterministic game loop by default, à la Fix Your Timestep" ([docs](https://docs.rs/tetra)); Avian: `Transform` interpolation/extrapolation for fixed timesteps, configurable substeps via `SubstepCount`, pausable/steppable `Physics` schedule ([Avian](https://docs.rs/avian2d)); Rapier: `enhanced-determinism` feature incl. WASM targets ([Rapier Bevy guide](https://rapier.rs/docs/user_guides/bevy_plugin/getting_started_bevy/)) | Deterministic sim at fixed dt with render interpolation is the pattern our TS sim already approximates. Bevy/Avian give it as engine service; on macroquad hand-roll the Glenn Fiedler accumulator (`get_frame_time` as the variable input) — ~30 lines, and it keeps ghost/replay determinism. |
| Input | `bevy_input`: keyboard, mouse, gamepad, touch; `ButtonInput<T>` press-ables, `InputPlugin`, gesture/touch modules ([module](https://docs.rs/bevy/latest/bevy/input/index.html)); gamepad via `bevy_gilrs`, keyboard/mouse auto-enabled by `bevy_window` ([features](https://docs.rs/bevy/latest/bevy/index.html)) | Polling API: `is_key_down/pressed/released`, `get_keys_*`, mouse pos/buttons/wheel, `touches()`; "gamepads soon" ([module](https://docs.rs/macroquad/latest/macroquad/input/index.html)); touch raises mouse events by default (`simulate_mouse_with_touch`) ([same](https://docs.rs/macroquad/latest/macroquad/input/index.html)) | ggez: "keyboard and mouse events easily through callbacks" ([ggez.rs](https://ggez.rs/)); Tetra: "easy input handling, via polling or events, with gamepads" ([docs](https://docs.rs/tetra)); `good-web-game`: no gamepad on WASM (`gilrs` needs wasm-bindgen) ([docs](https://docs.rs/good-web-game)) | Coast-only controls need pressed/down/released edges + touch fallback. Bevy `ButtonInput` and macroquad `is_key_*` both cover it; note the WASM gamepad gap if gamepad is ever required on web. |
| Audio | `bevy_audio`: `AudioPlayer` component + `PlaybackSettings::LOOP`, `AudioSink` controls, spatial audio types ([module](https://docs.rs/bevy/latest/bevy/audio/index.html)); format features `vorbis` default, `audio-all-formats` adds aac/flac/mp3/mp4/wav ([features](https://docs.rs/bevy/latest/bevy/index.html)) | `audio` module: `load_sound[_from_bytes]`, `play_sound[_once]`, `stop_sound`, `set_sound_volume` ([module](https://docs.rs/macroquad/latest/macroquad/audio/index.html)); Linux needs ALSA dev libs ([README](https://github.com/not-fl3/macroquad)) | ggez: plays/loads ogg+wav+flac via `rodio` ([ggez.rs](https://ggez.rs/)); `good-web-game`: audio API "differs slightly" (`quad-snd` not `rodio`), spatial audio missing ([docs](https://docs.rs/good-web-game)) | Engine hum + skid need loop + volume/pitch control: Bevy `AudioSink` vs macroquad `PlaySoundParams` + `set_sound_volume`. Either suffices; web autoplay policies apply equally (JS-side concern). |
| 2D physics | Two Bevy-native options. **bevy_rapier2d**: `RapierPhysicsPlugin::pixels_per_meter(100)`, `RigidBody`/`Collider`/`Restitution` components, `RapierDebugRenderPlugin` ([guide](https://rapier.rs/docs/user_guides/bevy_plugin/getting_started_bevy/)); features: `debug-render-2d`, `simd-stable/nightly`, `parallel` (rayon), `serde-serialize`, `enhanced-determinism` (incl. WASM), `wasm-bindgen` ([same](https://rapier.rs/docs/user_guides/bevy_plugin/getting_started_bevy/)). **Avian**: no separate physics world — pure ECS components (`RigidBody::Dynamic` + `Collider::circle(0.5)`), `PhysicsPlugins::default()` ([docs](https://docs.rs/avian2d), [repo](https://github.com/avianphysics/avian)); Parry-backed colliders, CCD (speculative + swept), joints, `CollisionLayers`, sensors, `SpatialQuery`, `PhysicsInterpolationPlugin`, `f32`/`f64` precision, `enhanced-determinism`/`parallel`/`simd` flags ([same](https://docs.rs/avian2d)); Avian FAQ: Rapier more mature/feature-rich, Avian less sync overhead + more native ECS feel, core APIs similar so switching is cheap ([FAQ](https://docs.rs/avian2d)); version table pins Bevy 0.19 ↔ Avian 0.7 ([repo](https://github.com/avianphysics/avian)) | First-party helper is platformer-only: "rectangular colliders", "very WIP" ([README](https://github.com/not-fl3/macroquad/tree/master/physics-platformer)); full Rapier usable as a plain dependency (no Bevy plugin needed) but then you own stepping/sync | Fyrox: built-in 2D physics incl. debug rendering (fixed in 1.0: "fixed debug rendering in 2d") ([post](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/)); Tetra/ggez: no bundled physics (roll your own) | A top-down arcade racer likely wants **custom 2D car math** (grip circles, drift states), not stacked rigid bodies — mine Avian/Rapier for CCD + `CollisionLayers` + interpolation patterns, but expect to keep the TS-style kinematic sim. If real rigid bodies are ever needed, Avian's `LockedAxes` + CCD + interpolation is the closest to "car on a track". |
| WASM build story | Supported but heavyweight: `web` feature (browser APIs, wasm32-only), `webgl2` default-compat vs `webgpu` override, examples ship as WebGL2 ([features](https://docs.rs/bevy/latest/bevy/index.html)); official setup guide prescribes a `wasm-release` profile (`opt-level="s"`, `strip`) used via `--profile wasm-release`, plus optional `wasm-opt -Os` from Binaryen, with links to the Rust WASM code-size books ([setup](https://bevyengine.org/learn/quick-start/getting-started/setup/)); 0.15→0.16 note: browser WASM features moved behind the `web` flag ([migration](https://bevyengine.org/learn/migration-guides/0-15-to-0-16/)) | Lightest story in the set: `rustup target add wasm32-unknown-unknown` + `cargo build --target wasm32-unknown-unknown`, load with `<script>load("CRATENAME.wasm")</script>` next to `mq_js_bundle.js` ([README](https://github.com/not-fl3/macroquad)); deeper JS↔Rust FFI via miniquad plugins (`register_plugin`, `wasm_exports.*`, `sapp-jsutils` for strings/objects) ([article](https://macroquad.rs/articles/wasm/)); `quad-storage` persists local data on WASM ([crate note](https://docs.rs/quad-storage)) | ggez: **no direct WASM** — port via `good-web-game`, a miniquad-based ggez-0.6.1/0.7.0 subset with listed gaps (no writable fs, no custom event loop, GLSL100 shaders, `quad-snd` audio) ([ggez #71](https://github.com/ggez/ggez/issues/71), [good-web-game](https://docs.rs/good-web-game)); Tetra: SDL + native libs, no web story ([docs](https://docs.rs/tetra)); Fyrox: `export-cli --target-platform wasm --build-target wasm32-unknown-unknown` ([post](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/)); plumbing: `wasm-bindgen` bridges rich types + TS bindings ([guide](https://rustwasm.github.io/docs/wasm-bindgen/)), `wasm-pack build --target web` emits the npm-ready package from a `cdylib` ([MDN](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Rust_to_Wasm)) | For outcome (a) Vite-embedded: macroquad's loader model drops straight into a Vite static asset + thin JS bridge; Bevy's whole-`App` + asset-pipeline model fights embedding. For outcome (b) greenfield: Fyrox export CLI or Bevy + `wasm-release` + `wasm-opt` are the documented paths. |
| Binary size / complexity | Feature-gated: `2d`-only profile, `default_font` costs "20kB", clipboard image "not supported on WASM", single-thread toggle `multi_threaded` ([features](https://docs.rs/bevy/latest/bevy/index.html)); dev-loop cost addressed via `dynamic_linking` + LLD/mold + dev-profile `opt-level` split ([setup](https://bevyengine.org/learn/quick-start/getting-started/setup/)) | Minimalism is the pitch: "minimal dependencies: build after `cargo clean` takes only 16s on x230", immediate-mode UI included, "single command deploy for WASM and Android" ([crate](https://docs.rs/macroquad)) | Fyrox 1.0 notes reduced VRAM (rgba16f→rgb10a2) as an example of size/perf care ([post](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/)); Rust WASM code-size discipline lives in the linked size books ([Bevy setup links](https://bevyengine.org/learn/quick-start/getting-started/setup/)) | If WASM payload budget rules (mobile Safari load), macroquad starts an order of magnitude leaner; Bevy pays for ECS+renderer generality and must earn it back with feature trimming + `wasm-opt`. |

## Implications for the two pending outcomes

### (a) Rust→WASM inside the existing Vite app

- macroquad fits the MDN "part of an application" posture: compile one
  `wasm32-unknown-unknown` module, serve it as a static asset, drive it from
  the existing TS shell (menus, HUD, leaderboard stay in TS)
  ([MDN use cases](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Rust_to_Wasm),
  [macroquad WASM](https://github.com/not-fl3/macroquad)).
- The miniquad plugin model (`wasm_exports.my_fn(...)` JS→Rust,
  `importObject.env.*` Rust→JS, `sapp-jsutils` for strings/objects) is the
  documented seam for ghost upload / track-code exchange without adopting
  `wasm-bindgen` ceremony
  ([article](https://macroquad.rs/articles/wasm/)).
- `wasm-pack build --target web` + `wasm-bindgen` remains the fallback if the
  module must expose typed/TS-friendly APIs to the Vite app instead of raw
  externs ([wasm-bindgen guide](https://rustwasm.github.io/docs/wasm-bindgen/),
  [MDN](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Rust_to_Wasm)).
- Bevy is embeddable in principle (`web` feature, `webgl2` compat) but its
  `App`-owns-everything shape, asset server, and WASM size pipeline
  (`wasm-release` + `wasm-opt`) make it a page-owner, not a widget
  ([features](https://docs.rs/bevy/latest/bevy/index.html),
  [setup](https://bevyengine.org/learn/quick-start/getting-started/setup/)).
- ggez/Tetra are non-starters here (no direct WASM / SDL natives); Fyrox's
  export CLI targets standalone pages, not Vite-embedded modules
  ([good-web-game gaps](https://docs.rs/good-web-game),
  [Tetra](https://docs.rs/tetra),
  [Fyrox export](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/)).

### (b) Native-first greenfield

- Bevy + Avian is the documented whole-game stack: 2D renderer + `Camera2d`,
  `FixedUpdate` sim with interpolation, `ButtonInput` input, `AudioPlayer`
  audio, ECS physics with CCD/determinism flags, debug rendering throughout
  ([crate](https://docs.rs/bevy/latest/bevy/index.html),
  [Camera2d](https://docs.rs/bevy/latest/bevy/prelude/struct.Camera2d.html),
  [Fixed](https://docs.rs/bevy/latest/bevy/time/struct.Fixed.html),
  [input](https://docs.rs/bevy/latest/bevy/input/index.html),
  [audio](https://docs.rs/bevy/latest/bevy/audio/index.html),
  [Avian](https://docs.rs/avian2d)).
- The fixed-timestep example's camera guidance (rotate-before / translate-after
  the step, interpolate the followed transform) ports directly to a chase cam
  ([example](https://bevy.org/examples/movement/physics-in-fixed-timestep/)).
- Avian's FAQ explicitly blesses the Rapier↔Avian swap (similar core APIs),
  so starting with one physics backend is reversible
  ([FAQ](https://docs.rs/avian2d)); Rapier's `pixels_per_meter(100.0)` shows
  the intended pixel-world scaling convention
  ([guide](https://rapier.rs/docs/user_guides/bevy_plugin/getting_started_bevy/)).
- macroquad remains viable as the "game-jam-shaped" greenfield (16-second
  clean builds, immediate UI, trivial native+WASM parity) if the team prefers
  velocity over engine services ([crate](https://docs.rs/macroquad)).
- Fyrox deserves a look only if a Unity-style scene editor is a requirement;
  that is a workflow decision, not a renderer decision
  ([post](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/)).

## Mineable examples (steal these regardless of engine)

| Idea | Primary source | Why it matters here |
|---|---|---|
| Fixed-step + interpolation + split camera update | [Bevy physics-in-fixed-timestep example](https://bevy.org/examples/movement/physics-in-fixed-timestep/) | Direct template for deterministic car sim + smooth chase cam; mirrors our TS fixed-step needs |
| `Time<Fixed>` 64 Hz default + overstep accumulator semantics | [`Time<Fixed>` rustdoc](https://docs.rs/bevy/latest/bevy/time/struct.Fixed.html) | Justifies sim rate choice; power-of-two note avoids refresh-rate pathology |
| Avian interpolation / extrapolation plugin + substeps + pausable schedule | [Avian docs](https://docs.rs/avian2d) | Ghost/replay determinism + pause/step debugging |
| Rapier pixel scaling + debug-render plugin + determinism/WASM features | [Rapier Bevy guide](https://rapier.rs/docs/user_guides/bevy_plugin/getting_started_bevy/) | Pixel-world conventions; `enhanced-determinism` for leaderboard integrity |
| miniquad JS plugin bridge + `sapp-jsutils` typed interop | [JS interop article](https://macroquad.rs/articles/wasm/) | Cheapest Vite↔WASM seam for track codes and ghosts |
| Tetra deterministic loop + built-in cameras/screen scaling | [Tetra docs](https://docs.rs/tetra) | Reference design for the hand-rolled macroquad loop (accumulator + letterbox) |
| `good-web-game` gap list (fs, loop, audio, gamepad-on-WASM) | [`good-web-game` docs](https://docs.rs/good-web-game) | Checklist of what breaks when porting any native-first engine to web |
| Bevy `wasm-release` profile + `wasm-opt` + size-book links | [Bevy setup guide](https://bevyengine.org/learn/quick-start/getting-started/setup/) | WASM payload budget playbook if Bevy wins greenfield |
| Fyrox `export-cli` multi-target flow | [Fyrox 1.0.0 post](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/) | CI/CD shape for native + WASM + Android artifacts |

## Sources (primary only)

- Bevy: [crate docs + feature table](https://docs.rs/bevy/latest/bevy/index.html),
  [`Camera2d`](https://docs.rs/bevy/latest/bevy/prelude/struct.Camera2d.html),
  [`Time<Fixed>`](https://docs.rs/bevy/latest/bevy/time/struct.Fixed.html),
  [fixed-timestep example](https://bevy.org/examples/movement/physics-in-fixed-timestep/),
  [`bevy_input`](https://docs.rs/bevy/latest/bevy/input/index.html),
  [`bevy_audio`](https://docs.rs/bevy/latest/bevy/audio/index.html),
  [setup guide (WASM profiles, linking)](https://bevyengine.org/learn/quick-start/getting-started/setup/),
  [0.15→0.16 migration (`web` flag)](https://bevyengine.org/learn/migration-guides/0-15-to-0-16/)
- macroquad/miniquad: [crate docs](https://docs.rs/macroquad),
  [README (features, WASM build)](https://github.com/not-fl3/macroquad),
  [JS-interop article](https://macroquad.rs/articles/wasm/),
  [camera](https://docs.rs/macroquad/latest/macroquad/camera/index.html),
  [input](https://docs.rs/macroquad/latest/macroquad/input/index.html),
  [time](https://docs.rs/macroquad/latest/macroquad/time/index.html),
  [window](https://docs.rs/macroquad/latest/macroquad/window/index.html),
  [audio](https://docs.rs/macroquad/latest/macroquad/audio/index.html),
  [physics-platformer](https://github.com/not-fl3/macroquad/tree/master/physics-platformer)
- Physics: [Rapier Bevy guide](https://rapier.rs/docs/user_guides/bevy_plugin/getting_started_bevy/),
  [Avian docs + FAQ](https://docs.rs/avian2d),
  [Avian repo (design, version table)](https://github.com/avianphysics/avian)
- Alternatives: [ggez homepage](https://ggez.rs/),
  [ggez WASM issue #71](https://github.com/ggez/ggez/issues/71),
  [`good-web-game` docs](https://docs.rs/good-web-game),
  [Tetra docs (maintenance status)](https://docs.rs/tetra),
  [Fyrox 1.0.0 release](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/)
- WASM plumbing: [`wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/),
  [MDN Rust→WASM](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Rust_to_Wasm)
