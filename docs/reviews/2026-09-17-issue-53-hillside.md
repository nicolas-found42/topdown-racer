# Issue #53: Hillside braking duel

Base `aaff675`; branch `issue/53-hillside-braking-duel`.

## Focused changes

Preserve #43's shipping Hillside geometry, width ramps, grid, finish gate,
handling and AI. Its broad 20–22-unit approach already supports competing
lines, and its connected right-left elbow at (120,76), (128,92), (144,112)
feeds the northern straight through (104,152). The sharp right turn reverses
the preceding left-hand arc; positioning through that reversal matters.

The remaining defect was readability, not insufficient vertices:

- Move the tire stack from (160,35) to (169,35); its old sprite covered road.
- Move the transition board from (119,65) to (117,62), beyond the kerb.
- Place each corner's kerbs on the outside of its **local turn**, not the
  outside of the entire CCW loop. Loop winding put right-turn strips across
  the exit leg: the failing regression found a kerb at (124.94,84.09), only
  0.8 units from that leg's centerline. The local cross-product sign fixes
  the cause without filtering out strips, changing physics, or redrawing Track.

## Three driven choices

Evidence: [trajectory plot](evidence/issue-53/trajectories.svg) and
[recorded samples](evidence/issue-53/trajectories.csv). These are actual Sim
trajectories, not hypothetical lines or Track vertex counts. A temporary
controller emitted manual `CarInput` (no AiDriver) toward authored waypoints;
this is not a claim of human keyboard laps. It used the existing Hillside
CornerChallenge start and timing gates, sampling every 16 fixed ticks.

| Choice | Entry / apex / transition tradeoff | Timed sector | Minimum speed |
| --- | --- | ---: | ---: |
| 0 | Brake early to 18 u/s on a neutral entry, then 20 through the bends: more entry margin, but gives up time before turn-in | 11.437500 s | 16.506586 u/s |
| 1 | Carry 24 toward the approach's inside at (110,4), tighten first apex via (143,23), then target 20: closes the inside defensive corridor and beats the neutral line in this fixture | 11.140625 s | 18.015087 u/s |
| 2 | Carry 24 wide via (110,-4), (151,22), then target 18 and a different reversal position via (122,78), (125,94): offers the other approach lane, but spends more distance/time setting up the reversal | 12.734375 s | 16.363245 u/s |

All three finish the sector with **zero wall / Grass ticks**. They converge
onto the same northern-straight exit and reach 26.37028 u/s at the exit gate;
these runs establish entry/apex/defence tradeoffs, not a fabricated exit-speed
advantage. Target speed rises to 29 u/s after the final bend. The plot's gray
ribbon is a schematic, not a second rendering of exact road boundaries.

## Centered-camera Computer Use review

A temporary native Bevy harness ran the real RacerGamePlugin on Metal at
1280×720 logical resolution (2560×1440 capture), with its centered camera.
It advanced the actual Sim in fixed ticks, followed the Player in Autopilot
through the sequence, and held selected frames for screenshot delivery.
Focus events were cleared to avoid another window pausing capture; physics
and AI were not replaced. Captures were opened and visually inspected.
The harness and other temporary probes were removed after verification.

All links below are final **post-fix** frames:

| Capture | Racing tick | Observation |
| --- | ---: | --- |
| [Grid](evidence/issue-53/grid.png) | 0 | Four Cars wholly on the eastbound straight, aligned and spaced behind the shared finish gate. Trailing Car is beyond the preceding bend. |
| [Approach landmarks](evidence/issue-53/approach-landmarks.png) | 132 | Original red/cream brake board at x=102 visible ahead of the centered Car at x=88.92, speed 22.83, before turn-in at x=120. The next board is partly covered by the minimap at this instant; the first gives an unobscured decision cue. |
| [Clear tire](evidence/issue-53/clear-tire.png) | 396 | Relocated tire entirely outside the road/kerb; pack follows the first arc. |
| [Right-entry landmark](evidence/issue-53/right-entry-landmark.png) | 520 | Transition board ahead and left of the centered Car approaching the right turn; 12.34 u/s at (142.26,62.12), a substantial reduction from the approach's 22.83. |
| [Direction change](evidence/issue-53/direction-change.png) | 640 | Car at (124.29,85.25), 17.78 u/s, reversing direction into the connecting leg. Corrected kerbs now border rather than cross the asphalt. |
| [Exit](evidence/issue-53/exit-straight.png) | 828 | Final bend opening toward the northern straight; 19.72 u/s. |
| [Pack overlap](evidence/issue-53/pack-overlap.png) | 100 | Deterministic outside passing fixture: blue and white side by side, orange blocks the other lane, all within clear road limits. GO text partly overlays blue, but its lateral corridor and separation remain visible. |
| [Finish](evidence/issue-53/results.png) | ~6030 | Blue first of four at the shared finish gate on the eastbound straight, LAP 3/3, BEST 00:29.59, SPEED 0 after finishing. |

Existing original procedural brake boards and kerbs retain their shared
palette materials; no external art, new palette colors or textures were added.
Sand and dark-grass Terrain Zones and scenery remain presentation-only; no
new grip or elevation mechanic is implied or implemented.

## Geometry, grid, identity and deterministic passing

`hillside_grid_and_scenery_leave_the_usable_road_clear` checks every grid Car's
heading, extents before the gate and beyond the preceding bend, non-overlapping
longitudinal spacing, width-ramp slope at most 0.1, and conservative full-sprite
clearance beyond the road plus 0.9-unit kerb. Track parsing still validates data.

No centerline/width/start-line changes: the shared finish gate and timing are
preserved. Existing `records::track_identity` hashes the entire typed Track
with FNV-1a, including geometry and authored presentation; these prop changes
therefore invalidate content comparisons without inventing a new identity path.

`hillside_braking_approach_allows_inside_and_outside_passes` runs a faster AI
behind a controlled slower Car, alternately blocking either side with a third
Car. Every tick is compared to an independent Sim; both complete before x=120
with zero Car/wall contacts or Grass ticks and 36 side-by-side ticks:

- Inside blocked (`blocked_side=1`): outside pass completes at tick 133.
- Outside blocked (`blocked_side=-1`): inside pass completes at tick 260.

This establishes room for both sensible lanes; it does not require an overtake
in every ordinary four-AI Race.

## Four-AI Race timing

6,222 tick calls including countdown (printed loop index 6,221); zero wall
contact and off-Track ticks. Every snapshot exactly matches a second independent
run. Geometry/handling unchanged, so timings correctly match #43 rather than
being retuned to force an obsolete constant.

| Grid slot | Standing lap | Flying lap 2 | Flying lap 3 |
| --- | ---: | ---: | ---: |
| 0 | 30.796875 | 29.593750 | 29.625000 |
| 1 | 32.437500 | 29.281250 | 29.953125 |
| 2 | 33.859375 | 29.640625 | 29.625000 |
| 3 | 34.937500 | 29.859375 | 29.437500 |

## Verification

All commands executed in this isolated worktree:

- `cargo test -p topdown-racer --lib track_geometry::tests::hillside_kerbs -- --nocapture`
  — initially failed at the crossing kerb above; passes after fix.
- `cargo test -p topdown-racer --lib track_geometry::tests -- --nocapture`
  — 11 passed after the local-turn fix.
- `cargo run -q -p topdown-racer-core --example hillside_probe`
  — three manual-input trajectories above (temporary example removed).
- `cargo run -p topdown-racer --example issue53_capture -- race`
  and `cargo run -p topdown-racer --example issue53_capture -- overlap`
  — native captures above (temporary example removed).
- `cargo fmt --all` then `cargo fmt --all -- --check` — exit 0.
- `cargo clippy --all-targets --all-features -- -D warnings` — exit 0.
- `cargo test --all-targets --all-features` — 216 tests passed.
- `cargo test -p topdown-racer-core --test hillside -- --nocapture`
  — 3 passed; both passing fixtures and clean Race as recorded above.

The only final Cargo notice is the pre-existing future-incompatibility notice
for dependency `block v0.1.6`. No remote CI was triggered. No changes to AI,
handling, geometry, physics, finish gate, or the presentation palette were needed.
