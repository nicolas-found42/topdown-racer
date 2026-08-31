# Deterministic hand-written simulation, no physics engine

The car is a hand-written, fixed-timestep, road-space slip model with a clamped lateral impulse. Same inputs plus same seed produce bit-identical simulation; Ghosts are pure input replays, so competition needs no netcode and no state storage beyond input timelines. Determinism is a hard constraint on every numeric choice in the simulation.

**Considered options**: Box2D-style libraries via planck or cannon (rejected: determinism control, engine-specific math variance, and dependency weight for what is roughly a hundred lines of model).
