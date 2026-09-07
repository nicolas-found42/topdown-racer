# Topdown Racer

Fresh native Rust top-down racing game. Solo developer v1: you versus AI opponents on a closed circuit.

## Language

**Track**:
A closed-loop circuit defined as a data-driven center polyline plus width and surfaces.
_Avoid_: level, map, course

**Car**:
The player- or AI-driven vehicle simulated as a point mass with heading, longitudinal accel/brake/drag and lateral grip with slip angle.
_Avoid_: ship, sprite, body

**Race**:
3 laps, you plus 3 AI cars, with live positions and best lap shown.
_Avoid_: match, round, heat

**AI Opponent**:
A decision-making driver that perceives the Track and other Cars each tick and answers with the same CarInput controls as the player: a speed plan derived from the car's real braking and grip limits, a self-chosen line through upcoming curvature, following and overtaking, and reverse recovery when stuck. No scripted racing line, fixed personalities per grid slot, no rubber-banding for v1.
_Avoid_: bot, NPC, ghost

**Drift**:
Controlled lateral slip from the grip model, distinct from uncontrolled wall-slide.
_Avoid_: skid, slide, powerslide
