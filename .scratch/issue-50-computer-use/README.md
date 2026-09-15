# Issue #50 — Computer Use evidence (2026-09-15, `main` @ `54cfdc1`)

Live run of `target/debug/topdown-racer` (`cargo run -p topdown-racer`), driven
through `computer.run` / `desktop.*` window helpers (`win.press`, `win.screenshot`).
HUD readings below are vision-model transcriptions of each PNG; caveats are noted
where the small HUD text was ambiguous.

The persisted record files already on this machine were snapshotted before the
runs and compared after: `~/Library/Application Support/topdown-racer/records-v2.json`
was byte-identical afterwards (every lap driven here was `AUTO`, so ineligible),
and `best_lap.txt` (`20.296875`) was untouched.

| File | Checklist item | Observed |
|---|---|---|
| `01-menu-persisted-best.png` | Process start reads the saved eligible record | Menu: `TARGET TO BEAT - BEST 00:31.34`, `YOU: BLUE CAR #1 \| MANUAL`, `PACE: RACE \| STEERING: RAW` |
| `02-countdown-grid.png` | Controls locked before green | Large `2`, `MANUAL`, `LAP 1/3`, `POS 1st/4`, `LAP TIME 00:00.00`, `SPEED 0 u/s`, four-car grid |
| `03-autopilot-engaged.png` | `T` gives the player Car a labelled Autopilot | `DEMO / AUTOPILOT \| LAP ASSISTED`, `LAP TIME 00:02.69`, `SPEED 23 u/s` |
| `04-dnf-results.png` | Finish window expires into explicit DNF entries | `RACE FINISHED`; `1st–3rd` are the AI Cars at `3/3 laps`, and `DNF - YOU / Car 1 - 0/3 laps`; eligibility note shown |
| `05-crossing-before-checker-lap1.png` | Approach | `LAP 1/3`, `POS 1st/4`, `LAP TIME 00:30.58`; car **left of** the painted checker |
| `06-crossing-after-checker-lap2-credited.png` | **Visible crossing credits the lap** | `LAP 2/3`, `LAP TIME 00:01.36`, `BEST 00:30.80`; car **past** the same painted checker — the lap flipped to 2 at the checker, not at a proximity radius |
| `07-finish-window-countdown.png` | Visible finish-window countdown | `PACE: RACE \| FINISH WINDOW 43.8s \| STEERING: RAW`, `LAP 3/3`, `POS 1st/4` |
| `08-finish-window-player-still-racing.png` | Countdown runs while the player is still racing | `DEMO / AUTOPILOT \| LAP ASSISTED`, `FINISH WINDOW 35.6s`, `LAP 2/3`, `POS 4th/4`, `BEST 00:29.63` |
| `09-results-losing-player-finished.png` | **Losing player's finish, classified last** | `1st - Car 2 - 3/3` / `2nd - Car 3 - 3/3` / `3rd - Car 4 - 3/3` / `4th - YOU / Car 1 - 3/3 laps - 00:39.81 AUTO 00:29.63 AUTO 00:29.70 AUTO - Best 00:29.63` |
| `10-menu-after-race-best-retained.png` | Best survives returning to the menu | `TARGET TO BEAT - BEST 00:31.34` after `Esc` from the results screen |
| `11-menu-after-relaunch-best-retained.png` | **Best survives an app relaunch** | Fresh process (defaults back to `MANUAL`) still shows `TARGET TO BEAT - BEST 00:31.34` |

## How each run was produced

- **Run A (frames 01–07).** Enter → countdown → `T` during the countdown →
  the player's Autopilot ran the whole Race and finished 1st.
  `1st - YOU / Car 1 - 3/3 laps - 00:30.80 AUTO 00:29.59 AUTO 00:29.63 AUTO - Best 00:29.59`.
- **Run B (frame 04).** Restart, then never leave Manual: the player sat on the
  grid, the three AI Cars finished, and the 45 s window expired with the player
  still on lap 1 → `DNF`.
- **Run C (frames 08–09).** Restart, sit still for the first ~8 s of racing, then
  engage Autopilot (toggle confirmed by reading the HUD back). The player was a
  lap down when the leading AI Car took the flag and finished **4th on its own
  crossing**, 27 s into the window, retaining `3/3` completed laps.

## Known limits of this capture set

- The driver here was the in-game Autopilot, not a human: these captures prove the
  gate, the per-Car finish, the window, the results projection and the record
  survival path, but they are not a claim about human driving feel.
- The eligible manual record shown in frames 01/10/11 was set in an earlier
  session and is present on disk; this run re-read it, never re-wrote it.
- Frames 05/06 bracket the lap-1→lap-2 crossing (≈1.6 s apart); the exact
  mid-crossing frame was not sampled because the sampling period is coarser than
  the checker's ~0.07 s crossing time at racing speed.
