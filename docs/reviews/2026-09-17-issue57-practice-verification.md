# Issue 57 — Corner Practice verification (2026-09-17)

## Contrasting approaches (scripted digital Manual input, not human driving)
Both runs: Menu → P → identical preparation, countdown, Gate A cyan (directional approach gate),
section, Gate B white (directional exit gate), results overlay, retry R.

| approach | section time | exit speed | elapsed ticks |
| --- | --- | --- | --- |
| tidy (AI pace plan, brakes before the first bend) | 13.516s | 25.603 u/s | 994 |
| late (same steering plan, held throttle until 136/8) | 12.953s | 25.583 u/s | 957 |

Evidence: screenshots `docs/reviews/issue-57/{menu,preparation,gates,tidy-result,late-result,retry-result}.png`.
Late braking was 0.562 s faster on this section while sacrificing 0.020 u/s exit speed; both
approaches finished, so no claim of universal optimal line is made, and results explicitly say
"compare exit speed, not just entry speed" rather than automated coaching.

## Rapid retry determinism
Run 3 (R immediately from results) repeated the second attempt bit-for-bit:
`seconds=12.953125 exit_speed=25.583374` and the fixture asserts exact `PracticeRecord` equality.
Screenshots `retry-result.png` show identical result overlay; record key
`practice:hillside-braking-transition-v1:bec2bba2320500f7:handling1:manual-section1:Raw` holds only
the fastest eligible attempt (`{"seconds":12.953125,"exit_speed":25.583374}`), and the first (slower)
attempt never appears as a race record; `.scratch/issue-57-records.json` shows separate
`entries`/`practice_entries`.

## Race restoration
After Esc from practice results, the shell returned to ordinary Race with four Cars, three-lap rules,
normal grid, no practice session, and no practice key in race entries (assertions in
`crates/topdown-racer/examples/issue57_capture.rs`; screenshot `race-restored.png` was captured but
emitted zero bytes because the app exits the same frame, so only console assertions verify this
capture).

## Honest limitation
These are scripted digital Manual inputs (AiDriver pacing plan + keyboard injection) verified against
the real Bevy window — they are not two human approaches, and cannot substitute for the PR 59 spec
review item "Two completed contrasting human practice approaches remain unverified."
