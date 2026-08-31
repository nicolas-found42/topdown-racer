# Pseudo-3D camera over top-down

The research notes recommended top-down; we chose a javascript-racer-style pseudo-3D projection onto a segment list instead. This decision drives the Track representation (per-segment curve and hill values), the physics frame (road-space, not world-space), and the renderer (Canvas 2D projection).

**Considered options**: top-down Canvas 2D (deepest reference code, but the camera bar was set higher); 3D chase-cam (rejected: needs quality tiers, native-physics performance, and mobile soak testing that a small team pays for heavily).
