# Gameplay batch verification evidence

Review baseline: `3bd95b97e0b1fa5aa89fc8d42034f81b6218a7e7`.

## Steering comparison (#51)

Run `cargo run -p topdown-racer --example steering_probe`. Every case starts the
same Hillside Track with unchanged physics. The correction experiment starts at
29.28 u/s and applies eight left-steering ticks, followed by four opposite ticks.
The full-Race comparison quantizes the same deterministic driver's steering to
left/neutral/right before applying each candidate response. This is a synthetic
controller experiment, not evidence of human preference.

| Rise | Eight-tick heading change | Steering after four reversal ticks | Wall-contact ticks in three laps | Flying laps (s) |
| --- | ---: | ---: | ---: | --- |
| Raw | 22.918 degrees | -1.000 | 0 | 29.266, 29.219 |
| 50 ms | 20.796 degrees | -0.250 | 0 | 29.875, 29.984 |
| 100 ms | 17.367 degrees | -0.250 | 0 | 29.641, 29.656 |
| 150 ms | 13.600 degrees | -0.208 | 0 | 29.500, 29.516 |

Raw stays default. The selectable 100 ms Smooth response reduces the measured
short correction without the larger initial delay of 150 ms; reversal uses the
faster return rate. No claim is made that this is the fastest or preferred human
setting. Sustained human comparison remains untested.

## Hillside sequence and choices (#43, #53, #57)

The shipping Track is the authored Hillside Circuit, with a straight grid and
braking boards before the first approach. The connected section begins on the
straight from (56,0) to (120,0), follows the widening first bend, then turns right
at (120,76) and back left toward (144,112). The exit leads through (104,152) onto
the long straight toward (40,152). It preserves the previously authored geometry.

Three useful choices are an earlier brake that preserves a tidy first apex;
a tighter first approach that defends the inside but constrains positioning for
the right turn; and delaying commitment through the right-left transition to
prioritize speed onto the exit straight. These are authored tradeoffs, not a
claim of measured optimal human lines. Computer Use captured the connected
sequence during an actual four-Car Race. Separate manual trajectory comparisons
have not been completed.

The headless Hillside regression runs the entire four-AI Race twice with identical
snapshots, zero wall contacts and zero off-Track ticks. Flying laps remain about
29.28–29.95 seconds. Computer Use captured the starting grid, the third-lap
transition with skid marks and rivals, and all four Cars classified after three
laps. Results visibly distinguish assisted laps and the standing-start lap.

Practice starts one Car at (81,0), heading east, at 22 u/s, with neutral controller
history and one second of frozen preparation. The cyan entry is (106,0); orange
route gates enforce the section; the white exit is on the following straight.
Timing is quantized to 1/64 second. Tests replay an entire successful synthetic
manual-input section and an identical retry, reject skipped/reversed gates,
assistance and recovery, and pin time/exit speed at the finish. Computer Use
verified practice entry, Autopilot invalidation with a visible reason, immediate
retry, and returning to the ordinary four-Car Race. Two completed contrasting
human approaches remain untested.

## AI and lifecycle (#49, #50, #54, #56)

Passing and encounter tests cover inside blockage/outside passing, late braking,
rejoin aborts, overlap, no-safe-pass following, and a lead change across the Track
seam. The encounter tests report contact severity, time near rivals, and pass
attempt/completion/abort counts with `--nocapture`. Race pace scales only the
planned speed envelope (Touring 0.70, Club 0.85, Race 1.0), retaining slot skill
0.88–1.0 and independent aggression 0.5–1.0. Established overlap reserves the
neighbor's Car-width corridor plus 0.6 units at turn-in; blocking either route
aborts a commitment instead of switching sides every tick.

Computer Use verified Manual/Autopilot labels, selection of Club/Smooth/Look
Ahead, pause, restart, and preservation of settings. Regression coverage includes
focus loss with Enter already pressed, deliberate resume, frozen simulation,
occupied recovery, cooldown, invalidation, and no ranking gain from teleportation.
Recovery resets the attached driver and prevents the distance tie-breaker from
improving until another checkpoint is legitimately cleared.

## Presentation, records, and remaining subjective checks

A small-window capture reproduced clipped menu content; sizing now follows the
actual camera viewport. Look Ahead's maximum offset also exposed a missing bake
margin; the regression now includes the full ten-unit lead and snapping slack.
Unsupported dash glyphs in targets/results were replaced with ASCII separators.

Record tests cover identity separation, eligibility, malformed input, atomic
replacement, and preserving an in-memory best on write failure. Legacy times
remain in their original file and are never compared with current records.
Practice uses a separate identity and cannot update Race records.

Decals use 2048 fixed slots and airborne FX use 128 slots. Original procedural
sound has four fixed player-feedback voices: engine, tire, rough surface, impact.
Impact onset has bounded volume and a decaying envelope, with quiet-time rearming
instead of tick-by-tick retriggering. B selects full, half, or muted volume. Pause
and menu silence feedback. Unit tests verify load/surface/contact distinctions.
An actual listening session remains outstanding; no claim of auditioned sound
quality is made. The #55 follow-up below adds smoke/dust/impact captures;
subjective listening and human motion-comfort judgment remain outstanding.

## Issue #55 follow-up (2026-09-17)

Native Bevy/Metal capture, not a simulated browser image or a human driving
session. Run `cargo run -p topdown-racer --example issue55_capture -- <case>`
with `BEVY_ASSET_ROOT` pointing to the absolute `crates/topdown-racer` directory.
Cases: `brake`, `drift`, `surface`, `impact`, `combined`. Each runs a four-Car
rolling fixture with real physics, snapshots, render systems, and audio plugin.
The fixture suppresses focus events (only in this example) and removes the
countdown GO label because the rolling grid has no countdown. Production
focus/pause policy is unchanged. Direct-executable runs use the same asset root.

Computer inspection of the actual native window screenshots:

- [Braking](issue-55/brake.png): red rear lamps on the blue player and white
  AI Opponent; all four silhouettes and road edges remain clear.
- [Lost grip](issue-55/drift.png): pale twin handbrake puffs and black tire
  marks are distinct from red brake lamps; nearby rivals remain readable.
- [Physical grass](issue-55/surface.png): warm dirt-colored rear trails on
  all four Cars, distinct from pale handbrake smoke. Terrain Zones are not read.
- [Wall impact](issue-55/impact.png): brief cream nose-local highlight on the
  blue Car after actual wall contact. No camera-shake effect exists.
- [Combined onset](issue-55/combined.png) and
  [sustained handbrake](issue-55/combined-sustained.png): smoke, dust and marks
  coexist; white road limit and three nearby rivals remain visible at the
  captured onset and after 80 ticks (1.25 seconds). This is sampled native
  visual evidence, not a subjective continuous-motion comfort judgment.

Initial captures exposed flat, fully opaque square smoke textures. Replaced
both stamps with original transparent stepped silhouettes and capped runtime
opacity at 45%. Cadence, density budgets, dimensions, and lifetime are unchanged.
The original artwork recipe and provenance are in `assets/sprites/fx/README.md`.

Acceptance audit:

1. **Pass:** all-Car brake-light system reads applied snapshot brake, not
   deceleration. Existing `brake_lights_follow_each_cars_echo_not_handbrake_or_motion`
   regression and native brake capture cover the contract.
2. **Pass:** #45–#47 triggers remain unchanged: Drift emits marks, handbrake
   emits smoke (including standstill, immediately removed on release), and
   physical non-Road movement above 2 u/s emits dust. Existing boundary tests
   pass; this change adjusts only particle artwork/opacity.
3. **Pass for captured examples:** the five captures above distinguish the
   four cues and show road/rival readability, including a sustained combined
   case. Human readability preference remains subjective and unclaimed.
4. **Pass:** engine gain follows throttle load (0.12 coast / 0.30 full load
   before effects volume), separately from speed-based pitch. Existing
   feedback regression compares load/coast at the same speed. No RPM/gearbox
   simulation is claimed or introduced.
5. **Pass:** tire gain smooths observable lateral velocity projected against
   Car heading, gated by Drift. New regression checks rise, severity change,
   and release decay. No snapshot or physics changes were necessary.
6. **Pass:** dust and rough-surface audio consume physical `Surface`; no
   decorative Terrain Zone participates in the mixer or emission path.
7. **Pass:** impact volume is clamped 0.1–0.8, flash fades in about 0.2 seconds,
   and retrigger requires eight consecutive contact-free ticks. Fixed quiet
   ticks erroneously accumulating through contact chatter. The old root-Sprite
   query matched no Cars after the child-Sprite migration; a dedicated player
   nose child now renders the cue. Audio uses a true one-shot restarted at
   onset, not a loop at an arbitrary phase. FixedUpdate onsets are latched
   until Update consumes them. No camera shake.
8. **Pass:** 2048 decal slots, 128 airborne slots shared by all Cars, and four
   player audio voices (engine/tire/rough/impact). Impact reuses one entity and
   stops its old sink before replacement. New ten-restart lifecycle regression
   confirms bounded entity/voice ownership, one-consumption onset and flash
   expiry; existing particle/decal capacity and reset regressions pass.
9. **Pass:** B still cycles full/half/mute. Menu, pause and preparation silence
   loops; impacts stop rather than replay after resume. Production focus loss
   retains deliberate-pause policy. No lifecycle policy change.
10. **Pass:** original procedural PCM and newly original pixel silhouettes;
    no copied game audio/art; asset provenance updated.
11. **Pass with explicitly outstanding audition:** scripted tests and native
    visual inspection completed. **No actual listening session was performed**;
    PCM tests and running the audio plugin are not evidence of perceived sound
    quality or speaker playback. Human audio audition remains outstanding.
12. **Pass:** `cargo fmt --all -- --check`,
    `cargo clippy --all-targets --all-features -- -D warnings`, and
    `cargo test --workspace --all-targets --all-features` pass (218 tests).
    `cargo test -p topdown-racer --test feedback -- --nocapture` passes five
    tests; the parallel simulations with and without presentation mixing have
    identical snapshots every tick and the player finishes at tick 6221.
    `cargo test -p topdown-racer feedback::tests -- --nocapture` passes the
    ten-restart lifecycle regression. No core files were changed.

Builds report the pre-existing upstream `block v0.1.6` future-incompatibility
notice, not a current compiler/Clippy warning failure. Earlier working checks
caught a nonexistent test API and an eight-argument system lint; both corrected
before the passing final checks. An initial impact fixture missed the wall;
the final fixture starts near the physical boundary and captures real contact.


## Final validation

`cargo check --workspace --all-targets --all-features`, formatting, Clippy with
warnings denied, and all 179 workspace tests pass. The final small/tall/minimum
window captures verified the menu no longer clips and the actual Car assets
render in the fixed viewport; the optional overview is hidden at small widths.
The direct-executable CI smoke run supplies the explicit crate asset root.

Measured encounter output: the outside pass completed after 172 ticks with one
attempt, one completion, no aborts, zero contacts/severity, and 173 near-rival
ticks. Rejoin: two attempts, one abort, zero contacts/severity, 150 near-rival
ticks. Narrow following: no pass attempts, zero contacts/severity, 300 near-rival
ticks and 9.949 units minimum gap. Solo Touring/Club/Race flying laps on the
controlled Sample Circuit were 33.141 / 28.562 / 25.750 seconds, repeated exactly.

Dedicated finish fixtures additionally verify equal-tick grid-order ties and a
lapped player taking the flag after the winner without requiring three laps.
