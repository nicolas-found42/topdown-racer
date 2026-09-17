# Issue #49 — Computer Use evidence (2026-09-14, `main` @ `b253f7e` + 3 baseline fixes)

Live run of `target/debug/topdown-racer`, driven via `computer.run`
(`desktop.screenshot` / `window.press`). Game has no key-hold primitive, so
"holding" W was a rapid `w`-press pulse stream. Readings below are
vision-model transcriptions of each PNG, quoted with their caveats.

| File | Checklist item | Observed HUD / state |
|---|---|---|
| `01-menu-manual-default.png` | Start defaults Manual; toggle discoverable | Menu: `YOU: BLUE CAR #1 \| MANUAL`, `TARGET TO BEAT - BEST 00:31.34`, `PACE: RACE \| STEERING: RAW`, `START RACE (Enter)` / `QUIT (Q)`, full `T/W/S/A/D/Space/Esc` help block |
| `02-countdown-grid.png` | Countdown locks controls, labels mode | Large `3`, `YOU: BLUE CAR #1 \| MANUAL`, 4-car grid, `LAP TIME 00:00.00`, `SPEED 0 u/s` (vision OCR reported `LAP 4/3`; small-text artifact, countdown state confirmed by `3` + zero timer + stationary grid) |
| `03-no-input-manual-stationary.png` | No-input Car does not self-drive | Racing, `MANUAL`, `LAP 1/3`, `POS 4th/4`, `LAP TIME 00:09.41`, `SPEED 0 u/s`, blue car stationary at the checker |
| `04-throttle-pulse-manual.png` | Manual throttle responds, stays Manual | `MANUAL`, `LAP 1/3`, `4th/4`, `00:40.13`, blue car just past the line (stationary at capture instant) |
| `05-autopilot-engaged.png` | `T` → Autopilot drives player Car, labelled | `DEMO / AUTOPILOT \| LAP ASSISTED`, `LAP 1/3`, `4th/4`, `01:19.94`, `SPEED 18 u/s` |
| `06-back-to-manual-assisted-sticky.png` | Return to Manual cannot relabel lap | `MANUAL \| LAP ASSISTED`, `LAP 1/3`, `4th/4`, `01:34.55`, `6 u/s`, `FINISH WINDOW 39.8s` |
| `07-restart-fresh-countdown.png` | Esc→menu→Enter rebuilds fresh race | Fresh countdown `2`, `MANUAL`, `LAP 1/3`, `POS 1st/4`, `00:00.00`, `SPEED 0`, 4 cars on grid |
| `08-manual-throttle-motion.png` | Manual motion after restart | `MANUAL`, `LAP 1/3`, `4th/4`, `01:09.31`, `SPEED 1 u/s`, blue car right of the start line |
| `09-coast-after-release.png` | Release → coast, no AI takeover | `MANUAL`, `LAP 1/3`, `4th/4`, `01:31.66`, `SPEED 0 u/s`, `FINISH WINDOW 42.5s`, blue + white rival visible |
| `10-minimum-window-results.png` | Legibility at 640×360 minimum | `RACE FINISHED`, `YOU: BLUE CAR #1 \| MANUAL`, all rows incl. `DNF - YOU / Car 1 - 0/3`, eligibility note, `Press R to restart, ESC for menu` — no clipping; small text soft but readable |

## Known limits of this capture set

- Sustained held-throttle: pulse taps fell between fixed ticks, so top-speed
  motion is not demonstrated live. Throttle→motion and release→coast physics
  are covered headlessly (`held_throttle_accelerates_the_car_from_rest`,
  `explicit_ownership…` coast tick).
- Player DNF in shot 10 is expected: the car sat still while AI finished and
  the 45 s window expired — itself a valid finish-window observation.
- Superseded temp captures (pre-game desktop probes, mid-sequence duplicates)
  were left in the OS temp dir, not copied.

## Follow-up verification (2026-09-17, issue/49-manual-driving-default)

The existing numbered captures above are historical evidence, not reruns on
this branch. No Computer Use device was mounted for this follow-up; the native
game was driven with System Events discrete keys and CoreGraphics HID key-down
and key-up events. The process was explicitly made frontmost. A two-second W
hold, followed by release and two seconds without input, produced:

| File | Observed state |
|---|---|
| `28-native-throttle-release.png` | Immediately after held W release: `MANUAL`, `SPEED 25 u/s`, lap time `00:40.78` |
| `29-native-coast.png` | Two seconds after release: `MANUAL`, `SPEED 6 u/s`, lap time `00:42.86` |
| `30-native-steering-release.png` | After a one-second W+A hold and release: `MANUAL`, `SPEED 16 u/s` |
| `21-manual-after-toggle.png` | T back from Autopilot: `MANUAL | LAP ASSISTED`, `SPEED 8 u/s` |
| `22-paused-label.png` | Pause retains `MANUAL | LAP ASSISTED` |
| `23-after-resume-label.png` | Resume preparation (`READY`) retains `MANUAL | LAP ASSISTED` |

Held-input latching and release/coasting are additionally asserted through the
real shell keyboard mapping and simulation systems in
`toggling_to_manual_requires_held_controls_to_be_released`. A fresh menu selection
after returning from pause and a paused T press followed by resume are covered
by dedicated lifecycle regressions. All 198 workspace tests pass.

Limitations: the native run displayed the Track, HUD and minimap but not the Car
sprites, so steering direction cannot be visually certified from these captures.
The no-input follow-up capture briefly reported 2 u/s (cause not established);
it does not prove exact stationarity. Historical capture 03 and the deterministic
neutral-input test provide that evidence. The historical minimum-window result
capture remains the 640x360 evidence; follow-up captures use 1280x720. Discarded
follow-up shots captured a terminal or a paused game and are not driving proof.
