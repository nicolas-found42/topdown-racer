# Spec review — implementation in progress

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
