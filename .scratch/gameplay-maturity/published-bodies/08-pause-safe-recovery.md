## What to build

The player can interrupt a Race without abandoning it, resume deliberately, or request a safe recovery when stranded. Recovery returns the Car to a valid position without granting checkpoint progress or a ranked lap, and without placing it into a rival.

## Brainstormed addition

Treat interruption and mistakes as part of the racing loop. ESC currently returns directly to the menu; reverse exists but is not discoverable. A pause-and-recovery flow reduces frustration while preserving the value of learning to brake and recover normally.

## Acceptance criteria

- [ ] ESC during countdown/racing opens a clear Pause menu with Resume, Restart, Return to Menu, and Recover Car where eligible.
- [ ] Pause freezes the entire simulation: Cars, countdown, lap clocks, finish window, AI memory evolution, and gameplay FX. Audio pauses or mutes appropriately.
- [ ] Window focus loss pauses the Race. Focus restoration does not automatically resume; stale held keys are cleared before deliberate resumption.
- [ ] Resume includes a brief visible preparation countdown while Race time remains frozen, and cannot create an accumulated-time jump.
- [ ] Reverse controls are explained before recovery is needed. Recover Car explains that the current lap becomes ineligible for a personal best.
- [ ] Recovery is available only below a documented low-speed threshold or after a documented stranded interval, preventing use as a high-speed shortcut.
- [ ] Recovery selects a valid road position at or behind the last validated progress, with forward heading and zero speed. It cannot increment laps/checkpoints or improve classification by teleportation.
- [ ] The destination respects Car clearance. If occupied, recovery waits or reports that it is unavailable; it never spawns overlapping Cars or silently grants collision immunity.
- [ ] A deterministic recovery event invalidates the current lap for records, while a subsequent full manual valid lap can become eligible again.
- [ ] Repeated recovery requests are rate-limited by simulation state; pausing cannot bypass the limit. Finished Cars cannot recover into active competition.
- [ ] Restart fully resets Race state and effects while retaining selected settings and previously saved eligible records. Return to Menu preserves those records.
- [ ] Tests cover pause/resume equivalence to an uninterrupted fixed-tick sequence, focus/input reset, occupied recovery location, seam progress, invalidation, and finish-window pause.
- [ ] Computer Use exercises countdown pause, mid-corner pause, focus loss, recovery, restart, and menu return. Required checks pass.

## Boundaries and design constraints

This intentionally replaces the v1 ESC-to-menu behavior; document the amendment. No rewind, quicksave, damage repair, or collision immunity. Lifecycle orchestration may live in the shell, but recovery placement/progress/eligibility rules belong to the engine-free domain boundary. Use the existing record-validity contract rather than inventing another one.

## Blocked by

- #50 — Match lap timing to the checker and preserve eligible Race results (supplies authoritative progress, per-Car finish status, and record invalidation).
