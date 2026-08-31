# Racing Game

A pseudo-3D time-attack racing game for the browser (desktop + mobile). Core loop: beat the Daily and Weekly Challenge Tracks, and race any shared seed; Drift is the mastery layer.

## Language

**Track**:
A playable course generated from a seed. Never hand-authored.
_Avoid_: level, map, course, circuit

**Track Code**:
The shareable encoding of a Track's identity. Identical code → identical Track.
_Avoid_: seed (the seed is the raw number; the code is the encoding)

**Run**:
A single timed attempt on a Track, recorded as inputs that replay bit-identically.
_Avoid_: attempt, race, replay

**Ghost**:
A Run replayed as a rival car.
_Avoid_: rival, replay car

**Drift**:
A controlled slide carried through a corner and traded for speed; the mastery skill separating fast laps from clean ones.
_Avoid_: slide, skid (a skid is uncontrolled)

**Leaderboard**:
Per-Track ranking of the best Runs.
_Avoid_: scoreboard, ladder

**Daily Challenge**:
The Track derived from the calendar date — identical for every player that day — with its own Leaderboard.
_Avoid_: track of the day

**Weekly Challenge**:
The Track derived from the calendar week — identical for every player that week — with its own Leaderboard.
_Avoid_: track of the week

**Generate and Share**:
Player-facing Track creation: roll a fresh seed into a Track, drive any Track Code, share codes. No editor — only seeds.
_Avoid_: editor, sandbox, custom tracks
