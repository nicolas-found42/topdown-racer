# Open-ticket implementation checkpoint

Starting HEAD: `3bd95b97e0b1fa5aa89fc8d42034f81b6218a7e7`.
The workspace was already dirty when implementation began; existing changes
were preserved. Nothing was committed, published, or closed in GitHub.

## Dependency order

#49 → #51 → #50 → #54 → #43 → #53 → #56 → #52 → #45 → #46 → #47 → #55 → #57.
Parent specs #3 and #32 are completion checklists, not duplicate features.
Independent Track/FX work advanced while test-boundary confirmation was pending.

## Current implementation

- Existing Manual/Autopilot ownership, pace presets, and passing changes
  were inspected and their targeted regressions passed.
- Timing and checker rendering now use `Track::finish_gate`. A regression
  first reproduced the old premature lap event near `(0,20.8)`, then passed
  at the checker near `(10,0)` on the original fixture.
- Rolling grids cannot gain an untravelled lap by starting near the finish.
- Per-Car finish status, a 45-second simulation finish window, DNF at expiry,
  and frozen completed-Race state are implemented. Finishers coast without
  physical collisions, and results show player identity and completed laps.
- The shell loads the authored Hillside Circuit. Its deterministic four-AI
  Race finishes at loop index 6221 including countdown, with zero wall and
  off-Track ticks. Flying laps range from 29.28125 to 29.953125 seconds.
  The old Sample Circuit remains a regression fixture.
- Skid decals have a pure bounded stream and an engine pool (2048 marks).
  Brake lights use brake echo; smoke uses handbrake; dust uses physical
  surface and speed; streaks require player speed above 26 world units/s.
  Airborne FX share 128 slots, with 24-tick puffs and 8-tick streaks.
  Effects reset on Race entry. CC0 donor license and import commands ship
  alongside processed 16×16 smoke/dust assets.

## Verification

- `cargo check --workspace --all-targets`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-targets --all-features`: 162 tests passed.
- `git diff --check`: passed.
- Computer Use verified neutral Manual after countdown, visible Autopilot
  and assisted-lap labels, a losing player's three-lap finish/results on the
  original fixture, and the Hillside grid and an active Race with assets.
- A temporary macOS verification bundle required an explicit `BEVY_ASSET_ROOT`
  when launched outside Cargo. No packaging change was made to the repo.
- FX event captures, full Hillside Race captures, sustained manual comparison,
  all window sizes, and audio audition remain incomplete.

## Remaining work and required input

All-ticket implementation is **not complete**. See the separate Standards and
Spec checkpoint reports for the remaining gaps. In particular, the existing
unversioned best-lap file is still shared with old timing/Track data; record
migration and separation must land before this becomes a release candidate.

The installed TDD skill explicitly requires the user to confirm new public
test boundaries. An asynchronous question is pending for human input
adaptation, camera/awareness geometry, race/practice lifecycle commands,
and configuration-aware local records. No answer has been received at this
checkpoint. Existing simulation/parser/geometry/bake/skid boundaries were
already approved by the parent specs and were used for the work above.
