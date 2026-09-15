## What to build

Complete a focused gameplay pass on the hillside Track after #43: one connected sequence where a heavy-braking approach permits competing lines, a right-left transition rewards positioning, and exit speed matters on the following straight. Players can explain the tradeoff and AI Opponents can drive it cleanly.

## Evidence and intended behavior

The old Sample Circuit contained seven left turns, constant width, and no landmarks. #43 already owns replacing it with a hillside-mix Track, width variation, scenery, and clean-AI verification. This ticket owns a bounded follow-up sequence and its racing decisions, not another Track replacement.

Inspect the delivered #43 Track first. Preserve any sequence that already satisfies this behavior and improve only what evidence shows is missing. Favor one demonstrable passing and transition complex over a wholesale redraw. Keep a fast sweeper or tightening bend elsewhere if already authored; they are not additional mandatory rebuilds here.

## Acceptance criteria

- [ ] The shipping Track includes a heavy-braking approach wide enough for two sensible lines, followed by a meaningful right-left change and an exit feeding a straight.
- [ ] At least three documented choices trade entry speed, apex position, defensive line, or exit speed. Evidence includes manual trajectories or captures, not only vertex count.
- [ ] Original, palette-compliant braking landmarks become visible before the braking decision in the centered camera; kerbs and props do not hide road limits.
- [ ] Road width ramps are gradual and valid; scenery and Terrain Zones remain presentation-only and do not imply unsupported grip changes.
- [ ] The full grid occupies a straight with valid spacing and headings; no trailing Car begins around a bend or outside the usable Track.
- [ ] Existing start-line behavior is preserved or updated consistently with the shared finish gate if that work has landed. Track content identity changes when the geometry changes.
- [ ] A four-AI clean Race retains zero wall-contact and off-Track ticks on the authored Track. Record new timing explicitly; do not adjust handling merely to force an obsolete timing constant.
- [ ] Deterministic inside/outside passing fixtures establish adequate space without requiring that every default Race contain an overtake.
- [ ] Computer Use reviews the sequence manually, follows AI through it, and captures landmarks, pack overlap, and the grid. Required checks pass.

## Boundaries and design constraints

No second shipping Track, editor, Track selector, physics change, or terrain-handling mechanic. #43 remains unchanged and owns initial authoring; this issue is not its replacement or parent. If #43 already meets the behavioral criteria, deliver the validation plus only the remaining focused improvements.

## Blocked by

- #43 — New hillside-mix circuit + AI clean-race revalidation (this is a gameplay pass on that delivered Track).

## Research basis

[Original Fire Games' Circuit Superstars interview](https://collective.square-enix-games.com/en_US/news/interview-original-fire-games-circuit-superstars) connects accessible control with wheel-to-wheel tactics. The local review motivates direction and line variety, not an arbitrary number of corners.
