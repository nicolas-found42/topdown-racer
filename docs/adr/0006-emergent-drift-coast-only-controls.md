# Drift is emergent physics; speed control is Coast-only

There is no drift input on any platform: slides start when cornering force exceeds grip, and the skill is entry speed and line choice, not button timing. There is no brake input either: speed rises on a curve while Accelerate is held and falls on a curve while released (Coast), identically on desktop and touch. No assists of any kind exist; recovery is the one-keypress lap restart.

Tuning is gated by a bench in CI: the same closed-loop driver drives the shipped physics in every arm, differing only in Coast timing. Drift must be strictly faster than a conservative line, sloppy entries strictly slower, and the advantage capped at 30% of a lap, measured on a fixed seed suite.

Feasibility invariant: on every generated corner, Coast decel must cover the speed window between a hot entry and the grip speed, using only the approach before turn-in. The generator inherits this constraint, and the bench's CONSERVATIVE arm verifies it: if the conservative driver cannot drive the seed suite without sliding, the gate fails.
