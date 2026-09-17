# Issue #47 — Dust + speed-streak capture evidence (2026-09-17, `issue/47-dust-speed-streaks` @ base `0c2d883`)

Native rendered runs of `target/debug/examples/issue47_capture <kind>` (the
`crates/topdown-racer/examples/issue47_capture.rs` probe on this branch). Each
run seeds the deterministic fixed-step sim with fixed grid Cars on the Hillside
circuit, waits at least 16 racing ticks plus 10 Update frames, then saves a full
2560x1440 screenshot from the live Bevy window (continuous update, no pause
overlay). The simulation continues during capture: speeds below describe grid
initialization, not constant velocities. The `-crop` files zoom to the Car region.

| File | Checklist item | Observed |
|---|---|---|
| `road-slow.png` (+`-crop`) | Dust on road: none; streaks at low speed: none | Cars initialized at 12 u/s on Road: clean charcoal, no puffs, no flank dashes |
| `road-fast.png` (+`-crop`) | Player-only streaks at high speed | Cars initialized at 40 u/s on Road; captured player HUD reads 28 u/s. Faint cream flank dashes behind the player Car only; rival Car shows none |
| `grass.png` (+`-crop`) | Dust off-road; none on road | Cars initialized at 12 u/s off-track on Grass: brown dust plumes behind both; the Road strip in frame stays clean |
| `gravel.png` (+`-crop`) | Dust on gravel | Cars initialized at 12 u/s on Gravel: dust puffs behind both Cars |

## How each run was produced

`cargo build -p topdown-racer --example issue47_capture` then one process per
kind, each exiting 0 on its own after saving its PNG (`BEVY_ASSET_ROOT` pointed
at `crates/topdown-racer` so the window loads the game assets). Kinds:

- `road-slow` — grid Cars initialized at 12 u/s on Road.
- `road-fast` — both Cars initialized at 40 u/s on Road. The headless regression
  separately holds all four Cars at 30 u/s to prove rival exclusion above cutoff.
- `grass` — Cars start off-track; the normal Track query supplies Grass.
- `gravel` — track surfaces overridden to Gravel.

Sim-level counterparts: `cargo test -p topdown-racer --lib dust_streak_tests`
(3 tests) assert the spawn rules headless — dust for moving Cars on
Gravel/Grass/off-track and none on Road, streak particles only for Car 0 above
the 26 u/s cutoff, and expiry after emission stops.

## Known limits of this capture set

- The GO! countdown banner overlaps the player Car; the flank dashes in
  `road-fast-crop.png` are still distinguishable from it and from the painted
  centre line (thinner, offset to both flanks).
- Streak dashes are deliberately faint (45% alpha CREAM); on some screens they
  may need the zoomed crops to read.
