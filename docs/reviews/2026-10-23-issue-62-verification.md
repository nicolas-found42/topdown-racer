# Issue #62 — four-wheel handling and per-wheel feedback

Base: main after PR #63 (`3a66640`). The user explicitly changed the public rate
from 64 to 120 Hz during implementation. Tire forces use four internal substeps
at 480 Hz. See ADR-0005 for the decision and timer/record consequences.

## Handling evidence

`cargo test -p topdown-racer-core --test vehicle_handling -- --nocapture`
drives the public simulation seam, including:

- Gripped corners across 5/10/20 u/s and small steering demands, with light
  throttle and either assist setting: no wheel beyond peak, no Drift, no assist
  intervention. A neutral rolling coast loses speed.
- An over-speed full-lock entry: front saturation, rear grip, no Drift reward.
- Rear-only handbrake lockup; rear-only driven-wheel spin; all four tires past
  peak during an uncontrolled full slide. Tire marks are not gated on Drift.
- A deliberate handbrake entry followed by countersteering: 315 consecutive
  Drift ticks (2.625 seconds) remain reachable with assist on.
- A snap-oversteer fixture: approximately 1.80 radians total rotation with assist
  versus 3.59 without. The same input stream reaches exactly the same steering
  demand and front wheel angle in both runs. Corrective yaw is bounded at
  3500 N m, dissipative, and absent when rear lateral grip has returned.
- Trail braking moves load to the front and tightens the line relative to power;
  lateral load transfer moves load outward while preserving total vehicle weight.
  Road, Gravel, and Grass produce ordered turning outcomes in the matched fixture.
- Standstill remains exact for ten seconds, reverse engages after a quarter
  second (30 ticks), release self-aligns faster than turn-in, and an offset wall
  contact applies yaw while losing speed and receiving no Drift credit.
- Four forward gears are reached, RPM drops after shifts, and the straight-line
  terminal speed measures 29.2343 u/s. Engine audio consumes RPM rather than speed.

Private solver tests additionally check tire force rising/falling around peak,
combined-force saturation, and load conservation. Retained lifecycle, gate,
records, input ownership, replay, encounter, passing, and opponent-pace tests
exercise these rules with the new model. Old point-mass canned trajectories
were replaced rather than preserving a second physics mode.

The raw handling output is in `issue-62/handling.txt`.

## Clean four-Car Races and cost

Reproduce with:

```sh
cargo run --release -p topdown-racer-core --example issue62_probe -- field
```

The final probe completed all three laps on Sample and Hillside, with player
assist both off and on, at **zero wall contacts, zero Grass ticks, and zero Car
contacts** in all four runs. Opponent assist stays off. Raw laps and runtime
measurements are retained in `issue-62/clean-field.txt`.

Sample flying laps are approximately 35.8–36.7 seconds; Hillside flying laps are
47.6–50.0 seconds. These replace the old grip-damper pace pins; Track geometry
was not changed. The tests retain the strict clean-field bar and a measured
46–51-second Hillside flying-lap band. Touring/Club/Race solo ordering remains
covered by repeated deterministic runs on both Tracks.

The release probe averages approximately 12–22 microseconds per four-Car tick
on this development machine, against an 8333-microsecond 120 Hz tick interval.
This measures core physics, AI, collisions, and Race rules. It excludes Bevy,
rendering, audio, and OS scheduling, and is not an end-to-end frame-rate claim.

AI changes include tire-grip planning, pure-pursuit steering, perceived axle-slip
countersteering, cornering/launch traction budgeting, stopping-distance-aware
following, and seven-unit corridor reservation before turn-in. A settled
follower can pass a slower Car without first closing dangerously. A stationary
queue waits for its front Car to clear; passing a stationary obstruction uses
the full low-speed turning radius instead of a high-speed aim distance. The
parked-player Sample regression still requires all three AI Opponents to lap.

## Native per-wheel effects

Reproduce from repository root, with the crate asset root:

```sh
BEVY_ASSET_ROOT="$PWD/crates/topdown-racer" cargo run -p topdown-racer --example issue62_capture -- ordinary
BEVY_ASSET_ROOT="$PWD/crates/topdown-racer" cargo run -p topdown-racer --example issue62_capture -- handbrake
```

The example uses real physics, production rendering/effects, and the ordinary
80-by-45 view at native pixel density. It asserts rendered decal visibility,
records per-wheel slip counts, and waits for a nonempty capture before exiting.
It clears the rolling fixture's unrelated GO overlay so the Car and tire paths
can be inspected.

- `issue-62/ordinary.png`: gripped corner, 0 visible marks, all four wheel-slip
  counters zero.
- `issue-62/handbrake.png`: two parallel rear-wheel trails; 66 visible marks,
  front counters zero and both rear counters 101 at the capture. The Car has
  released the handbrake by this frame; its marks persist.

`issue-62/captures.txt` retains the capture measurements. Geometry tests also
verify an independently slipping front wheel's rotated contact location and
feed real gripped/handbrake simulation snapshots through the decal seam. The
2048-mark budget, oldest-first eviction, Recovery preservation, restart reset,
and bounded airborne effects remain covered.

Audio-cue and mixer tests verify continuous wheel excess, silence with grip,
release decay, surface differentiation, and RPM-based pitch. Native captures
and PCM/contract tests are not evidence of a human listening session.

## Records, practice, and timing

Race and Corner Practice keys use handling2 plus assist identity. Old handling1
entries remain in the same records-v2.json file and cannot become new targets.
Storage format is unchanged. Tests cover on/off separation, preservation across
save/load, restart, and menu-to-practice transitions. Changing policy clears the
cached eligible best and invalidates a mixed-policy active lap.

Corner Practice uses the same vehicle model. The scripted late-braking fixture
now holds throttle only to x=105 rather than x=136; the old fixture ran off the
Track under the new grip ceiling. The new measured approaches both finish:
tidy 25.050001 s / 18.621687 u/s, late 23.975 s / 18.619408 u/s. These are synthetic
inputs, not human preference evidence. See `issue-62/practice.txt`.

Countdown, pause preparation, reverse engagement, Recovery cooldown, finish
window, steering response, particle lifetimes, and audio envelopes retain their
time durations at 120 Hz. AI reverse recovery is two seconds under the new
acceleration envelope. The late-approach encounter starts far enough away for
real tire-limited braking; its old 28-to-5 u/s approach in 25 units was physically
impossible. Passing checks still require actual overtakes without contact.

Human keyboard feel and acoustic audition remain unmeasured. The scripted
handling tests and native renders establish the stated simulation and effects
contracts without claiming those subjective results.

## Workspace checks

`cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
`cargo test --all-targets --all-features` (251 tests across 33 targets), and
`git diff --check` passed. The existing dependency future-incompatibility notice
for block 0.1.6 remains; there are no project Clippy warnings.
