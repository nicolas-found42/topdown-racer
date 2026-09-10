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
quality is made. Full smoke/dust/streak capture coverage and motion-comfort
judgment also remain outstanding.


## Final validation

`cargo check --workspace --all-targets --all-features`, formatting, Clippy with
warnings denied, and all 177 workspace tests pass. The final small/tall/minimum
window captures verified the menu no longer clips and the actual Car assets
render in the fixed viewport; the optional overview is hidden at small widths.
The direct-executable CI smoke run supplies the explicit crate asset root.

Measured encounter output: the outside pass completed after 172 ticks with one
attempt, one completion, no aborts, zero contacts/severity, and 173 near-rival
ticks. Rejoin: two attempts, one abort, zero contacts/severity, 150 near-rival
ticks. Narrow following: no pass attempts, zero contacts/severity, 300 near-rival
ticks and 9.949 units minimum gap. Solo Touring/Club/Race flying laps on the
controlled Sample Circuit were 33.141 / 28.562 / 25.750 seconds, repeated exactly.
