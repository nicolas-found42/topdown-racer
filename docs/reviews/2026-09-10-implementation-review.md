# Implementation checkpoint review

## Standards

Fixed point: `3bd95b97e0b1fa5aa89fc8d42034f81b6218a7e7`; working-tree mode.
No commits after the fixed point. Includes pre-existing working-tree changes
and new untracked implementation/test/assets files. This is a checkpoint,
not approval to merge or a claim that all tickets are complete.

Sources: `AGENTS.md`, `docs/agents/domain.md`, ADR-0001, ADR-0002,
ADR-0003, and the installed code-review skill's smell baseline.

1. **Possible Primitive Obsession / Mysterious Name:**
   `crates/core/src/simulation.rs` represents a finished Car in AI perception
   as `(Vec2::splat(1.0e6), Vec2::ZERO)`. The coordinate sentinel hides the
   actual concept of an inactive competitor and can affect pass statistics.
   Prefer explicit participation in the perception boundary while preserving
   stable Car indices. This is a design judgment, not a documented violation.

2. **Possible Duplicated Code:** `tick` and `snapshots` each assign
   `snapshot.finish_window_ticks = self.winner_tick.map(...)`.
   Keep the finish-window projection in one place to prevent divergence
   during the pending pause/recovery changes. This is a small maintenance
   judgment, not a documented violation.

No hard documented-standard violation identified in this pass. Engine-free
rules remain in core; effect budgets and geometry are in the shell; the
physics constants and fixed pixel scale are preserved. Formatting and lint
findings are handled by tooling and are excluded from these findings.

Total: 2 heuristic findings; highest concern is the inactive-Car sentinel.

## Spec

Fixed point: `3bd95b97e0b1fa5aa89fc8d42034f81b6218a7e7`; working-tree mode.
Sources: the open GitHub tickets #43, #45–#47, #49–#57 and parent specs #3/#32,
fetched through `gh`. This pass was performed after saving the Standards
report, against the ticket criteria directly.

1. **#50, records incomplete:** “Every eligible manual best is persisted at
   lap completion, keyed by Track content identity, handling version, and
   eligibility policy.” Persistence still uses the old single unversioned
   file at Race end. The new Track and corrected timing must not share that
   target. Migration, atomic replacement, and visible write errors remain.

2. **#50, finish validation partial:** “Deterministic tests exercise the
   gate cases, winner/player/lapped/DNF finishes, equal-tick crossings.”
   Checker alignment, untravelled rolling starts, winner continuation,
   DNF expiry, and frozen results are covered. Dedicated high-speed,
   oscillation, lateral, equal-tick, and lapped-finisher cases remain.

3. **#51 absent:** “The menu exposes Raw and Smooth.” The deterministic
   human input adapter, response comparison, and focus/reset behavior remain.

4. **#52 absent:** “Centered and Look Ahead can be selected before a Race.”
   Camera lead, Track overview, nearby-rival indicators, and bounds coverage
   remain.

5. **#56 absent:** “ESC during countdown/racing opens a clear Pause menu.”
   ESC still returns to Menu. Pause, deliberate resume, focus handling,
   safe recovery, and eligibility invalidation remain.

6. **#57 absent:** “The menu offers Race and a clearly separate Corner
   Practice entry.” Challenge attempts, timing/results, deterministic retry,
   record separation, and return-to-Race behavior remain.

7. **#55 partial:** “Engine sound differentiates throttle load from coasting”
   and “Impact feedback scales within bounded limits.” The new visual
   emitters do not implement load/surface/impact audio, smooth slip cues,
   volume control, or listening verification.

8. **Visual/gameplay evidence incomplete:** #43 requires “Capture screenshots
   of a full race on the new circuit”; #53 requires “At least three documented
   choices” with manual trajectories/captures; #45–#47 require effect
   captures. Headless Hillside laps are clean at approximately 30 seconds,
   but those capture and driving-comparison requirements remain open.
   #49/#54 also need their remaining input/window/preset encounter checks.

No unrelated new feature identified. Total: 8 grouped findings; highest
concern is incompatible records being compared across Track/timing changes.

Standards: 2 heuristic findings (inactive-Car sentinel highest concern). Spec: 8 grouped findings (record compatibility highest concern).
