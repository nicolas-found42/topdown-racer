# Make braking, lost grip, surfaces, and impacts distinguishable during a Race

## What to build

A player can recognize braking, controlled Drift, gravel/off-Track travel, and a wall impact from coordinated visual and audible cues while still seeing the next apex. Complete a gameplay readability pass over the effects delivered by #45–#47 and add a small load/surface/impact audio layer.

## Evidence and intended behavior

#45–#47 already own skid marks, brake lights, smoke, dust, and speed streaks. Do not reimplement their emitters. The reviewed audio adapter used speed-based engine pitch and boolean Drift volume; engine load, surface sound, and impact transients were absent in code. Audio quality was not auditioned during that review.

Use observed per-Car controls and outcomes. Improve cue prioritization and tune density/lifetimes so multiple effects remain readable. Throttle/load can change engine character at similar speed; sustained tire slip and rough surfaces should sound distinct; an impact should trigger a bounded transient rather than continuous retriggering.

## Acceptance criteria

- [ ] Brake lights remain visible on all Cars from applied brake input, including AI Opponents. No unrelated cue guesses braking from deceleration alone.
- [ ] Existing marks, handbrake smoke, and surface dust keep their documented trigger contracts; density and lifetime tuning do not silently change #45–#47 scope.
- [ ] The player can distinguish the four target events in captured examples. Smoke, dust, and streaks do not conceal road limits or a nearby rival for a sustained corner approach.
- [ ] Engine sound differentiates throttle load from coasting at comparable speed without claiming a gearbox/RPM simulation that does not exist.
- [ ] Tire feedback varies smoothly with an observable slip/severity signal where available; adding a required snapshot output remains additive and does not change physics.
- [ ] Surface cues follow physical surface classification, never decorative Terrain Zones.
- [ ] Impact feedback scales within bounded limits and does not retrigger every tick during persistent contact. The first delivery uses audio and a brief Car-local cue; camera shake is excluded.
- [ ] Simultaneous Cars and particles obey documented voice/particle/decal budgets; repeated racing/restart does not leak entities or audio instances.
- [ ] A minimal effects-volume control supports mute, and leaving a Race silences its loops/transients. Focus/pause behavior follows the active lifecycle policy.
- [ ] Assets are original or license-compatible with project policy, with attribution where required; no external game's sounds/art are copied.
- [ ] Scripted scenarios verify cue triggers and bounded outputs. Computer Use verifies visual readability; an actual listening session verifies audio and is explicitly recorded, or audio audition remains honestly reported as outstanding.
- [ ] Required workspace checks pass; presentation effects do not change deterministic Race outcomes.

## Boundaries and design constraints

No full soundtrack, gearbox model, damage model, haptics, or wholesale audio middleware. This extends the earlier presentation spec's audio exclusion in a focused way; record that scope amendment locally rather than editing its parent issue. Preserve palette, z-order, fixed pixel scale, and snapshot-driven presentation.

## Blocked by

- #44 — Snapshot input echo: throttle, brake, handbrake.
- #45 — Skid-mark decal seam + shell wiring.
- #46 — Brake lights + tire smoke.
- #47 — Off-track dust + player speed streaks.

## Research basis

[Circuit Superstars' developer description](https://store.steampowered.com/app/1097130/Circuit_Superstars/) emphasizes tactile tire contact. The desired outcome is actionable feedback, not more visual volume.
