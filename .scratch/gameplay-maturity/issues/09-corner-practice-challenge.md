# Practice one corner sequence with repeatable entry and exit-speed feedback

## What to build

A player selects one authored Corner Practice challenge from the menu, begins from the same approach state, drives the hillside Track's braking/transition sequence, and immediately sees elapsed time plus exit speed. One action retries the exact challenge so the player can compare a late-braking entry with a better exit.

## Brainstormed addition

Turn the broad practice recommendation into a small experimental driving lesson: equal entry conditions, one section, two useful measurements, and instant retry. Avoid building an entire time-trial career or ghost system first. The challenge teaches why the fastest entry is not always the fastest way through a sequence.

## Acceptance criteria

- [ ] The menu offers Race and a clearly separate Corner Practice entry with a short explanation of the selected sequence and controls.
- [ ] Exactly one challenge ships, on the sequence delivered by Draft 05; there are no AI Opponents in the practice session.
- [ ] Every attempt begins with the same Track-relative pose, heading, velocity, and neutral controller history, preceded by a preparation countdown. Position is far enough upstream for a deliberate braking decision.
- [ ] Timing starts at a directional approach gate and stops at a directional exit gate after ordered section progress; gate geometry is visible and agrees with measured crossings.
- [ ] Results show elapsed section time, exit speed in the game's actual world-unit convention, and change against the player's best eligible attempt for this challenge.
- [ ] A short authored hint explains the entry-versus-exit tradeoff. Do not claim automated coaching or infer a universal optimal racing line from one speed measurement.
- [ ] Reverse crossings, cutting the sequence, Autopilot, and recovery invalidate an attempt with a visible reason; failed attempts still offer immediate retry.
- [ ] Retry restores the exact initial simulation/controller state, camera history, and transient FX without reloading all static assets or carrying prior input.
- [ ] Challenge records are local and keyed by challenge/Track identity, handling version, and eligibility policy. They cannot overwrite or appear as full-Race lap records.
- [ ] Returning to ordinary Race restores four Cars, three-lap rules, normal grid, selected settings, and ordinary records without leaking practice state.
- [ ] Headless tests verify repeatable starts, gate/route validity, invalid attempts, retry determinism, and record separation. Computer Use verifies at least two contrasting approaches and rapid retries.
- [ ] Required workspace checks pass, and the challenge is understandable without reading developer documentation.

## Boundaries and design constraints

This is a focused post-v1 practice-mode addition. Do not add ghosts, replay recording, medals, multiple selectable sections, a Track selector, or automatic driving advice. Reuse authoritative directional gates and record identity from Draft 02, but keep challenge attempts distinct from the domain's three-lap Race. Add only the domain vocabulary needed to explain that distinction.

## Blocked by

- Draft 02 — Match lap timing to the checker and preserve eligible Race results (reliable gates and record eligibility/identity).
- Draft 05 — Give the hillside Track a readable braking duel and direction-change sequence (the single authored challenge target).

## Research basis

[Gran Turismo's Circuit Experience](https://www.gran-turismo.com/au/news/00_1796083.html) demonstrates section-based learning. This proposal adds a repeatable entry-state and exit-speed comparison tailored to the local handling model; it does not copy a licence-test campaign.
