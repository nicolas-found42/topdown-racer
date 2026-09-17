# Issue #43: Hillside Circuit verification

Base: `0c2d883c0ae9d8b9019ec85975b1a8bede045791`.
Branch: `issue/43-hillside-circuit`.

## Loaded Track

The existing `RacerGamePlugin` loads `HILLSIDE_CIRCUIT`, not `SAMPLE_CIRCUIT`.
The canonical data lives in `crates/core/data/tracks/hillside-circuit.json`,
embedded by `crates/core/src/track.rs`; no duplicate shell asset is needed.
It authors a closed 18-segment circuit, per-vertex widths of 18–22 units,
the `hillside` theme, start segment 0, eight scenery props (three braking
boards, four trees, one tire stack), and sand/dark-grass Terrain Zones.
Sample Circuit remains a deliberately separate simulation test fixture.
No geometry or AI retuning was necessary.

## Clean four-AI Race and determinism

`cargo test -p topdown-racer-core --test hillside -- --nocapture` passes.
It drives all four Cars with AI (explicit player Autopilot), compares every
snapshot against a second independent simulation, and requires every Car to
finish with zero wall-contact ticks and zero Grass ticks across the whole Race.
The run finishes after 6,222 tick calls (the printed loop index is 6,221),
including the countdown.

| Grid slot | Standing-start lap (s) | Flying lap 2 (s) | Flying lap 3 (s) |
| --- | ---: | ---: | ---: |
| Player / 0 | 30.796875 | 29.593750 | 29.625000 |
| 1 | 32.437500 | 29.281250 | 29.953125 |
| 2 | 33.859375 | 29.640625 | 29.625000 |
| 3 | 34.937500 | 29.859375 | 29.437500 |

The previously printed-only pace is now an assertion: every Car has all three
lap times, and each flying lap is within 10% of 30 seconds (27–33 seconds).
This defends the authored circuit duration against layout or pace regressions
without pinning incidental floating-point values. Standing-start laps are
excluded from that band because grid spacing and acceleration make them slower.
Existing snapshot equality still verifies deterministic replay at each tick.

## Full-race visual evidence

A throwaway shell harness (`RacerGamePlugin`, player Autopilot forced on,
sim force-resumed each frame so focus loss cannot stall it) captured a full
three-lap race through the real renderer into
`docs/reviews/evidence/issue-43/`:

| File | Moment (racing ticks) | Captured content |
| --- | --- | --- |
| `01-grid.png` | 0 | Countdown "3", four-car grid behind the checker start gate |
| `02-lap-one.png` | 640 | Lap 1, first hairpin, sand zone, guardrail + brake boards |
| `03-lap-two.png` | 2,400 | Lap 2, dark-grass zone, tire-stack prop, trees |
| `04-lap-three.png` | 4,200 | Lap 3, hairpin exit, guardrail kink, tree prop |
| `05-results.png` | results | "RACE FINISHED", 4/4 cars 3/3 laps, per-lap AUTO times |

The harness counted per-tick `wall_contact` and `Surface::Grass` on every
snapshot of the rendered race: 0 and 0. The Results screen lap times are
identical to the headless table above (30.80 / 32.44 / 33.86 / 34.94
standing, 29.28–29.95 flying), confirming the renderer ran the same
deterministic simulation. The harness was deleted after the run; it is not
part of the branch.

## Validation and CI

All commands ran inside this isolated worktree:

- `cargo fmt --all -- --check` — exit 0.
- `cargo clippy --all-targets --all-features -- -D warnings` — exit 0.
- `cargo test --all-targets --all-features` — exit 0; 196 tests passed.
- `cargo test -p topdown-racer-core --test hillside -- --nocapture` — exit 0;
  clean completion and lap table above.

Cargo reports a pre-existing dependency future-incompatibility notice for
`block v0.1.6`; there are no project Clippy warnings.

[Base Rust CI run 35196199576](https://github.com/nicolas-found42/topdown-racer/actions/runs/35196199576)
passed for the exact base SHA, including formatting, Clippy, all tests and the
bounded native-app smoke step. Remote CI was not triggered for this local
branch: campaign instructions explicitly prohibit pushing. The complete local
format/Clippy/test equivalent passed with the new lap-time assertion; it must not
be represented as hosted CI on the final branch commit.
