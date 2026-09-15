# Make small keyboard steering corrections deliberate and repeatable

## What to build

A keyboard player can make a small high-speed correction, release smoothly, and countersteer quickly without a physics rewrite. Ship a selectable Raw/Smooth steering response, with a documented default chosen from comparative driving evidence.

## Evidence and intended behavior

At approximately 29.28 world units/s, eight full-steering ticks (125 ms) rotated the Car 22.92 degrees. Throttle, coast, and braking hit the same yaw cap in that experiment. This demonstrates a sensitive input region, not proof that a particular smoothing constant feels best.

Add deterministic digital steering rise and return behavior at the human-control boundary. Opposite steering must recover promptly rather than waiting through an excessive center delay. Compare Raw with short rise-time candidates, such as 50, 100, and 150 ms, before selecting a Smooth preset. These are experiment candidates, not immutable acceptance thresholds.

## Acceptance criteria

- [ ] The menu exposes Raw and Smooth with a short explanation; the chosen setting applies consistently to the next Race.
- [ ] Smooth turns digital direction into bounded applied steering through fixed simulation ticks, independently of rendered frame rate.
- [ ] Release, rapid direction reversal, both directions held, lost focus, mode toggle, and restart have explicit, tested behavior. Both directions held resolves to neutral.
- [ ] The applied steering echo reports the actual command reaching Car physics, preserving replay observability.
- [ ] AI Opponent steering and Autopilot do not pass through human smoothing; engine, braking, grip, and yaw constants are unchanged in this slice.
- [ ] Replaying the same effective commands reproduces the same snapshot sequence; differing render cadence does not change a fixed-tick control stream.
- [ ] Record identity includes a handling/control-policy change where needed, without erasing unrelated valid data. If the new record system is not yet available, avoid pretending Raw and Smooth comparisons are a ranked leaderboard.
- [ ] A comparison note reports correction pulses, steering reversal response, clean laps, wall contacts, and driver observations for the same Track and starting conditions.
- [ ] Human preference is reported as tested or untested; automated lap speed is not presented as evidence of better feel. Keep Raw as an escape hatch if sustained human comparison is unavailable.
- [ ] Computer Use verifies control selection, steering/release, and restart; meaningful deterministic regression tests and required workspace checks pass.

## Boundaries and design constraints

No gamepad, remapping, dynamic grip, speed-sensitive steering authority, or AI retuning. A small pure input adaptation boundary is appropriate; a general input framework or wide refactor is not necessary. Record the selected response and evidence so later tuning is deliberate.

## Blocked by

- Draft 01 — Manual driving by default with visible Autopilot ownership (ensures the measured response belongs to the player).

## Research basis

[SuperTuxKart's player controller](https://github.com/supertuxkart/stk-code/blob/master/src/karts/controller/player_controller.cpp#L254) ramps digital steering with a separate return rate. Use the concept without copying source or importing kart handling.
