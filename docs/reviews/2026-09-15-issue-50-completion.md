# Issue #50 — completion checkpoint

**Issue:** [#50 Match lap timing to the checker and preserve eligible Race results](https://github.com/nicolas-found42/topdown-racer/issues/50)
**Repository:** `nicolas-found42/topdown-racer`
**Base for this session:** `main` @ `54cfdc1` (clean tree; the gameplay batch for
#49–#57 was merged earlier in PR #59, plus its follow-up fixes)

**Status: implemented and verified locally.** The merged implementation already
satisfied the behavioural criteria; the residual gaps were missing evidence — the
deterministic gate cases the issue names by hand, the persistence wiring, and the
live Computer Use pass. Those are closed. One workflow gate remains open: see
**Dependency gate** below.

## What was already in place vs. what this session added

The finish-gate slice (`Track::finish_gate()` shared by timing and the rendered
checker, per-Car `FinishStatus`, the 45 s `FINISH_WINDOW_TICKS` window, DNF at
expiry, frozen classification with a non-colliding coast-out, and the
content/version/eligibility-keyed `records-v2.json`) shipped in PR #59.

This session added **tests, evidence, and one small testability change**
(the only production edit is the save-path field described below; the rest of the
working-tree diff is insertions-only test code):

- `crates/core/src/simulation.rs` — 5 internal tests + a shared ordered-checkpoint
  walk helper: high-speed single-tick swept crossing, oscillation across the gate,
  stationary overlap on the gate, lateral crossing outside the gate width (with a
  positive control), finished Cars coasting through a racing rival without
  contact/order/lap changes, and the standing-start/flying-lap manual-record rule.
- `crates/core/tests/finish_gate.rs` — a gate sweep with skipped checkpoints
  cannot credit a lap; an Autopilot lap records a time but never a manual record.
- `crates/topdown-racer/src/records.rs` — `LocalRecords` now carries its save path
  (`from_path`, defaulting to `record_path()`), so `persist_completed_laps` is
  driven by tests against a temp file instead of the user's config dir. Default
  behaviour is unchanged. Two tests: an eligible manual best reaches the file and
  the menu target at the lap-completion tick; an assisted race writes no record.
- `crates/topdown-racer/tests/records.rs` — incompatible configurations keep
  separate records (Track identity differs for different content and is stable
  across parses; `race_key` splits on the steering response, the policy dimension
  that varies today — its handling-version and eligibility fields are constants);
  an ineligible lap can neither create nor overwrite a record.
- `crates/topdown-racer/src/menu.rs` — the HUD announces the finish window while
  the Race is still running.
- `.scratch/issue-50-computer-use/` — 11 live captures + README mapping each to
  the checklist.

## Acceptance matrix

| Criterion | Result | Evidence |
|---|---|---|
| Checker and gate share one Track-derived placement (start-line override, varying width) | Pass | `track_geometry.rs` builds the checker from `track.finish_gate()`; `start_line_moves_to_the_overridden_segment`, `start_line_spans_the_local_width`, `overridden_start_line_shifts_grid_spawn`, `overridden_start_line_shifts_lap_origin_and_completion` |
| Checkpoint proximity before the checker cannot credit; a correct crossing credits exactly one | Pass | `completed_lap_requires_crossing_the_painted_checker`, `checkpoint_radius_follows_local_vertex_width`, `high_speed_swept_crossing_credits_exactly_one_lap` |
| High-speed sweep; reverse/oscillation/stationary/skipped/lateral cannot grant extra laps | Pass | `directional_gate_accepts_swept_forward_crossings_only_inside_its_width`, `oscillation_across_the_gate_never_grants_extra_laps`, `stationary_overlap_on_the_gate_never_credits_a_lap`, `lateral_crossing_outside_the_gate_width_never_credits_a_lap`, `skipped_checkpoints_cannot_credit_a_lap_at_the_gate`, `cut_attempts_and_wrong_way_progress_never_increment_the_lap_counter` |
| Standing-start time counts for the Race but is labelled separately from eligible flying laps | Pass | `standing_start_lap_is_never_a_manual_record`; results line `Lap 1: standing start (Race time only)`; capture 06 shows `BEST 00:30.80` (lap 1) while the menu target stays `00:31.34` |
| Winner leaves the player controllable; each later proper finish classified in order, incl. lapped; visible countdown ends in DNF | Pass | `winner_leaves_the_manual_player_a_finish_window`, `lapped_player_takes_flag_on_next_valid_crossing_after_winner`, `driving_summary_shows_the_finish_window_countdown`; captures 07/08 (countdown visible) and 04 (explicit DNF rows) |
| Finished Cars cannot obstruct, change order, or accumulate laps | Pass | `finished_cars_coast_without_extra_laps_or_orders` (asserts a genuine overlap happened, then no contact on either Car, frozen laps/status/position); `resolve_car_collisions` skips non-racing Cars |
| Results show player, classification, laps, valid times, and the ineligible reason | Pass | `results_show_correct_finishing_order_and_per_car_lap_times`, `assisted_lap_stays_labelled_in_manual_results`; captures 04 and 09 |
| Eligible manual best persisted at lap completion, keyed by Track content / policy; Autopilot disqualifies | Pass | `record_key`/`race_key` (`race:<track-id>:handling1:manual-flying-gate2:<steering>`), `autopilot_lap_is_ineligible_for_a_manual_record`, `an_eligible_manual_best_reaches_the_record_file_at_the_lap_completion_tick`, `an_assisted_race_never_writes_a_record_file`; `persist_completed_laps` runs in `FixedUpdate` right after `step_simulation` |
| Legacy unversioned times retained, never compared; malformed/missing records do not block a Race | Pass | `malformed_records_and_failed_writes_keep_an_eligible_session_best`; `best_lap.txt` (`20.296875`) sits beside `records-v2.json` and is never read by the record path |
| Writes replace safely; failure keeps the in-memory best and shows a nonfatal notice | Pass | `RecordBook::save` uses a PID-suffixed temp file then `rename`, removing the temp on failure; `malformed_records_and_failed_writes_keep_an_eligible_session_best`; `LocalRecords.notice` renders on the menu and results |
| Deterministic tests for gate cases, winner/player/lapped/DNF, equal-tick tie, restart; persistence eligible/ineligible + incompatible configurations | Pass | The tests listed above plus `equal_tick_finish_uses_stable_grid_order`, `esc_to_menu_and_restart_runs_a_fresh_countdown`, `restart_produces_a_clean_fresh_race`, `incompatible_configurations_keep_separate_records` |
| Computer Use verifies visible crossing, losing player's finish, results, best surviving menu exit/relaunch; workspace checks pass | Pass | `.scratch/issue-50-computer-use/` captures 01–11 and the README table |


## Dependency gate

#50 carries GitHub's native `blockedBy` edge on **#49, which is still OPEN**
(`gh issue view 50 --json blockedBy`). The *code* prerequisite — authoritative
Manual/Autopilot participation provenance — is merged and exercised here
(`DrivingMode`, `request_player_mode`, `current_lap_assisted`, the `LAP ASSISTED`
label), which is what let this slice be built and verified. The tracker gate is
not: per `docs/agents/issue-tracker.md` a ticket is unblocked only when every
blocker is closed, so **#50 must not be marked done before #49 closes.**

## Validation actually executed

| Command | Result |
|---|---|
| `cargo test --workspace --all-targets --all-features` (baseline, before edits) | green |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean (only the pre-existing `block v0.1.6` future-incompat notice from a dependency) |
| `cargo test --workspace --all-targets --all-features` (final) | **192 passed, 0 failed** |
| `cargo test -p topdown-racer-core --lib` | 72 passed (was 67) |
| `cargo test -p topdown-racer-core --test finish_gate` | 8 passed (was 6) |
| `cargo test -p topdown-racer --lib` | 74 passed (was 71) |
| `cargo test -p topdown-racer --test records` | 4 passed (was 2) |
| `git diff --check` | clean |

Unavailable/not claimed: human driving-feel and audio judgments (explicitly
out of scope here — #51 and #55 own those), and a real listening session.

## Decisions and notes

- **No dependency added.** The work needed none; the gate maths is a few lines in
  `topdown-racer-core` and the records are `serde_json` over a `BTreeMap`.
- **One production line of behaviour-preserving surface was added** for
  testability: `LocalRecords` carries the path it saves to. Without it, the
  persistence wiring could only be tested by writing the user's real config dir.
  A regression there (reading `best_lap_time` instead of `best_manual_lap_time`,
  or dropping the eligibility flag) would have passed every other test in the
  repo while writing Autopilot laps into the user's records.
- **Ineligible races cannot move the record file.** Verified live: after three
  Autopilot/Manual-stationary races the on-disk `records-v2.json` was byte
  identical, because `RecordBook::consider(.., eligible=false)` short-circuits.
- **Swept-crossing speed in the new unit test is synthetic.** One 64 Hz tick at
  200 u/s cannot clear the 10-unit approach on the Sample Circuit, so the fixture
  uses 2000 u/s. The property under test — a crossing detected from previous and
  current pose, not from overlap — is the same one the gate ships.
- **`Race` state, not `Results`, owns the countdown.** The finish-window text comes
  from `menu::driving_summary`, which is why the HUD test lives in `menu.rs`.

## Remaining risks / next action

- The window is a fixed 45 s of simulation time; the captures show it expiring
  into DNF (04) and being met with time to spare (09). No boundary flake was seen.
- Nothing here is published, pushed, or closed. Preparing the PR/issue summary is
  the next action; closing #50 is **not** recommended until a reviewer accepts the
  capture set, though every mandatory criterion above now has executed evidence.
