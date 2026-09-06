# Rust 2D Game Stack — Awesome-List Mining

> Source boundary: curated awesome-list corpus only (via context-awesome
> `search_awesome_items` / `get_awesome_items`). Every claim below cites the
> canonical URL from its awesome-list record plus the endorsing list(s).
> No code changes; no secondary write-ups used as authority.

## Verdict

- **Default for a top-down racer: macroquad on miniquad.** `macroquad`
  is endorsed by 5 awesome lists (strongest host `ellisonleao/magictools`,
  17,252 stars) and its companion list `ozkriff/awesome-quads` (254 stars,
  141 items) is the densest mine of 2D/WASM examples: Rapier integration,
  web quickstart templates, top-down shooters, and Verlet physics demos.
  Steal its single-file game loop, `cargo-webquad` / `good-web-game` WASM
  deploy path, and `macroquad_rapier_interface` before writing custom physics.
- **Bevy only if we want ECS + fixed-timestep + rollback later.**
  `bevyengine/bevy` is endorsed by 5 lists (strongest
  `rust-unofficial/awesome-rust`, 59,204 stars). The mineable parts are not
  the renderer but the patterns: `bevy-cheatbook` fixed-timestep schedules,
  `Extreme Bevy` rollback netcode, and `bevy_rapier` / `bevy_xpbd` (Avian)
  physics plugins. Heavier than needed for coast-only arcade drift.
- **Physics: Rapier2D first, custom circle/Verlet fallback.**
  `dimforge/rapier` is endorsed by 4 lists (strongest
  `stevinz/awesome-game-engine-dev`, 1,406 stars) and already has both a
  Bevy plugin and a macroquad bridge plus a JS/WASM build (`rapier.rs` docs
  listed under JavaScript). For a top-down car, Rapier2D kinematic bodies +
  manual drift model beats a full rigid-body car; `circle2d` and the Verlet
  playground show the minimal custom alternative.
- **WASM story survives either answer:** miniquad ("everything built on top
  of it supports WASM too") → macroquad → `macroquad_quickstart` (web + PC
  template) → `good-web-game` / `cargo-webquad` / `quad-storage` /
  `quad-rand`. Bevy WASM works but pays a larger download + longer builds;
  macroquad is the boring WASM pick.

## Candidates / examples — what to steal

| # | Candidate | What to steal for top-down racer | Source links (canonical URL + endorsing lists) |
|---|-----------|----------------------------------|------------------------------------------------|
| 1 | macroquad — simple 2D-first Rust game library | Game-loop shape (`#[macroquad::main]`, per-frame `next_frame().await`), 2D drawing/texture API sized for one-file racer; base for rows 9–12. 5 endorsements; strongest host 17,252 stars. | Canonical: https://github.com/not-fl3/macroquad — hosts: `ozkriff/awesome-quads` (Game engines, 254★), `ellisonleao/magictools` (17,252★), `correia-jpv/fucking-magictools` (23★), `stevinz/awesome-game-engine-dev` (1,406★), `ccamel/awesome-ccamel` (28★) |
| 2 | miniquad — low-level graphics base under macroquad | WASM portability guarantee ("Supports WASM. Therefore everything built on top of it supports WASM too"); window/input/audio seam if macroquad feels too high-level. 2 endorsements. | Canonical: https://github.com/not-fl3/miniquad — hosts: `ozkriff/awesome-quads` (Game engines, 254★), `stevinz/awesome-game-engine-dev` (Libraries › Rust, 1,406★) |
| 3 | Bevy — data-driven 2D/3D ECS engine | Fixed-timestep schedules, ECS car/track/systems split, `bevyengine.org/assets/#games` gallery of jam racers; official examples as fixed-step reference. 5 endorsements; strongest 59,204★. | Canonical: https://github.com/bevyengine/bevy — hosts: `stevinz/awesome-game-engine-dev` (1,406★), `jslee02/awesome-entity-component-system` (697★), `nightswatchgames/awesome-rust-gamedev` (109★), `rust-unofficial/awesome-rust` (59,204★), `correia-jpv/fucking-awesome-rust` (17★). Site: https://bevyengine.org/ ; examples: https://github.com/bevyengine/bevy/tree/latest/examples#examples |
| 4 | Rapier — 2D/3D physics, perf-focused | Rapier2D colliders for track walls + kinematic car body; shares lineage with `nphysics`; official JS/WASM port proves browser path. 4 endorsements. | Canonical: https://github.com/dimforge/rapier — hosts: `stevinz/awesome-game-engine-dev` (1,406★), `nightswatchgames/awesome-rust-gamedev` (109★), `axiomecg/awesome-threejs` (974★), `xiaomingx/awesome-threejs` (6★). WASM port: https://rapier.rs/docs/user_guides/javascript/getting_started_js |
| 5 | bevy_rapier — Rapier plugin for Bevy | How to wire Rapier2D into an ECS fixed-step loop (colliders, joints, debug render) if Q0 picks Bevy; car-controller mine in its examples dir. 2 endorsements. | Canonical: https://github.com/dimforge/bevy_rapier — hosts: `d-bucur/awesome-bevy` (Physics, 77★), `coderonion/awesome-rust-list` (49★) |
| 6 | bevy_xpbd (now Avian) — pure-Rust ECS physics | Lighter alternative to Rapier for 2D arcade collision; read its determinism/fixed-step docs before choosing Rapier. Listed in rust-gamedev corpus. | Canonical: https://github.com/Jondolf/bevy_xpbd — via `nightswatchgames/awesome-rust-gamedev` (109★, 56 items retrieved) |
| 7 | Fyrox — 3D+2D Rust engine | Scene/track-editor pattern and 2D sprite pipeline; heaviest option — mine only its asset/scripting layout, not the engine. 3 endorsements. | Canonical: https://github.com/FyroxEngine/Fyrox — hosts: `ccamel/awesome-ccamel` (28★), `tribixbite/awesome` (81★), `r44cx/stars` (3★); also via `nightswatchgames/awesome-rust-gamedev` |
| 8 | ggez — 2D-focused "good games easily" framework | Immediate-mode 2D + `conf.toml` + resource loading; closest Bevy-free 2D API if macroquad is too minimal. Listed in rust-gamedev corpus. | Canonical: https://github.com/ggez/ggez — via `nightswatchgames/awesome-rust-gamedev` (109★) |
| 9 | macroquad_rapier_interface — Rapier2D + macroquad bridge | Copy-paste starting point: stepping Rapier2D inside a macroquad frame loop, collider→sprite mapping. | Canonical: https://github.com/Kenkron/macroquad_rapier_interface — via `ozkriff/awesome-quads` (Libraries › Integrations, 254★) |
| 10 | macroquad_quickstart — web + PC template | Opinionated template "specifically focused on targeting the web and PC": build scripts, itch/WASM deploy, input/audio skeleton. | Canonical: https://github.com/brettchalupa/macroquad_quickstart — via `ozkriff/awesome-quads` (Example usage, 254★) |
| 11 | Macroquad Game with ECS and Rapier physics (rodneylab tutorial) | Steal the hecs-ECS + Rapier + macroquad composition (systems split without Bevy) — closest architecture to "Bevy patterns on macroquad". | Canonical: https://rodneylab.com/macroquad-rapier-ecs — via `ozkriff/awesome-quads` (Publications, 254★); also top hit for `rapier physics` search |
| 12 | RecWars — multiplayer top-down tank shooter in browser (macroquad) | Mineable 2D top-down controller: tank accel/turn, wall collision, browser networking (`martin-t/cvars` + naia/nakama demos nearby). | Canonical: https://github.com/martin-t/rec-wars — via `ozkriff/awesome-quads` (254★). Related: https://github.com/martin-t/cvars (in-game console), https://github.com/naia-lib/naia/tree/main/demos/macroquad, https://github.com/heroiclabs/fishgame-macroquad |
| 13 | WASM deploy chain: good-web-game + cargo-webquad + quad-* helpers | `good-web-game` crate to port to web; `cargo-webquad` debug-on-web helper; `quad-storage` (Web Storage persistence for ghosts/bests), `quad-rand` (wasm-friendly RNG for track gen), `quad-snd`. | Canonicals: https://github.com/not-fl3/good-web-game ; https://github.com/not-fl3/cargo-webquad ; https://github.com/optozorax/quad-storage ; https://github.com/not-fl3/quad-rand ; https://github.com/not-fl3/quad-snd — all via `ozkriff/awesome-quads` (254★) |
| 14 | Fixed-step / netcode patterns: bevy-cheatbook + Extreme Bevy + Platformer in Bevy | Cheatbook fixed-timestep + rollback-netcode post + platformer series = deterministic sim + ghost/replay + deferred PvP pattern source. | Canonicals: https://bevy-cheatbook.github.io/ ; https://johanhelsing.studio/posts/extreme-bevy ; https://youtube.com/playlist?list=PL6uRoaCCw7GMujF_6PtzvkrZBlB_ZKWyZ — all via `nightswatchgames/awesome-rust-gamedev` (109★) |
| 15 | Minimal-physics fallback: circle2d + Verlet Physics Playground | If Rapier is overkill: circle-only collisions (`circle2d`) and Verlet integration playground — matches coast-only drift with custom yaw/grip code. | Canonicals: https://github.com/koalefant/circle2d ; https://codeberg.org/polaris64/verlet-physics-playground-macroquad — via `ozkriff/awesome-quads` (254★) |

## List notes (endorsement weight / staleness caveats)

- `ozkriff/awesome-quads` (https://github.com/ozkriff/awesome-quads, 254★, ~141 items, miniquad/macroquad-specific) — densest signal; all macroquad rows above come from it. Actively shaped around current miniquad ecosystem.
- `nightswatchgames/awesome-rust-gamedev` (https://github.com/nightswatchgames/awesome-rust-gamedev, 109★, 56 items retrieved in full) — Bevy/Rapier/ggez/Fyrox/godot-rust hub; small-star but topic-focused, so endorsement here counts more than stars suggest.
- `rust-unofficial/awesome-rust` (59,204★) and `ellisonleao/magictools` (17,252★) — mega-list endorsements for Bevy and macroquad respectively; high endorsement weight, low topic specificity.
- `stevinz/awesome-game-engine-dev` (1,406★) — cross-engine endorsement for Bevy, Rapier, miniquad; useful as a neutral second vote.
- `d-bucur/awesome-bevy` (77★) — Bevy-plugin-specific (bevy_rapier); niche but precisely on physics integration.
- Staleness: star counts and item membership are corpus snapshots, not live GitHub state — e.g. `bevy_xpbd` has since been renamed Avian, and `awesome-rust-gamedev` (109★) is a small community list. Re-verify activity (commits, Bevy-version support) before locking physics choice.
- Corpus gaps (honest): awesome lists did not surface a dedicated "top-down car controller" or "ghost/replay" item — rows 11/12/14 are the closest mineable proxies (ECS+Rapier composition, top-down tank shooter, rollback post). Avian/XPBD details and Bevy fixed-timestep specifics belong to DocsMiner/GitHubMiner streams.

## Queries run

- `rust game engine 2d`, `bevy engine`, `macroquad`, `rapier physics`, `avian physics 2d`, `rust wasm game`, `rust gamedev physics 2d engine`, `ggez tetra miniquad 2d`, `bevy rapier physics car controller`
- Full-list retrieval: `nightswatchgames/awesome-rust-gamedev` (56/56 items), `ozkriff/awesome-quads` (~141 items)
