# Spec: Racing Game v1 — pseudo-3D time-attack with daily/weekly generated Tracks

## Problem Statement

Players who want a quick, skill-deep racing fix in the browser have two bad options: heavy 3D racers that punish low-end devices and phones, or shallow toys with no competitive spine. No browser racer ships a shared daily/weekly generated Track with Leaderboard Ghosts — the one competitive loop that costs almost nothing to run. And everywhere, the small skill verbs that make racing worth mastering (the slide, the late lift) are either behind a wall of buttons or absent entirely.

## Solution

A pseudo-3D time-attack racing game that runs in a browser tab on desktop and phone with zero install. Every Track is generated from a seed: a new Daily Challenge and Weekly Challenge give everyone worldwide the same road and the same Leaderboard, and Generate and Share lets players roll fresh Tracks and trade Track Codes. Competition is asynchronous and account-free: your best Run is saved locally, you race it as a Ghost, you pick rival Ghosts off the Leaderboard, and you post your own times with just a display name.

The car has exactly two inputs: steer, and Accelerate. Holding Accelerate builds speed; releasing it (Coast) bleeds speed. There is no brake and no drift button: Drift happens when you corner too hot, and a well-judged slide is measurably the fastest way down the road — while a sloppy one runs you wide. Mastery is entry speed and Coast timing, nothing else.

## User Stories

1. As a desktop player, I want to steer with the keyboard, so that I can drive with familiar controls.
2. As a desktop player, I want speed to rise while Accelerate is held and fall while it is released, so that Coast timing is my speed control.
3. As a mobile player, I want a floating joystick that appears where my thumb lands, so that steering works in any grip.
4. As a mobile player, I want an Accelerate pad under my right thumb, so that speed control matches desktop exactly.
5. As a player, I want the car to slide when I corner too fast, so that Drift emerges from driving rather than from a button.
6. As a player, I want a well-judged Drift to carry more corner speed, so that Drift is the faster way down the road.
7. As a player, I want a sloppy entry to run wide and cost time, so that sloppiness is punished and precision matters.
8. As a player, I want no driving assists of any kind, so that my Run reflects only my driving.
9. As a player, I want the car to yaw and leave skid marks while sliding, so that I can read the Drift.
10. As a player, I want to restart a lap with one keypress, so that "one more Run" is instant and a crash is never a dead end.
11. As a player, I want every Track to lap in 40–70 seconds, so that a Run fits into a spare moment.
12. As a player, I want checkpoints on every Track, so that cutting is impossible and Leaderboard times are honest.
13. As a competitive player, I want per-checkpoint splits for my Run, so that I can see where I gain and lose time.
14. As a player, I want my best Run on each Track saved on my device, so that I always have a target.
15. As a player, I want to race my best Run as a Ghost, so that I am always chasing myself.
16. As a player, I want to pick Ghosts from the Leaderboard, so that rivals near my time give me something to beat.
17. As a player, I want a global Leaderboard per Track, so that I know where I stand worldwide.
18. As a player, I want to submit my Run to the Leaderboard, so that others have something to chase.
19. As a player, I want a display name stored on my device, so that my Runs carry my name without an account.
20. As a player, I want to edit my display name at any time, so that my identity on the Leaderboard stays mine.
21. As a player, I want a new Daily Challenge at UTC midnight, so that there is a reason to come back every day.
22. As a player, I want a new Weekly Challenge every Monday at UTC, so that I have a longer arc to master.
23. As a player, I want the Daily and Weekly Challenge identical for everyone, so that the Leaderboard compares fairly.
24. As a returning player, I want past Daily and Weekly Challenges playable forever, so that no Track ever dies.
25. As a player, I want unlimited retries on every Track, so that mastery is a loop, not a gate.
26. As a player, I want Generate and Share to roll a fresh Track, so that there is always a new road.
27. As a player, I want a difficulty class picker in Generate and Share, so that I choose my own level.
28. As a player, I want to share a Track Code, so that anyone can race exactly my Track.
29. As a player, I want to paste a Track Code, so that I can race Tracks other players generated.
30. As a player, I want identical Track Codes to build identical Tracks, so that sharing never breaks.
31. As a mobile player, I want the game rendered at my screen's pixel density, so that it is sharp, not blurry.
32. As an offline player, I want to race my PB Ghost on any Track I already have, so that losing the network never ends a session.
33. As a player, I want the Leaderboard to reappear when the network returns, so that connectivity loss is invisible when possible.
34. As a new player, I want to be driving within seconds of opening the page, so that nothing stands between me and the road.

## Implementation Decisions

Governed by ADRs 0001–0006 and the glossary in `CONTEXT.md` (pseudo-3D camera; deterministic hand-written simulation; generated-only Tracks; PvP excluded; client-validated Run integrity; emergent Drift with Coast-only controls).

**Stack and deployment.** TypeScript + Vite, zero game frameworks, Canvas 2D. The client is fully static and deploys to GitHub Pages; any visitor gets the complete game and Leaderboard from the URL alone.

**Deterministic core (the test seam).** Two pure functions carry the whole domain:
- Simulation step: fixed-timestep, road-space state (position along Track, lateral offset, lateral velocity, speed). Same inputs plus same Track produce bit-identical simulation on the same engine. A Run records its input timeline only — that recording is the replay.
- Track generator: (generator version, difficulty class, seed) → segment list with per-segment curve and hill values, length, and checkpoint placement. Pure; generator versions are frozen once released so existing Track Codes never change meaning.

**Car physics.** Road-space slip model with a clamped lateral impulse: cornering adds lateral velocity, grip cancels it up to the clamp, saturation is a slide. No physics library. Drift is emergent — no drift input exists on any platform.

**Controls.** Desktop: hold a key to Accelerate, two keys to steer. Touch: floating joystick (left thumb, steer only) + Accelerate pad (right thumb). Speed rises on a curve while Accelerate is held and falls on a curve while released (Coast), identically on both platforms. No brake, no assists, no reverse gear — recovery is the one-keypress lap restart.

**Coast feasibility invariant (ADR-0006).** On every generated corner, Coast decel must cover the speed window between a hot entry and the grip speed, using only the approach before turn-in. The generator must never emit a corner that violates this.

**Drift bench (CI gate).** The same closed-loop driver (seeded jitter, reads car state every frame) drives the shipped physics in all arms, differing only in Coast timing: CONSERVATIVE (coasts early, never slides), DRIFT (coasts later, slides, carries speed), OVERDRIVE (coasts too late, runs wide). Gates on a fixed ~10-seed suite: DRIFT strictly faster than CONSERVATIVE; OVERDRIVE strictly slower; DRIFT advantage capped at 30% of a lap; null run (identical arms) agrees within 1%. Drift pays in speed only — no boost rewards in v1.

**Track Code.** Shareable string encoding generator version, difficulty class, and seed. Identical code → identical Track. Round-trip codec.

**Content.** Daily Challenge: seed derived from the UTC calendar date; Weekly Challenge: seed derived from the ISO week, one difficulty class harder than Daily; both rotate at fixed UTC boundaries and keep permanent Leaderboards. Generate and Share: difficulty class picker (3 classes), fresh-seed roller, and Track Code entry.

**Ghosts.** Personal best stored locally; race-the-Leaderboard lets the player pick rival Runs to spawn as Ghosts. Ghost playback is a plain input replay through the simulation step; no netcode, no server state.

**Run integrity.** Checkpoint gates at generator-placed Track positions; every Run records per-checkpoint times (these double as the splits display). Validation is client-side (ADR-0005); the server does not re-simulate.

**Leaderboard service.** Cloudflare Worker + D1 (rankings) + R2 (input blobs). Endpoints: submit Run (Track Code, display name, time, checkpoint times, input blob reference), fetch Leaderboard per Track Code, fetch a Run's input blob. No auth. The client degrades gracefully offline: local PB Ghosts and any loaded Track stay fully playable; Leaderboard views and submissions resume when the network returns.

**Renderer.** Canvas 2D pseudo-3D projection over the segment list; back the canvas at devicePixelRatio, CSS-size the element; touch surface uses `touch-action: none`; fixed-step accumulator with render interpolation and clamped frame delta (background-tab safe).

**Local persistence.** Display name, per-Track personal bests, and input timelines in local storage.

## Testing Decisions

**What makes a good test here.** External behavior only, never implementation detail: given a seed, the generator returns the same Track; given an input timeline, the simulation returns the same lap time and positions to the bit; a Run that skips a checkpoint is rejected; a submitted Run appears on the Leaderboard in rank order; a Track Code decodes to the Track that encoded it. The determinism contract is the product — the strongest test is "replay any Run twice, observe byte-identical positions."

**Seams (confirmed in session).** One domain seam, two pure functions: the simulation step and the Track generator. Everything above — determinism, Ghost playback, checkpoint validation, the Coast feasibility invariant per generated corner, the drift bench — tests through it, headless and browser-free. The drift bench is a CI job at this seam: scripted driver policies feed inputs; the gate result is the assertion. Second seam: the Leaderboard API contract, tested against a local Worker runtime. Renderer, input, and UX are verified by browser smoke runs, not unit seams.

**Modules under test.** Simulation step; Track generator; Track Code codec; checkpoint validation; drift bench (CI); Leaderboard Worker contract.

**Prior art.** No tests exist yet (greenfield repo). External models: Kart Royale's drift-bench harness for the bench structure (same-driver A/B, null test, advantage gates); javascript-racer's fixed-step accumulator for the simulation loop pattern.

## Out of Scope

- Real-time PvP in any form (ADR-0004).
- Accounts, auth, profiles, cross-device sync.
- Track editor or any hand-authored Track (ADR-0003).
- Gamepad input (appeared in research; never decided — a later additive option).
- Tilt steering (iOS permission detour; possible later option).
- Slide rewards / mini-turbo-style boosts (ADR-0006; revisit only if the bench shows fast laps that feel unrewarded).
- Server-side re-simulation or anti-cheat beyond checkpoint validation (ADR-0005).
- PWA/service worker (v1 is a plain static page with full features; installability is a later enhancement that must never gate any feature).
- Multi-tier quality settings (single tier + correct devicePixelRatio handling in v1).
- Audio.
- Weekday difficulty ramp for the Daily Challenge (Weekly is one class harder; a Daily ramp may come later through the same class mechanism).
- Friends-only ghost lists (Track Code sharing covers it).

## Further Notes

- Full design history: the session record and research live in the repo (`docs/research/racing-game-inspiration.md`, `CONTEXT.md`, `docs/adr/0001`–`0006`).
- Determinism scope: bit-equality is required per engine, not across engines — each client simulates every Run locally (Ghosts are input replays), and the server never re-simulates. Engine-specific math variance is therefore acceptable.
- The repo name `topdown-racer` is a historical artifact from before the camera decision (ADR-0001); the product has no name yet.
- The 30% advantage ceiling, the seed-suite size, and the drift floor are structure commitments; their exact numbers are set the first time the bench runs against real curves.
