# Match lap timing to the checker and preserve eligible Race results

## What to build

A player completes a lap by crossing the visible checker in the correct direction after valid ordered Track progress. Each Car receives its own finish, and a valid personal best survives immediately, even if the player leaves before the Race ends.

## Evidence and intended behavior

The reviewed Sample Circuit credited a lap near (0, 20.8), before the final corner, while the painted checker was near (10, 0). Lap completion currently uses a checkpoint proximity radius. The measured Race ended with completed-lap counts [3, 2, 2, 2] because any Car finishing stopped everyone. Best-lap persistence stores an unversioned number only when the Race ends.

Introduce one authoritative directional finish gate used by timing and presentation. Keep ordered route validation, but require a swept crossing of that gate for lap completion. Track finish status per Car. After the winner, allow a documented, visible 45-second simulation-time finish window; classify unfinished Cars explicitly at expiry. A Car takes the flag on its next proper crossing after the winner, with completed laps retained for lapped classification. Freeze its classification then, and remove its physical obstruction without a disruptive disappearance; a bounded non-colliding coast-out is sufficient.

## Acceptance criteria

- [ ] The rendered checker and simulation gate share the same Track-derived placement, including a start-line override and varying width.
- [ ] Entering checkpoint proximity before the checker cannot credit a lap; crossing the gate correctly after ordered progress credits exactly one.
- [ ] Swept crossings work at high speed. Reverse crossing, repeated oscillation, stationary overlap, skipped checkpoints, and lateral crossings outside gate bounds cannot grant extra laps.
- [ ] Standing-start elapsed time contributes to Race duration but is labelled separately from eligible flying-lap records.
- [ ] The winner finishing leaves the player's Race controllable; each subsequent proper finish is classified in order, including lapped Cars. A visible finish-window countdown ends with explicit DNF entries.
- [ ] Finished Cars cannot obstruct remaining competitors, change order, or accumulate additional laps.
- [ ] Results display the player, final classification, completed laps, valid times, and the reason a record is ineligible.
- [ ] Every eligible manual best is persisted at lap completion, keyed by Track content identity, handling version, and eligibility policy; Autopilot participation disqualifies a manual record.
- [ ] Legacy unversioned times are retained as legacy data or explicitly migrated, never silently compared against the new configuration. Malformed or missing records do not prevent a Race.
- [ ] Record writes replace data safely; write failures keep the in-memory best and show a nonfatal notice.
- [ ] Deterministic tests exercise the gate cases, winner/player/lapped/DNF finishes, equal-tick crossings with a stable tie policy, and restart. Persistence tests exercise eligible/ineligible updates and incompatible configurations.
- [ ] Computer Use verifies the visible crossing, losing player's finish, results, and a best surviving menu exit/relaunch. Required workspace checks pass.

## Boundaries and design constraints

This is one complete timing-to-results slice. Do not add sectors, ghosts, leaderboards, or online storage. Define lap eligibility narrowly around ordered progress, direction, full manual control, and explicit invalidation; do not invent a full motorsport penalty system. Record the finish-rule and record-identity changes in the domain documentation. Existing lap-time pins must change for the corrected gate with an explained new baseline, rather than retaining the incorrect old timing.

## Blocked by

- Draft 01 — Manual driving by default with visible Autopilot ownership (supplies authoritative participation provenance).

## Research basis

[Forza's clean-lap Rivals targets](https://forza.net/events/featured-rivals-bridgestone) and [Dust Racing 2D's configuration-specific records](https://github.com/juzzlin/DustRacing2D) motivate explicit record eligibility. The measured early lap event is a local correctness defect.
