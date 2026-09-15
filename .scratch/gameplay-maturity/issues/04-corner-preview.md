# Show upcoming corners and nearby rivals without changing pixel scale

## What to build

A player approaching a corner vertically receives useful forward preview, can locate themselves on the Track, and can tell when a rival is just outside the viewport. Preserve the world-aligned, fixed-scale pixel-art presentation.

## Evidence and intended behavior

The reviewed 80-by-45 view gives a centered Car 40 horizontal units but only 22.5 vertical units ahead. At measured speed this is roughly 1.37 versus 0.77 seconds of visibility. The HUD currently provides neither Track overview nor off-screen rival information.

Ship a bounded velocity-based camera lead with a Centered option and a compact Track overview with Car markers. Off-screen indicators identify nearby rivals only; do not fill the viewport with arrows for distant Cars. Use a small fixed world-distance threshold, document it, and tune it during visual review.

## Acceptance criteria

- [ ] Centered and Look Ahead can be selected before a Race; both preserve world orientation, fixed zoom, nearest sampling, letterboxing, and integer mapping at supported sizes.
- [ ] Look Ahead responds to velocity rather than nose direction, has a low-speed dead zone, and is smoothed before final texel snapping.
- [ ] Lead is bounded so the player Car remains visible during acceleration, Drift, reverse, abrupt stops, and recovery. Restart resets camera history.
- [ ] The baked world covers every allowed offset at every Track boundary; no black void, stale scenery position, or density change appears.
- [ ] A compact overview uses actual Track geometry, highlights the player's Livery/number, and tracks all four Cars consistently through the finish seam.
- [ ] Off-screen rival cues appear only for nearby eligible Cars, clamp inside the play viewport, disappear when the rival becomes visible, and never imply a wrong side through letterbox bars.
- [ ] HUD text and markers remain legible without covering the immediate driving corridor; tiny-window fallback deliberately hides optional information rather than clipping core status.
- [ ] Camera calculations are bounded under zero velocity and rapid direction changes; rendering changes never alter Race outcomes.
- [ ] Computer Use captures horizontal and vertical corner approaches, Drift/reverse, pack separation, and wide/tall/small windows. Document preview improvement and any untested motion-comfort judgment.
- [ ] Required checks pass, with tests limited to useful pure coordinate/bounds behavior rather than shell implementation details.

## Boundaries and design constraints

No rotating camera, variable zoom, full radar, racing-line overlay, or new Track authoring. Braking landmarks are handled by the Track work. Preserve ADR-0003; a camera lead is an offset, not a reason to abandon fixed density.

## Blocked by

- #42 — Wire the world bake into the shell (the real viewport, bake bounds, and scenery stack must be available to validate the offset).

## Research basis

[Forza's proximity radar](https://forza.net/news/forza-motorsport-update-10) addresses information lost to blind spots. Top-down already exposes overlap locally, so this slice addresses off-screen awareness with smaller cues.
