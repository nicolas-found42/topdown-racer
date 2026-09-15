## What to build

A new player starts a Race knowing which Car they control and whether they are driving manually. Manual is the default. An explicitly selected Demo/Autopilot mode remains available, is visible throughout the Race, and never produces an apparently manual personal best.

## Evidence and intended behavior

The gameplay review observed the player Car accelerating without input. The current driver also resumes AI on neutral input, so releasing a key can hand control back to AI. A single player pedal input replaces the whole AI input, including steering. This discontinuity makes learning and comparing laps confusing.

Make ownership an explicit mode. In Manual, neutral means neutral. In Autopilot, movement keys do not temporarily replace parts of the driver; the displayed toggle switches to Manual. Toggling ownership clears stale input and takes effect at a fixed-step boundary. A visible help message explains the toggle and ordinary throttle, brake/reverse, steering, and handbrake controls.

## Acceptance criteria

- [ ] Launching and selecting Start Race defaults to Manual; after countdown a Car with no input does not accelerate itself.
- [ ] Menu and Race HUD identify the selected driving mode and the player's Car; the toggle binding is discoverable before starting.
- [ ] Autopilot is an explicit option, drives the player Car when selected, and remains visibly labelled during countdown, racing, and results.
- [ ] Throttle release in Manual coasts according to physics; no implicit AI takeover occurs.
- [ ] Manual input and AI decisions have one unambiguous owner per fixed tick; switching modes cannot leave a held throttle or steering command latched.
- [ ] A lap remembers any Autopilot participation until that lap ends. Returning to Manual cannot relabel that lap as manual.
- [ ] Until configuration-aware records land, assisted laps cannot overwrite the legacy manual best. The next wholly manual flying lap may become eligible.
- [ ] Restart and menu transitions reset ownership/input according to the visible selection, without carrying hidden driver state.
- [ ] Headless tests cover neutral Manual, continuous Autopilot, toggles mid-lap, and identical fixed-step input/mode streams producing identical snapshots.
- [ ] Computer Use verifies starting without input, holding/releasing controls, toggling, restarting, and legibility at supported window sizes.
- [ ] Required formatting, lint, and workspace tests pass.

## Boundaries and design constraints

Keep AI Opponents independent of the player's assistance mode. Do not add general accessibility assists, gamepad support, remapping, or AI difficulty here. Preserve the engine-free deterministic core and snapshot-driven shell. This deliberately changes the current implicit control behavior, without changing Car physics.

## Blocked by

None (can start immediately).

## Research basis

[BeamNG's official assistance and input notes](https://www.beamng.com/game/news/patch/beamng-drive-v0-25/) demonstrate explicit assistance settings and intervention feedback. The local gameplay review supplies the actual defect evidence; external behavior is precedent, not an instruction to copy its physics.
