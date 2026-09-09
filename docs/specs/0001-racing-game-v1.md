# Racing Game v1 — native Rust top-down racer with AI opponents

## Problem Statement

The previous TypeScript browser racer was deleted on purpose; the project restarts as a native Rust game. There is currently nothing runnable: no project scaffold, no driving, no Track, no Race. The player wants a desktop top-down racing game in Rust that feels like a real racer — speed you carry through corners, rivals to beat — not an on-rails arcade toy.

## Solution

A native desktop (Bevy) top-down racer: one hand-authored closed Track, the player's Car plus 3 AI Opponents, 3-lap Races with live positions and lap timing, custom 2D physics (lateral grip with slip, surfaces, walls), keyboard controls, a minimal shell (menu → countdown → race → results) with local best-lap persistence.

## User Stories

1. As a player, I want to launch the game into a menu, so that I can start a Race or quit.
2. As a player, I want the menu to show my saved best lap, so that I have a target to beat.
3. As a player, I want a countdown before the Race starts, so that I can prepare to accelerate.
4. As a player, I want to accelerate with a key, so that I can build speed down straights.
5. As a player, I want to brake, so that I can slow for corners.
6. As a player, I want to reverse, so that I can recover after spinning or hitting a wall.
7. As a player, I want to steer left and right, so that I can hold a racing line.
8. As a player, I want a handbrake, so that I can provoke a Drift into tight corners.
9. As a player, I want the Car to carry speed through corners with lateral grip and slip angle, so that cornering feels physical rather than on-rails.
10. As a player, I want the off-Track surface to be slower and slippier than road, so that staying on the Track matters.
11. As a player, I want wall contacts to bleed speed and bounce the Car, so that crashes punish without hard-stopping the Race.
12. As a player, I want weight transfer under braking and acceleration to affect grip, so that how I enter a corner changes handling.
13. As a player, I want the camera to follow my Car without rotating the world, so that I always know my orientation.
14. As a player, I want a fixed zoom with letterboxing, so that speed and spacing read consistently at any window size.
15. As a player, I want a lap counter, so that I know my progress through the 3-lap Race.
16. As a player, I want a live lap timer, so that I can gauge the lap I am driving.
17. As a player, I want my best lap shown during the Race, so that I can compare the current lap against it.
18. As a player, I want live Race positions (1st–4th), so that I know where I stand against the AI Opponents.
19. As a player, I want a speed readout, so that I can sense acceleration and top speed.
20. As a player, I want 3 AI Opponents racing the same Track, so that I compete rather than time-trial.
21. As a player, I want AI Opponents to follow a racing line and slow for corners, so that they drive believably.
22. As a player, I want AI Opponents to hold a fixed skill level with no rubber-banding, so that wins and losses feel earned.
23. As a player, I want AI Cars to collide with walls, the Track surface, and me like any other Car, so that the Race feels fair.
24. As a player, I want the Race to end after 3 laps, so that there is a clear finish.
25. As a player, I want a results screen with finishing order and lap times, so that I can review my performance.
26. As a player, I want to restart a Race immediately, so that I can retry without leaving to the menu.
27. As a player, I want my best lap saved locally on this machine, so that it persists across sessions.
28. As a player, I want ESC to leave a Race for the menu, so that I can exit cleanly.
29. As a player, I want engine and skid audio (stretch), so that driving feels alive.
30. As a developer, I want the simulation to be a plain-Rust deterministic fixed-step module with no Bevy dependency, so that physics, AI, laps, and race flow are testable headlessly.
31. As a developer, I want Track files validated by a dedicated parser seam, so that malformed Track data fails loudly with a useful error.
32. As a developer, I want tests that drive the simulation seam with scripted input streams, so that handling, surfaces, walls, lap counting, race flow, and AI behavior are pinned by observable snapshots.
33. As a developer, I want CI to run `cargo fmt`/`clippy`/tests, so that the Rust pipeline replaces the retired npm workflows.
34. As a developer, I want the Bevy shell to hold no game rules, so that rendering/input glue can change without touching behavior under test.
35. As a developer, I want a single cutover commit that removes the TypeScript game, its workers, the legacy ADRs, the legacy research note, and the npm/vite/wrangler workflows from the repository, so that origin/main agrees with the fresh Rust restart and no future push runs the retired pipeline.
36. As a developer, I want the engine and physics posture recorded as ADR-0001 (Bevy + custom physics, no physics crate), so that the decision is binding and future sessions do not re-litigate it.

## Implementation Decisions

- **Fresh start.** The TypeScript game, its ADRs, spec, and issue #1 are retired. Rust workspace scaffolded from scratch; this spec is the single source of truth.
- **Engine: Bevy.** Owns window, input, rendering, audio, schedules. Chosen over macroquad for the scheduler (`FixedUpdate`), input/audio subsystems, and room to grow (see ADR-0001 and research notes).
- **Simulation module (seam 1).** Plain Rust, zero Bevy types. Constructors take a `Track` and Car setup; the tick function advances the world one fixed step (64 Hz) from a set of per-Car inputs (player-mapped + AI-generated) and returns a snapshot: race phase, laps, lap times, positions, per-Car pose/velocity/surface/contacts. All game rules live here.
- **Custom physics, no physics engine.** Longitudinal: throttle/brake/reverse force, drag, rolling resistance. Lateral: grip with slip angle; handbrake reduces lateral grip to provoke Drift. Light weight transfer under braking/acceleration biases front/rear grip. Walls: bounce with speed loss. Surfaces: road vs off-Track grip and drag factors. Rapier/Avian deliberately not used in v1 (ADR-0001).
- **Track model (seam 2).** A hand-authored data file (data-driven, one circuit in v1): center polyline, width, surface segments. The parser validates geometry (closed loop, positive width, known surfaces) and returns a typed `Track` or a descriptive error. Additional hand-made Tracks drop in as data; generated Tracks are future work.
- **AI Opponents.** Three Cars driven inside the simulation: waypoint racing line, corner slowdown from upcoming curvature, small lateral offsets so they don't stack, fixed skill. They emit the same per-Car inputs as the player; no rubber-banding.
- **Race flow.** Phases: countdown → racing → finished. 3 laps, 4 Cars, live positions, per-lap timing, best lap. Lap validation uses ordered progress along the Track so cuts and reversals don't count.
- **Camera/render.** World-aligned orthographic camera following the player Car, fixed zoom, letterboxed; render interpolates between fixed steps; the shell renders snapshots only.
- **Input.** Keyboard only: arrows/WASD throttle-brake-steer, Space handbrake, reverse when stopped and braking, ESC to menu. The shell maps keys to the simulation's input struct; the simulation never reads devices.
- **Persistence.** Best lap saved to a local file in the user config directory; menu reads it. No network.
- **Audio (stretch).** Engine loop pitch-by-speed and skid cues when Drifting, after driving feels right.
- **CI.** Replace the retired npm workflows with a cargo pipeline: fmt check, clippy, tests (both seams).

## Testing Decisions

- **Two seams, agreed with the user:** (1) the headless simulation tick — Track + inputs in, snapshot out; (2) the Track parser — file text in, typed `Track` or error out. Nothing else gets a seam; the Bevy shell is smoke-tested manually.
- **A good test** drives a seam and asserts only externally observable outcomes (snapshot fields, error variants): scripted input streams produce expected pose/velocity/lap/position/phase transitions; malformed Track text produces a specific error. No assertions on internals, no physics-constant peeking.
- **Determinism:** fixed 64 Hz step, seeded everything; identical input streams must produce identical snapshot sequences — this is asserted directly, because AI learning and difficulty tuning later depend on replayability.
- **What gets covered where:** simulation tests — grip/drift behavior (handbrake raises lateral slip), surface effects, wall speed loss, lap counting including cut attempts, race phase transitions, AI progress and corner slowdown, positions; parser tests — unclosed loop, bad width, unknown surface, malformed data.
- **Prior art:** none in-repo (fresh Rust codebase). Patterns mined for the shape of these tests live in the research notes: the ordered-checkpoint lap counter, the FixedUpdate chained physics step, and the fixed-step determinism-with-hashing example.

## Out of Scope

- Multiplayer, netcode, any server or leaderboard.
- WASM/web and mobile builds; native desktop only.
- Gamepad and remappable controls.
- Generated Tracks, Track codes/sharing, a Track editor.
- Learning/optimizing-line AI (v1 AI is waypoint + fixed skill by decision).
- Rubber-banding.
- Ghost/Run input-replay recording.
- Full audio pass (engine/skid cues only as the stretch story).

## Amendment: Track model width ramp (2026-09-09, spec #32 / ticket #39)

The Track model above ("center polyline, width, surface segments") gained a
per-vertex **width ramp**: road width interpolates linearly between polyline
vertices, expanding into straights and contracting into corners. The global
`width` field is gone — the parser always resolves a per-vertex `widths`
array (a single authored `width` still loads, as a uniform ramp). Walls,
surface classification, checkpoints, AI planning, and render geometry all
consume the local interpolated width. See the pixel-art presentation spec
(issue #32) and ADR-0003 for the presentation-side decisions.

## Further Notes

- Research that informed the stack and steal-lists: `docs/research/rust-stack-awesome.md`, `docs/research/rust-stack-github.md`, `docs/research/rust-stack-docs.md` (closest prior art: a Bevy + rapier2d top-down racer's drift model, and an engine-free FixedUpdate drift formulation).
- Engine and physics decision recorded in ADR-0001 (Bevy over macroquad; custom physics over a physics crate).
- Vocabulary lives in `CONTEXT.md` (Track, Car, Race, AI Opponent, Drift). Retired vocabulary from the deleted TS game — Track Code, Run, Ghost, Leaderboard — is intentionally dead; do not reuse it.
- Cutover state: the TypeScript-era working-tree deletions, legacy ADR/spec removal, and retirement of the npm/vite/wrangler workflows land together in the cutover commit (story 35); stale specs #1 and #2 are closed with restart notes; CI is rebuilt as the cargo pipeline (story 33).
