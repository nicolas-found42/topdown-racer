# Gameplay maturity review

Date: 2026-09-09. Reviewed the current working tree, including the uncommitted implementations of #42 and #44, against starting commit `7f702a8bfc4d99ceab112644d6c6cf6d9bc191e1`.

The best next step is a precision racer with clear control, trustworthy timing, readable cornering, and contested positions. The existing deterministic simulation is a useful foundation. Its main weaknesses are the player-facing control and learning experience, the correctness of the finish, and the lack of racing situations that require different decisions. A larger car collection or a career economy would not resolve these weaknesses.

This is a research and gameplay review, not an implementation plan already approved for development. Proposals that extend the v1 exclusions are identified below.

## Evidence and limits

I interacted with the actual macOS game using Computer Use: menu, countdown, default driving without pedal input, manual/autopilot toggle, coast, window resizing, menu return, results, and restart. The preceding presentation review also inspected a temporary Track with all three scenery and Terrain Zone types. Normal gameplay uses the bundled Sample Circuit.

I inspected the Rust control, physics, AI, timing, camera, HUD, and persistence code. A temporary program used only the public core API to measure handling and a four-AI Race. It changed no game code. Its source and output are retained locally in `target/gameplay-review/probe.rs` and `target/gameplay-review/metrics.txt` (ignored build artifacts).

Computer Use provides intermittent key actions and screenshots, not a sustained human gamepad session. The handling measurements below are scripted experiments, not claims about a statistically representative human playtest. I did not listen to game audio through these tools; audio recommendations are based on the current implementation. External games were researched through primary documentation and code, not played locally. Proposed numerical tuning values are hypotheses to compare, not validated settings.

## Measured baseline

| Experiment | Result | Interpretation |
| --- | --- | --- |
| Four AI Cars on the bundled Track, Racing phase from tick zero | Finished at 4,921 ticks / 76.891 s | A short Race; little space for endurance strategy |
| Default four-AI Race position changes | Zero for every Car | This clean baseline provides no overtaking drama; it does not prove the AI cannot overtake in other scenarios |
| Default four-AI Race wall/off-Track ticks | Zero / zero for every Car | Preserve this competence while introducing more challenging interactions |
| Laps when the Race ends | `[3, 2, 2, 2]` | The whole Race stops when the first Car completes its third lap |
| First Car's lap times | 25.391, 25.750, 25.750 s | Deterministic, repeatable performance; no human difficulty calibration implied |
| Position of first Car when each lap is credited | Approximately `(0.002, 20.775)` | Timing triggers before the visible finish, which is centered around `(10, 0)` |
| Steady full throttle on a very wide straight after 10 s | 29.281 units/s | Reference speed for the input experiment; world units are not asserted to be metres |
| 8 fixed ticks / 125 ms of full-left steering at that speed | 22.92° heading change | A short digital input can produce a large correction |
| Same steering interval with throttle, coast, or brake | Same 22.92° yaw; ending speeds 28.381, 25.587, 21.197 respectively | The yaw-rate cap masks the weight-transfer steering distinction in this specific high-speed test; other speeds may behave differently |
| Full brake from 29.281 to at most 14 units/s | 0.297 s, 6.149 units | Braking decisions occupy a short spatial interval at this scale |
| Bundled Track | 498.310 units long, seven left turns, uniform width 14, no props or zones | No right-left transitions, authored width decisions, or braking landmarks yet |

At the fixed 80×45 view, a centered Car has 40 units to the horizontal screen edge but only 22.5 to the vertical edge. At 29.281 units/s, that is about 1.37 s versus 0.77 s of straight-ahead visible distance. These are geometric estimates, not human reaction-time requirements; bends and HUD overlap further affect practical visibility.

## 1. Make control ownership explicit

**Priority: immediate. Confidence: high.**

The normal shell enables an AI driver on Car 0 in `ShellSimulation::from_sim`. Starting a Race without driving input therefore accelerates and steers the player's Car. The menu provides no explanation of this mode, and neither the HUD nor the control screen identifies an active assist. `T` toggles it, but the binding is not presented in the menu.

There is an additional ownership discontinuity: when AI is enabled, a non-neutral player input replaces the whole AI input for that tick; neutral input allows the AI to resume. Releasing a key is therefore not necessarily coasting. Pressing only throttle can also temporarily replace the AI's steering with zero. These are valid demonstration controls but a confusing default for learning to drive.

**Recommendation:** default to manual control; expose a clearly labelled Demo/Autopilot option. Show a persistent assisted-driving indicator and a compact control guide. Keep assistance settings independent of opponent difficulty. Explicitly mark any lap with autopilot participation as assisted, rather than silently counting it as a manual personal best. Accessibility assists need their own documented eligibility policy; they should not automatically be treated as the same thing as full autopilot.

BeamNG documents its autonomous-driving toggle and exposes assistance behavior in settings; its steering UI also indicates intervention. Forza documents configurable assistance independently from overall difficulty. The transferable lesson is understandable ownership, rather than a blanket ban on assists. [BeamNG v0.25 input and gameplay notes](https://www.beamng.com/game/news/patch/beamng-drive-v0-25/), [Forza accessibility support](https://support.forzamotorsport.net/hc/en-us/articles/46524064744851-Forza-Motorsport-Accessibility-Support).

**Verify:** a new player can identify their Car, explain the controls and current assistance mode, and start/coast without hidden driver takeover. Hold-and-release control streams remain deterministic.

## 2. Make the finish line and personal bests trustworthy

**Priority: immediate, ahead of progression. Confidence: high.**

`advance_lap_progress` credits an ordered checkpoint when the Car enters a radius of local width × 1.5 while travelling in the incoming direction. This same proximity rule completes a lap. On the sample Track the radius is 21 units around `(0,0)`. The checker renderer puts the line on the outgoing straight at `half_width + 3`, approximately `(10,0)`. The probe confirms a lap completes around `(0,20.8)`, before the corner and before the painted checker. The visual promise and actual timing disagree.

The Race then enters Finished as soon as **any** Car reaches three laps. In the default probe three rivals lose their last-lap finish; the earlier live review also displayed unfinished lap slots. This removes the player's final fight when an opponent wins first.

Persistence stores one floating-point time in `best_lap.txt`, only on Race finish. There is no Track identifier/version, physics version, lap-validity flag, or autopilot provenance. Returning to the menu before Race finish does not use the persistence path. The displayed saved target cannot explain which configuration produced it; this becomes especially important before replacing the Track in #43.

**Recommendation:** define one directional finish gate in the Track model and use it for both rendering and simulation. Ordered progress validates the route; crossing the finish gate validates the lap. Detect the crossing between successive poses to avoid missed high-speed crossings. Separate the standing-start first-lap timing policy from flying-lap records. Introduce per-Car finish state and a bounded finish window so each active Car can take the flag, with explicit lapped/DNF classification where needed. Save an eligible best at lap completion, keyed by Track and handling version and labelled assistance policy.

Forza's first-party Rivals events explicitly distinguish clean laps, showing why a record needs eligibility rather than merely a low number. Dust Racing 2D also distinguishes stored records by race configuration. [Forza Bridgestone Rivals](https://forza.net/events/featured-rivals-bridgestone), [Dust Racing 2D README](https://github.com/juzzlin/DustRacing2D#races).

**Verify:** approaching, reversing through, cutting to, or sitting beside the checker never credits a lap. Proper crossings do. A losing player can complete the finish sequence. Quitting after a valid personal best preserves it; a new Track or physics version cannot silently inherit a misleading target.

## 3. Tune the keyboard-to-Car relationship before replacing the physics

**Priority: next playable slice. Confidence: measured risk; preferred tuning needs human testing.**

Keyboard input jumps directly between neutral and full steering. At the measured high speed, a 125 ms full-steer pulse rotates the Car 22.92°. Full brake, coast, and throttle all reach the same yaw cap over this interval. The simulation has weight transfer and lateral slip, but this input region does not give the player much room to express them. The short measured braking distance also compresses brake-point decisions.

**Recommendation:** prototype deterministic steering rise and release rates, with enough immediate response to feel connected and rapid countersteer recovery. Compare raw input with several short rise-time presets rather than choosing a heavily smoothed default by theory. Consider speed-sensitive steering authority only after measuring the ramp. Apply the adaptation to the human input boundary; do not accidentally change AI steering at the same time. Record the applied input for replays.

SuperTuxKart's actual player controller increases digital steering gradually and uses a separate return-to-center rate. This is direct implementation precedent for handling digital controls deliberately. [SuperTuxKart player controller](https://github.com/supertuxkart/stk-code/blob/master/src/karts/controller/player_controller.cpp#L254).

Keep the current momentum, braking, reverse, and surface distinctions as the starting point. A later handling pass could evaluate whether speed loss and progressive grip saturation give trail braking, lift-off rotation, and recovery clearly different outcomes. Art of rally explicitly makes countersteering, left-foot braking, and handbrake techniques part of its handling offer; these are useful capabilities to test, not an argument to copy rally balance into a GT circuit Car. [Art of rally handling](https://www.artofrally.com/#features).

**Verify:** novice players can make a small correction without an unintended large turn, while experienced players can catch a slide without fighting input lag. Compare clean-lap rate, wall contacts, steering reversals, and self-reported control across the same Track. Keep scripted input replay deterministic. Do not equate a faster automated lap with better human handling.

## 4. Give players time and information to commit to a corner

**Priority: pair with handling and #43. Confidence: high on visibility asymmetry; camera preference needs testing.**

The centered, fixed, world-aligned camera gives substantially less preview when moving vertically. The HUD lacks an overview, upcoming-corner cue, gap to the next Car, or off-screen rival indication. The current Track has no brake boards; kerbs mainly appear at the corner itself.

**Recommendation:** retain the fixed pixel scale and world orientation but compare a modest, bounded velocity-based camera offset against the centered camera. Smooth the target, then snap to the texel grid. Use velocity rather than instantaneous nose direction so the view does not whip around during a Drift. Give the offset a low-speed dead zone. Re-check canvas margins for the added offset.

Add stable braking landmarks before demanding corners and a compact Track overview with a clear player marker. Prefer small off-screen rival arrows or a nearby-gap display before adding a full proximity radar: top-down already reveals nearby overlap well. Forza's radar is specifically motivated by blind spots; transfer the information goal, not its cockpit-oriented layout. [Forza Proximity Radar](https://forza.net/news/forza-motorsport-update-10).

**Verify:** players can identify the next turn before reaching their braking decision on both horizontal and vertical approaches. Camera changes should not increase motion discomfort or obscure adjacent rivals. These are proposed playtest criteria, not evidence that one offset value is already correct.

## 5. Make grip, braking, and mistakes legible

**Priority: complete #45–#47, with restraint. Confidence: high on missing channels.**

The bake now communicates road versus gravel/grass, and the Car Sprite heading communicates rotation. The current presentation still offers little explanation of traction loss, braking, or a contact. The two-loop audio adapter derives engine pitch from speed and switches skid volume from a boolean Drift flag; it does not expose engine load, surface-specific rolling sound, or collision transients. This is code evidence, not an auditory quality judgment.

**Recommendation:** prioritize all-Car brake lights, grounded skid traces, surface dust, and graded tire feedback. Use smoke for sustained handbrake slip and keep it low enough that it does not obscure the exit. Add brief impact feedback proportional to severity, with an option to disable camera shake. Let throttle/load change the engine's character even at similar speed. Layer surface sound and opponent proximity selectively rather than creating a continuous wall of sound. Speed streaks are lower value if they hide brake references.

Circuit Superstars emphasizes tactile tire contact and rewarding vehicle control. The relevant target is a player who can tell when grip is available or lost; its extensive car roster and pit systems are not prerequisites. [Circuit Superstars developer description](https://store.steampowered.com/app/1097130/Circuit_Superstars/).

**Verify:** in short clips without HUD text, players can distinguish braking, controlled Drift, gravel travel, and a wall strike. Effects should not change the simulated outcome or prevent seeing the next apex. Keep #44's echoes as the observable source of controls.

## 6. Build a Track that asks different driving questions

**Priority: #43, after timing/control foundations. Confidence: high.**

The shipping circuit turns left at every vertex, uses one width everywhere, and has no authored prop or Terrain Zone. Its seven turns range from approximately 31° to 90°, but there is no direction-change sequence. More polished ground cannot supply the missing corner combinations.

**Recommendation:** give the new hillside Track a deliberate rhythm: a fast sweeper, a brake-heavy overtaking corner, a right-left transition, a tightening bend, and an exit whose speed matters down the next straight. Place useful landmarks before the braking point. A wide approach should permit two lines; a later narrowing should reward an earlier positioning decision. Keep decorative zones visually subordinate to the actual handling surface.

Author one memorable Track before many interchangeable ones. Validate each corner manually as well as with AI. Consider a proper grid on a straight so trailing Cars do not start around the bend as the current grid does. Different layouts and distinct vehicle-handling challenges are central to Circuit Superstars' mastery proposition, while its developers describe accessible control as the basis for wheel-to-wheel tactics. [Original Fire Games developer interview](https://collective.square-enix-games.com/en_US/news/interview-original-fire-games-circuit-superstars).

**Verify:** identify at least three places where two reasonable approaches trade entry speed, position, and exit speed. Test passing on inside and outside lines, not just zero-contact solo completion. The acceptance target is distinct decisions, not an arbitrary number of curves.

## 7. Evolve AI from clean circulation to enjoyable racecraft

**Priority: after varied Track and input tuning. Confidence: high on baseline; interaction quality needs scenarios.**

The existing driver already plans speed, chooses an apex bias, reacts to slower Cars, and reverses when stuck. Keep this. It is more capable than a waypoint follower. But the default measured Race has no position changes and near-identical later lap times. A clean lap benchmark alone will not measure the quality of a duel.

**Recommendation:** expose three initial pace settings, fixed for the Race, and keep pace separate from aggression. Retain the no-rubber-banding policy. Build deterministic scenarios for a blocked inside line, established overlap at turn-in, an outside pass, a late-braking player, a slow rejoin, and a faster Car starting behind. Give a pass a committed side and an abort path; respect a Car that has earned space alongside. Use bounded, seeded variation in decisions only if it improves racing, not random steering errors or hidden power boosts.

Sony's Sophy account shows that blanket collision aversion can produce overly timid racing, while insufficient constraints permit bullying. The arXiv paper *Fair Play in the Fast Lane* specifically studies space and defensive-movement constraints. These support assessing pace, assertiveness, and etiquette separately; they do not justify importing a reinforcement-learning or game-theory stack here. [Sony AI sportsmanship account](https://ai.sony/blog/dont-cross-that-line-how-our-ai-agent-learned-sportsmanship), [Huang et al., arXiv 2503.03774](https://arxiv.org/abs/2503.03774).

**Verify:** a meaningfully faster Car can make a clean pass in a designed passing scenario; the defender does not weave or squeeze an established overlap; aggression changes decisions without changing physical capability. Track near-rival time, completed pass attempts, abandoned attempts, and contact severity alongside lap time. Do not demand overtakes in every Race or manufacture them to meet a metric.

## 8. Add an immediate, useful learning loop

**Priority: after timing is trustworthy. Confidence: high. Scope extension beyond v1.**

A saved best is currently the only persistent target. During a Race there is no sector comparison or personal-best delta. The results table shows laps but gives no obvious next improvement. A beginner who loses learns little about where or why.

**Recommendation:** add Practice/Time Trial on the same Track, immediate reset, a valid-lap indicator, sector splits, and a personal-best comparison. Then add an optional non-colliding replay of the player's own best lap. Allow a small lead offset and hide it when it obscures the Car. A later corner-practice mode can restart a single difficult section with a repeatable entry speed.

Trackmania exposes personal-best ghosts and medal targets; Gran Turismo's Circuit Experience teaches sectors and its Licence Centre uses short driving challenges. These are complementary precedents for showing progress and isolating skills. [Trackmania access/features](https://www.trackmania.com/access?lang=en), [Gran Turismo 7 first-party overview](https://www.gran-turismo.com/au/news/00_1796083.html). Gran Turismo Sport's documented ghost offset and sector reset illustrate how a reference can remain nearby while learning. [GT Sport update 1.41](https://www.gran-turismo.com/gb/gtsport/news/ps4/detail/?id=%2F00_5457155).

**Verify:** after a slower lap, the player can name the section that lost time and retry it quickly. Show small improvements even when finishing position stays unchanged. Avoid grading a standing-start lap against a flying-lap ghost.

## Supporting improvements

| Improvement | Why it matters here | First useful version |
| --- | --- | --- |
| Pause and deliberate recovery | ESC currently exits to the menu and starting again rebuilds the Race; reverse exists but is undiscoverable | Pause/Resume/Restart/Menu; explain reverse; optional safe reset with explicit lap invalidation and no shortcut gain |
| Input and accessibility settings | Keyboard is the only player input; fixed UI sizes clip at small windows | Control help and remapping, UI scale, high-contrast player marker, independent assist settings; gamepad as a separate scope expansion |
| Results that suggest another attempt | Numbered rows and lap times do not explain a loss | Highlight the player, personal-best change, clean laps, finished/DNF/lapped status, then the weakest sector once measured |
| Identity and overview | Liveries help locally, but the player and rival gaps become unclear when the pack separates | Player number/marker, useful opponent names, gap ahead/behind, simple overview |
| Track/car/configuration-scoped records | #43 changes the Track and later tuning changes the performance envelope | Track content identity, handling version, assistance policy, migration that never silently compares incompatible laps |
| Small progression | A single best-time target limits return motivation | Bronze/silver/gold targets and a short three-event cup after there are genuinely different Tracks; avoid grind currencies initially |

Dust Racing 2D offers a relevant small-project precedent for difficulty selection, Race/Time Trial/Duel modes, pause, remapping, and simple Track-unlock goals. Use it as evidence that modest racing games benefit from complete loops, not as a checklist to copy wholesale. [Dust Racing 2D README](https://github.com/juzzlin/DustRacing2D).

## What to defer

Defer tire/fuel strategy, damage repair, many car classes, weather, multiplayer, an economy, and a broad campaign until the basic Race is worth repeating. These features are not intrinsically bad. They would add balancing and content work before the measured control, timing, and duel issues are addressed.

Do not add automatic speed catch-up, forced lead changes, or drift boosts merely to manufacture excitement: those would move away from the current fair, momentum-based circuit-racing intent. Preserve the deterministic core and explicit driver inputs. Evaluate a physics-engine replacement only if a concrete handling or collision requirement cannot be met cleanly by the present model.

## Suggested order and success gates

1. **Trust:** explicit manual/default control, correct shared finish gate, per-Car finish sequence, eligible/versioned best-lap persistence. A new player understands who drives; lap events match the checker.
2. **Control and readability:** compare keyboard response variants, camera preview, and brake/grip/surface feedback. New players complete clean laps; experienced players can make intentional small corrections.
3. **A worthwhile duel:** finish #43 with varied corner demands, add fixed difficulty presets, and validate racecraft scenarios. Faster driving can earn a pass while defensive positioning remains useful.
4. **A reason to retry:** Practice, sector comparison, personal-best replay, clear results, and modest mastery targets. Players can explain their next improvement before restarting.

No calendar estimates are claimed. The first gate is mostly correctness/product behavior; the second needs repeated human comparison; the third combines Track authoring with interaction tuning; the fourth depends on stable timing and record eligibility.

## Scope and existing decisions

The v1 spec explicitly excludes remapping, gamepad, and ghost/input-replay recording. The practice/replay and accessibility recommendations reopen those product exclusions; they are not missing requirements in #42/#44. #43 already covers the new Track; #45–#47 cover much of the visual feedback. Integrate the recommendations into that work rather than duplicating tickets.

No proposal requires overriding ADR-0001's engine-free custom physics or ADR-0002's deterministic driver/no rubber-banding. A bounded camera offset can preserve ADR-0003's fixed density, nearest sampling, world orientation, and integer texel mapping. Changing the zoom itself would require a separate decision.

## Search coverage

Every available search-provider MCP was used, plus native web search. Different tools of one provider are listed where used; local-only search and feedback endpoints were not mistaken for additional web-search providers.

| Provider | Search performed | Contribution |
| --- | --- | --- |
| Native web search | Circuit Superstars, art of rally, Gran Turismo, Forza, Trackmania, Sony AI, BeamNG; opened first-party pages and source code | Primary feature, assist, practice, feedback, and sportsmanship evidence |
| Firecrawl web search | Circuit Superstars handling, ghosts, difficulty | Discovery; community snippets were not treated as verified game design facts |
| Firecrawl developer search | SuperTuxKart control/AI behavior | Led to official BeamNG input/assistant release notes |
| Firecrawl paper search | Gran Turismo Sophy, human racing, difficulty and sportsmanship | Cross-checked the research framing; no unsupported claim of a fun/difficulty result from time-trial papers |
| Firecrawl GitHub research search | Dust Racing controls, camera and timing | Retrieved the project's first-party README |
| arXiv MCP | Gran Turismo racing; racing sportsmanship/difficulty | Found the fair-play paper and distinguished time-trial speed research from human racing interaction |
| Context Awesome MCP | Racing resources | Discovered Dust Racing 2D and TORCS; verified selected project facts against their owners, not list popularity |
| GitHub code-search MCP | Literal steering patterns in Dust Racing 2D and SuperTuxKart | Located steering code; followed through to the full first-party player controller |

Some initial official-site URLs failed or returned no useful body (Circuit Superstars home, a GT7 manual route, and a redirected Forza support page). Usable first-party alternatives were retrieved through the developer storefront, publisher interview, official news, source repositories, and indexed official support content. No conclusions depend on a failed page.

## Relevant local implementation references

- [Player control ownership and mapping](../../crates/topdown-racer/src/lib.rs)
- [Camera](../../crates/topdown-racer/src/camera.rs)
- [Simulation, lap progress, and finish condition](../../crates/core/src/simulation.rs)
- [Painted start/finish geometry](../../crates/topdown-racer/src/track_geometry.rs)
- [AI driver](../../crates/core/src/ai.rs)
- [Bundled Track](../../crates/core/data/tracks/sample-circuit.json)
- [Best-lap persistence](../../crates/topdown-racer/src/best_lap.rs)
- [HUD](../../crates/topdown-racer/src/hud.rs), [results](../../crates/topdown-racer/src/results.rs), [audio](../../crates/topdown-racer/src/audio.rs)
- [v1 scope and decisions](../specs/0001-racing-game-v1.md)
