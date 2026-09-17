# Decision-making AI driver, no scripted racing line

The v1 AI followed the Track polyline waypoint-by-waypoint with a fixed lateral offset and two canned target speeds. It raced, but it could not be called driving: it never reasoned about its own speed, chose no line, saw no rivals, and recovered from nothing. We replaced it with a per-Car driver that turns one tick of perception into the same `CarInput` controls the player has.

The driver plans speed from the car's real limits: forward-looking centerline curvature over a 72-unit horizon, corner speed from the grip model's slip equilibrium (`v = sqrt(LATERAL_GRIP_RATE * slip / κ)`) capped by steering rate and full-lock curvature, then a braking backward pass using `BRAKE_ACCEL` with margin. It picks its own line (apex bias toward the inside of the dominant upcoming corner, smoothed), lifts or dodges for slower Cars only while on a collision course, and reverses out when stuck or facing away off-track. Perception is a new seam: `AiView` (own state, the Track, every Car's pose/velocity), built in `Sim::tick`; the driver never touches physics or Car state. The Track gained arc-length queries (`total_length`, `point_at_arc`, `centerline_frame`) to support sampling; `spawn_pose` is reimplemented on top, behavior unchanged.

Replay determinism is load-bearing (the sim is a bit-for-bit function of the input stream), so the driver holds no RNG, no clock, and no external state: its only memory (smoothed line offset, stuck/reverse counters) re-evolves identically from the same inputs, seeded per grid slot by a deterministic personality hash (skill 0.88–1.0, aggression 0.5–1.0). There is still no rubber-banding — personalities are fixed, and the lap-1 spread (~2 s) persists. If per-driver variation turns out unwanted, collapsing to a single skill is a one-line change in `AiDriver::new`.

Validated empirically, not by formula: a full 4-AI race on the sample circuit finishes in 4921 ticks with 0 wall contacts, 0 off-track ticks, 0.17% drift, leader laps ~25.4 s — pinned by `ai_field_races_three_laps_cleanly`. Corner apexes sit at ~13–14 u/s against a ~24.6 cruise, pinned by the speed-profile test.

Considered and rejected: scoring-based or randomized planners (unverifiable against replay determinism); accelerating through overtakes instead of lift-and-pass (ramming risk; the measured pass costs ~2 s behind a parked car, which is correct caution); keeping the waypoint follower with tuned constants (caps out at line-following, answers none of the ask).

## Fixed pace and clean passing (#54)

The menu fixes one opponent pace for the Race, retained on restart and shown in
the Race and results: Touring uses 0.70, Club 0.85, and Race 1.0 of the planned
speed envelope. Each grid slot retains its deterministic 0.88–1.0 skill and
0.5–1.0 aggression; pace does not alter aggression, engine, braking, grip,
collision response, or speed according to race position.

A pass commits to one rival and one lateral corridor. Clearance considers
current and projected rival lateral movement. A blocked corridor, lost target,
or six-second attempt causes an abort and 1.5 seconds of following before a
new attempt. A clear move completes only when the rival is seven units behind.
The driver never changes the side of an active commitment. Following a stopped
Car on the road does not count as being stuck; reverse recovery remains for
unobstructed failure to make progress or facing away off-road.

Established overlap means the Cars' nose-to-tail extents overlap along the
local Track direction (center gap at most 4.4 units, with 0.01 tolerance), with
centers on distinct lateral lines (more than 0.5 units apart). The AI Opponent
reserves the other Car's occupied side until fully clear: no defensive move or
apex choice may close a corridor of one Car width plus 0.6 units (3.2 total).
Both sides receive the same protection, including across the Track seam. When
the road cannot hold all reserved corridors, hold the current line and slow
rather than select a new defensive side. This is a local sportsmanship rule,
not an entitlement to force another Car off the road.

Encounter tests exercise the policy in motion, with pass counts, contact
severity and time near rivals. A clean field Race is not required to invent
overtakes; solo pace ordering and purpose-built passing opportunities measure
different contracts. Static perception tests pin the turn-in and seam overlap
boundaries; moving encounters prove that steering decisions avoid contact.
