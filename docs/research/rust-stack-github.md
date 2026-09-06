# Rust stack — GitHub mining (top-down racer)

## Verdict

- **Closest thing to our game that already exists: `gbredz1/BevyRaceTiles`.**
  Bevy + `bevy_rapier2d` top-down racer with Tiled tracks, sensor checkpoints,
  lap counting, and an arcade drift model (forward/lateral velocity split).
  Steal its `car_driving` shape first; it maps almost 1:1 onto our
  `simulation.ts` grip model.
- **If we want physics-engine-free determinism: `RyanSpaker/bevy_car_ai`.**
  Custom `TrackTransform` physics with the one-line drift formulation
  (`project_onto_normalized(...).lerp(...)`), all inside `FixedUpdate`.
  This is the strongest precedent for "hand-rolled arcade physics on a fixed
  step" — i.e. what we already do in TS, ported to Rust.
- **If we want real rigid-body cars: `alexichepura/bevy_garage`
  (+ `gavlig/gryazevichki` for the joint recipe).**
  Rapier joints + motors + ESP stability control. Powerful but 3D, heavy, and
  the opposite of deterministic — only relevant for a greenfield sim rethink.
- **Physics engine choice, if we use one: `avianphysics/avian` over
  `dimforge/bevy_rapier`.** Avian is ECS-native (no separate physics world),
  has a cross-platform determinism test with transform hashing, and ships 2D
  joint/character/sensor examples. Rapier is still the safe default with the
  biggest example corpus.
- **WASM story is settled on both sides:** macroquad documents a one-command
  `wasm32-unknown-unknown` build; Bevy ships to WASM in production
  (`bevy_garage` has a live WASM demo). Either answer to Q0 keeps a web build.
- **Method note:** `searchGitHub` (literal code-pattern search) was tried for
  `avian2d`, `car_controller`, `fn drift`, `steering` — it returns mostly
  unrelated hits (agent-steering code, lock-service drift, etc.) and is a poor
  discovery tool for games. Repos below were discovered via web search and
  **verified by reading the actual source files** (raw GitHub URLs); every
  claim links to the file it came from.

## Candidate table

| Repo | Stack | What to steal | Source links |
|---|---|---|---|
| `gbredz1/BevyRaceTiles` | Bevy + `bevy_rapier2d` + `bevy_ecs_tiled`/`bevy_ecs_tilemap`, 2D top-down | Drift model (`forward_velocity + right_velocity * drift_factor`), road/off-road grip switch, sensor checkpoints + lap tracker, Tiled collision groups, lap HUD | [car.rs](https://github.com/gbredz1/bevyracetiles/blob/main/src/entities/car.rs) · [physics.rs](https://github.com/gbredz1/bevyracetiles/blob/main/src/level/physics.rs) · [car_checkpoint.rs](https://github.com/gbredz1/bevyracetiles/blob/main/src/entities/car_checkpoint.rs) · [player](https://github.com/gbredz1/bevyracetiles/blob/main/src/player/mod.rs) · [ui_laps.rs](https://github.com/gbredz1/bevyracetiles/blob/main/src/camera/ui_laps.rs) |
| `RyanSpaker/bevy_car_ai` | Bevy only (no physics engine), custom 2D arcade physics | One-line drift (`project_onto_normalized().lerp()`), `FixedUpdate` physics chain, `TrackConfig` track↔world mapping + auto-scale | [car.rs](https://github.com/RyanSpaker/bevy_car_ai/blob/main/src/car.rs) · [track.rs](https://github.com/RyanSpaker/bevy_car_ai/blob/main/src/track.rs) |
| `alexichepura/bevy_garage` | Bevy + `bevy_rapier3d`, 3D car sim + DQN driving AI | Impulse-joint car assembly, GenericJoint steering basis, ESP slip/torque control, aero drag/downforce, ring-buffer replay, WASM-deploy precedent | [car.rs](https://github.com/alexichepura/bevy_garage/blob/main/car/src/car.rs) · [wheel.rs](https://github.com/alexichepura/bevy_garage/blob/main/car/src/wheel.rs) · [esp.rs](https://github.com/alexichepura/bevy_garage/blob/main/car/src/esp.rs) · [joint.rs](https://github.com/alexichepura/bevy_garage/blob/main/car/src/joint.rs) · [replay.rs](https://github.com/alexichepura/bevy_garage/blob/main/nn/src/replay.rs) · [README (WASM demo)](https://github.com/alexichepura/bevy_garage) |
| `gavlig/gryazevichki` | Bevy + Rapier3D joints/motors | Chained revolute-joint wheel (steer joint + spin joint), per-wheel density/size tuning UI | [src/main.rs (legacy branch)](https://github.com/gavlig/gryazevichki/blob/gryazevichki_rapier_v0.12.0-alpha.0/src/main.rs) · [repo](https://github.com/gavlig/gryazevichki) |
| `dimforge/bevy_rapier` | Official Rapier Bevy plugin (2D + 3D) | `pixels_per_meter` 2D setup, `CollisionGroups` layers, joints/events/character-controller examples | [repo + 2D examples](https://github.com/dimforge/bevy_rapier) |
| `avianphysics/avian` | ECS-native Bevy physics (2D + 3D, Parry collisions) | `PhysicsPlugins` + `FixedUpdate` pattern, `RevoluteJoint` API, determinism test w/ transform hash, sensor/collision-layer examples | [repo](https://github.com/avianphysics/avian) · [determinism_2d.rs](https://github.com/avianphysics/avian/blob/main/crates/avian2d/examples/determinism_2d.rs) |
| `not-fl3/macroquad` | macroquad (miniquad, immediate-mode) | `#[macroquad::main]` loop + `next_frame()`, one-command WASM build, asteroids top-down movement (thrust/friction/speed-clamp), `physics-platformer` side crate | [repo + WASM docs](https://github.com/not-fl3/macroquad) · [asteroids.rs](https://github.com/not-fl3/macroquad/blob/master/examples/asteroids.rs) |
| `bones-ai/bevy-2d-shooter` | Bevy only, no physics engine (kd-tree collisions per README) | Engine-free top-down movement + input, event-based collision → damage, `GameState`-gated systems | [repo](https://github.com/bones-ai/bevy-2d-shooter) · [player.rs](https://github.com/bones-ai/bevy-2d-shooter/blob/main/src/player.rs) |
| `bevyengine/bevy` | Bevy engine + official examples | `FixedUpdate` + `Time<Fixed>` scheduling, canonical `App`/`Plugin`/`Query` structure | [fixed_timestep.rs](https://github.com/bevyengine/bevy/blob/main/examples/ecs/fixed_timestep.rs) · [repo](https://github.com/bevyengine/bevy) |

## Details per repo

### 1. `gbredz1/BevyRaceTiles` — the reference implementation
Bevy top-down 2D racer, MIT, Kenney assets, Tiled (`level0.tmx`) maps.
https://github.com/gbredz1/bevyracetiles

- **Game loop:** `App` + plugins (`LevelPlugin`, `PlayerPlugin`, `CameraPlugin`,
  `EntitiesPlugin`) —
  [main.rs](https://github.com/gbredz1/bevyracetiles/blob/main/src/main.rs).
- **Car physics** (`src/entities/car.rs`): `RigidBody::Dynamic` + `Velocity`
  driven directly each frame — gas applies force along `transform.up()`,
  speed clamped to `max_speed`, steering sets `angvel` with a min-speed gate
  and reverse flip, then the drift step:
  `linvel = forward_velocity + right_velocity * drift_factor`
  ([car.rs](https://github.com/gbredz1/bevyracetiles/blob/main/src/entities/car.rs)).
  This is exactly our `simulation.ts` grip decomposition with different
  constant names.
- **Grip surfaces:** `car_update` switches `max_speed`/`drift_factor`/
  `damping_add` on `on_road`, which is set by rapier `Sensor` overlap with
  road colliders; handbrake raises damping and sets `drift_factor = 0.99`
  ([car.rs](https://github.com/gbredz1/bevyracetiles/blob/main/src/entities/car.rs)).
- **Track representation:** Tiled map with collider `group` properties
  (0 wall / 1 road / 2 checkpoint / 3 car spawn) mapped to rapier
  `CollisionGroups` (`ROAD/WALL/CHECK/CAR_GROUP`), gravity set to
  `Vec2::ZERO` for top-down
  ([physics.rs](https://github.com/gbredz1/bevyracetiles/blob/main/src/level/physics.rs)).
- **Laps/ghost-seed:** `CheckpointTracker { checkpoint_visits: HashSet<usize>,
  laps_completed, total_checkpoints }` consumes `CheckpointEvent`s in order,
  tolerates hitting checkpoint `n-1` twice, increments laps on returning to 0
  ([car_checkpoint.rs](https://github.com/gbredz1/bevyracetiles/blob/main/src/entities/car_checkpoint.rs));
  lap HUD via `TextSpan` queries
  ([ui_laps.rs](https://github.com/gbredz1/bevyracetiles/blob/main/src/camera/ui_laps.rs)).
- **Input:** `ButtonInput<KeyCode>` arrows + Space handbrake → `CarControls`
  ([player/mod.rs](https://github.com/gbredz1/bevyracetiles/blob/main/src/player/mod.rs)).
- **Steal:** the whole `car_driving` function shape; the road-sensor →
  grip-factor pattern (generalizes to our surface/grip model); the ordered
  checkpoint-set lap counter (robust to sensor double-fires — directly
  applicable to our checkpoint validation).

### 2. `RyanSpaker/bevy_car_ai` — engine-free arcade physics on a fixed step
Top-down driving + AI in Bevy with **no physics crate**.
https://github.com/RyanSpaker/bevy_car_ai

- **Physics** (`src/car.rs`, `TrackTransform::update_physics`): rotational
  accel clamped to max rotvel; acceleration along heading; exponential
  friction (`velocity *= 0.97`); drift in one line —
  `velocity.project_onto_normalized(forward).lerp(velocity, drift_factor)`
  with `drift_factor = 0.99` — then `clamp_length_max`
  ([car.rs](https://github.com/RyanSpaker/bevy_car_ai/blob/main/src/car.rs)).
  Cleaner formulation of the same idea as BevyRaceTiles; note the math is
  identical to ours (keep `1 - drift` of lateral velocity).
- **Fixed step:** input + physics + transform-sync chained in `FixedUpdate`
  ([car.rs `CarPlugin`](https://github.com/RyanSpaker/bevy_car_ai/blob/main/src/car.rs)) —
  precedent for deterministic sim scheduling in Bevy.
- **Track mapping:** `TrackConfig { logical_size, scale }` converts
  track↔world coords and auto-fits the window (`compute_scale`/`update_scale`)
  — mineable for our canvas-fit / minimap mapping
  ([car.rs](https://github.com/RyanSpaker/bevy_car_ai/blob/main/src/car.rs)).
- **Steal:** the `project_onto_normalized().lerp()` drift one-liner; the
  `PreUpdate`-scale / `FixedUpdate`-physics / transform-sync split.

### 3. `alexichepura/bevy_garage` — full rigid-body car + AI training harness
180★ 3D car playground: Bevy + `bevy_rapier3d`, DQN driving AI (`dfdx`),
axum API server, iOS + **live WASM demo**.
https://github.com/alexichepura/bevy_garage

- **Car assembly** (`car/src/car.rs`): body = `round_cuboid` collider with
  NHTSA-referenced inertia (`mass: 1000`, custom `principal_inertia`),
  `CollisionGroups`, CCD, `Sleeping::disabled`; 4 wheels attached via
  `ImpulseJoint`
  ([car.rs](https://github.com/alexichepura/bevy_garage/blob/main/car/src/car.rs)).
- **Wheels** (`car/src/wheel.rs`): `round_cylinder` colliders, friction
  coefficient 5.0, `MODIFY_SOLVER_CONTACTS` hooks
  ([wheel.rs](https://github.com/alexichepura/bevy_garage/blob/main/car/src/wheel.rs)).
- **Steering joint** (`car/src/joint.rs`): `GenericJointBuilder` locking
  `ANG_Y | ANG_Z | LIN_X | LIN_Z`, steering applied later via
  `set_local_basis1(quat)` with smoothed angle
  ([joint.rs](https://github.com/alexichepura/bevy_garage/blob/main/car/src/joint.rs),
  [esp.rs](https://github.com/alexichepura/bevy_garage/blob/main/car/src/esp.rs)).
- **ESP/stability** (`car/src/esp.rs`): slip-angle from velocity-vs-heading
  delta, speed-sensitive torque/steer curves (`torque_speed_x`,
  `steering_speed_x`), per-wheel slip gating, plus `aero_system` quadratic
  drag/downforce — the "assists" layer a top-down arcade game would fake
  with two constants
  ([esp.rs](https://github.com/alexichepura/bevy_garage/blob/main/car/src/esp.rs)).
- **Replay/ghost seed** (`nn/src/replay.rs`): ring-buffer `ReplayBuffer`
  storing `(state, action, reward, next_state, done)` tuples with batched
  tensor sampling — the structural template for a ghost/replay store
  ([replay.rs](https://github.com/alexichepura/bevy_garage/blob/main/nn/src/replay.rs)).
- **WASM:** README links a live WASM demo of the Bevy app — proof Bevy ships
  to the browser in practice
  ([README](https://github.com/alexichepura/bevy_garage)).
- **Steal:** joint-build + `set_local_basis1` steering recipe (if we ever go
  rigid-body); ESP speed-sensitive steer curve idea; replay-buffer shape for
  ghosts; WASM-deploy precedent for Bevy.

### 4. `gavlig/gryazevichki` — minimal jointed vehicle recipe
Prototype vehicle from rigid bodies + joints + motors only, inspired by
Godot's 6DOF demo. https://github.com/gavlig/gryazevichki

- Each wheel = axle body + wheel body attached with **two chained revolute
  joints** (steer about Y, spin about X), motors for drive/steer; params
  (wheel size/density, body density) tunable live via `bevy_egui`
  ([README](https://github.com/gavlig/gryazevichki),
  [src/main.rs](https://github.com/gavlig/gryazevichki/blob/gryazevichki_rapier_v0.12.0-alpha.0/src/main.rs)).
- Caveat: `master` is mid-migration/broken; the working code is on branch
  `gryazevichki_rapier_v0.12.0-alpha.0`.
- **Steal:** the two-joint wheel pattern and the live-tuning-via-UI approach
  for finding drift constants fast.

### 5. `dimforge/bevy_rapier` — official plugin + 2D example corpus
1.5k★ official Rapier↔Bevy bridge. https://github.com/dimforge/bevy_rapier

- `bevy_rapier2d/examples/` includes `joints2.rs`, `rope_joint2.rs`,
  `events2.rs`, `debugdump2.rs`, `serialization2.rs`, `voxels2.rs` — joint,
  event, and debug-dump coverage for anything BevyRaceTiles doesn't show.
- `RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0)` is the 2D
  scaling idiom (seen in BevyRaceTiles' `PhysicsPlugin`).
- **Steal:** `pixels_per_meter` setup; `ActiveEvents::COLLISION_EVENTS` +
  `EventReader<CollisionEvent>` for checkpoints; `debugdump` for track-content
  debugging.

### 6. `avianphysics/avian` — ECS-native physics with determinism tests
3.1k★, "made with Bevy, for Bevy — no wrappers", XPBD solver, Parry
collisions. https://github.com/avianphysics/avian

- No separate physics world: `RigidBody`/`Collider`/`LinearVelocity` are
  plain components; `PhysicsPlugins::default()` + normal Bevy schedules
  ([README usage](https://github.com/avianphysics/avian)).
- `crates/avian2d/examples/determinism_2d.rs` runs a chaotic joint scene for
  500 fixed steps and hashes every body isometry (djb2 over `Position` +
  `Rotation`), compared per-PR on multiple platforms in CI — the exact
  testing pattern our deterministic-sim ADR wants
  ([determinism_2d.rs](https://github.com/avianphysics/avian/blob/main/crates/avian2d/examples/determinism_2d.rs)).
- 2D corpus also covers `revolute_joint_2d`, `distance_joint_2d`,
  `joint_motors_2d`, `sensor.rs`, `collision_layers.rs`,
  `dynamic_character_2d` / `kinematic_character_2d`, `move_and_slide_2d`,
  `interpolation.rs` (Transform interpolation for fixed steps) — see
  `crates/avian2d/examples/` ([repo](https://github.com/avianphysics/avian)).
- `f32`/`f64` precision features — relevant if float determinism ever
  matters.
- **Steal:** Avian over Rapier if we take a physics engine (ECS-native,
  deterministic-CI precedent); the transform-hash determinism test;
  `PhysicsDebugPlugin` for track debugging.

### 7. `not-fl3/macroquad` — minimal loop, trivial WASM, top-down examples
4.6k★ immediate-mode game library inspired by raylib.
https://github.com/not-fl3/macroquad

- Loop: `#[macroquad::main] async fn main() { loop { …; next_frame().await } }`;
  same code for PC/HTML5/Android/iOS; WASM = `cargo build --target
  wasm32-unknown-unknown` + `mq_js_bundle.js` + one `<canvas>` page
  ([README](https://github.com/not-fl3/macroquad)).
- `examples/asteroids.rs`: complete top-down ship — thrust along heading,
  linear friction (`-vel/100`), speed clamp, rotate-with-keys steering,
  screen wrap, circle-collision bullets/ship
  ([asteroids.rs](https://github.com/not-fl3/macroquad/blob/master/examples/asteroids.rs)).
  The thrust/friction/clamp shape ports directly to a car without lateral
  grip (add the drift split and it's a racer).
- Also ships `physics-platformer` helper crate and `snake.rs`/`platformer.rs`
  examples ([repo](https://github.com/not-fl3/macroquad)).
- No built-in 2D rigid-body physics — Rapier pairs via
  `rodneylab/macroquad-rapier-bevy-ecs`-style glue (secondary reference,
  not verified here).
- **Steal:** fastest native+WASM spike path; asteroids movement as the
  day-one car placeholder; WASM HTML shell.

### 8. `bones-ai/bevy-2d-shooter` — engine-free top-down at scale
99★ Bevy top-down shooter, no physics crate (README: kd-tree collisions,
100K enemies). https://github.com/bones-ai/bevy-2d-shooter

- `player.rs`: WASD/arrows → normalized `delta * PLAYER_SPEED` on
  `Transform`, `Idle`/`Run` state, damage via `PlayerEnemyCollisionEvent`
  reader, death → `NextState(GameState::MainMenu)`
  ([player.rs](https://github.com/bones-ai/bevy-2d-shooter/blob/main/src/player.rs)).
- Systems gated with `.run_if(in_state(GameState::InGame))` — the pattern
  for menu/countdown/racing states
  ([player.rs](https://github.com/bones-ai/bevy-2d-shooter/blob/main/src/player.rs)).
- **Steal:** state-gated systems for race flow; event-driven
  collision→effect plumbing without a physics engine.

### 9. `bevyengine/bevy` — fixed-step scheduling contract
https://github.com/bevyengine/bevy

- `examples/ecs/fixed_timestep.rs`: `FixedUpdate` schedule +
  `Time::<Fixed>::from_seconds(..)` configuration — the canonical
  fixed-timestep loop any Bevy port or greenfield uses
  ([fixed_timestep.rs](https://github.com/bevyengine/bevy/blob/main/examples/ecs/fixed_timestep.rs)).
- Every game repo above doubles as a Bevy API sample for the
  `App`/`Plugin`/`Query<ButtonInput<KeyCode>>` patterns.

## Gaps (not found as Rust+Bevy/macroquad primary sources)

- **Ghost/replay for racing:** no verified top-down racing ghost system;
  closest structural template is `bevy_garage`'s RL replay buffer (different
  purpose, same ring-buffer shape). Our TS ghost (input-frame recording) has
  no direct Rust precedent in this set.
- **Generated-track loaders in Rust:** BevyRaceTiles loads hand-made Tiled
  maps; nothing found parses generated track codes/polylines in Rust. Our
  track-code scheme would be novel — `track-positions.ron` in bevy_garage
  shows RON is an accepted track-data format, though.
- **2D Rapier car-controller tutorial:** gryazevichki/bevy_garage are 3D;
  the 2D top-down Rapier car (BevyRaceTiles) drives `Velocity` directly
  rather than using joints/motors — which conveniently is also the portable,
  deterministic-friendly approach.
