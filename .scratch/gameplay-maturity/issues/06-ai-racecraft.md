# Race fixed-difficulty AI that commits to clean passes and respects overlap

## What to build

A player selects a fixed opponent pace before a Race and experiences AI Opponents that can commit to a passing side, abandon a blocked move, and leave room for established overlap. Faster racing earns progress without hidden catch-up power.

## Evidence and intended behavior

The existing driver already derives speed from curvature and braking limits, selects line offsets, avoids slower Cars, and recovers in reverse. The measured four-AI Race was clean with no position changes. That baseline does not prove overtaking is broken; it identifies the need for encounter-based verification rather than lap-time-only tuning.

Add three named pace presets and a compact deterministic pass commitment/abort policy. Preserve per-Car variation within bounded, documented ranges. Keep aggression fixed and independent of the pace selection in this first slice; a separate aggression UI is unnecessary.

## Acceptance criteria

- [ ] Menu offers three opponent pace presets with plain descriptions; the chosen preset is visible in Race/results and stays fixed for the Race.
- [ ] Presets affect driver decisions/margins, not Car engine, grip, collision rules, or artificial position-dependent speed. The same physical limits apply to every Car.
- [ ] Repeated clean solo runs establish the intended pace ordering on controlled Tracks without requiring exact universal lap-time ratios.
- [ ] A faster Car behind a slower one can choose a side and complete a clean pass in a designed passing opportunity.
- [ ] Pass commitment cannot oscillate left/right each tick; a blocked route causes a controlled abort/follow decision rather than ramming or indefinite deadlock.
- [ ] Established side-by-side overlap at turn-in receives sufficient road space. Define overlap and the defensive-movement policy in domain terms and exercise boundary cases.
- [ ] Scenario coverage includes inside blockage, outside pass, late-braking approach, slower rejoin, no-safe-pass corridor, and a lead change across the Track seam.
- [ ] Existing stuck/reverse recovery still works and cannot be triggered by ordinary short following delays.
- [ ] Identical Track, preset, grid, and input streams produce identical snapshots. No runtime randomness, wall clock, learned model, or rubber-banding is introduced.
- [ ] Preserve the clean four-AI Race bar. Report attempted/completed/aborted passes, contact severity, and time near rivals for scenarios; do not manufacture overtakes to meet a quota.
- [ ] Computer Use verifies preset selection and at least one visible passing encounter; required checks pass.

## Boundaries and design constraints

Use small deterministic fixtures so this can start without the new Track or finish-rule work. Preserve ADR-0002's driver/physics separation and deterministic personality policy. This is a bounded pass policy plus player-facing pace selection, not a generalized race planner or reinforcement-learning project.

## Blocked by

None (can start immediately; purpose-built Track fixtures avoid a false dependency on #43).

## Research basis

[Sony AI's sportsmanship account](https://ai.sony/blog/dont-cross-that-line-how-our-ai-agent-learned-sportsmanship) describes the timid-versus-bullying tradeoff. [Fair Play in the Fast Lane](https://arxiv.org/abs/2503.03774) motivates explicit space and defensive constraints; no research architecture is prescribed here.
