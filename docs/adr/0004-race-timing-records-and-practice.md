# Directional timing, explicit eligibility, and separate practice records

Tickets #49–#57 replace implicit player assistance and proximity-based lap timing with explicit Manual/Autopilot ownership and one Track-derived directional gate shared by timing and the checker. After the winner, each Car takes the flag at its next valid lap crossing within a 45-second simulation-time window; classification then freezes, finishers coast without collisions, and remaining Cars become DNF. Equal-tick finishes use stable grid order.

Eligible Manual flying laps save immediately in `records-v2.json`, keyed by Track content, handling version, and steering/eligibility policy. The legacy unversioned file is preserved separately because its old timing and Track cannot support meaningful comparisons. Recovery invalidates the current lap, preserves classification until another checkpoint is validated, requires speed below 1 u/s and six units of clearance, and has a three-second simulation-time cooldown.

Pause deliberately replaces the former ESC-to-menu behavior: focus loss or Escape freezes simulation, FX, and audio; resuming requires an action and one second of preparation. Corner Practice is a distinct one-Car attempt with its own gates, results, and record namespace; returning to Race restores the grid and selected settings. These lifecycle changes are intentional amendments to the v1 scope.

The focused load/surface/impact audio layer in #55 also amends the original presentation spec's audio exclusion. It remains snapshot-driven, uses original procedural audio, and has four fixed player-feedback voices; it adds no gearbox model, physics change, or camera shake. The human listening and motion-comfort judgments are documented separately from automated verification.
