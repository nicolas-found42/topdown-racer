# Free pixel-art assets for a top-down GT racer: pack survey

Scope: free car sprites, road/track tiles, environment props, and FX sheets
usable in a commercial title without a non-commercial restriction. Every
license claim below was read on the pack's own page (or in its shipped
`License.txt`), not taken from a search snippet. Pixel dimensions marked
"measured" were read from the actual downloaded files with `sips`; the rest
are stated by the author on the pack page. Anything whose license could not
be confirmed on a primary page is marked **unverified** and excluded from
the recommendation.

## Verdict

- **No single free pack meets the bar out of the box, so: packs as a base,
  original authorship for the hero cars.** Nothing free combines true crisp
  pixel art, 25–50 px single-heading GT cars, 4+ liveries, and a unified
  palette. The honest split: prototype and ship tracks/props/FX from CC0
  Kenney packs, but draw the cars original (in one chosen palette, §2).
- **Top 3 candidates:** (1) [Kenney Racing Pack](https://kenney.nl/assets/racing-pack) —
  the only CC0 complete kit (cars + roads + props + skid marks, SVG sources
  included); (2) [Pixel Vehicles by MinZinn](https://minzinn.itch.io/pixelvehicles) —
  best true-pixel-art cars with 8 liveries each, CC-BY 4.0;
  (3) [2D Top Down Pixel Art Car Pack by marcusvh](https://marcusvh.itch.io/2d-cars) —
  true pixel-art GT-style cars in 4 colors, permissive custom license.

## 1. Asset packs by category

### Car sprites

| Pack | License + terms | Dimensions | Style notes (fit for crisp top-down GT) | Base for modification? |
|---|---|---|---|---|
| [Kenney Racing Pack](https://kenney.nl/assets/racing-pack) — `PNG/Cars/`, 50 files | **CC0.** Pack page shows "License: Creative Commons CC0" ([page](https://kenney.nl/assets/racing-pack)); shipped `License.txt` repeats "Creative Commons Zero, CC0 … free to use in personal, educational and commercial projects"; studio-wide grant: "all game assets on the asset pages are public domain licensed (CC0)" ([support](https://kenney.nl/support)). No attribution required. | **Measured:** full cars ~70×131 px (e.g. `car_red_1.png` 71×131), small variants 40×70 px. 5 body styles × 5 colors (black/blue/green/red/yellow) × full/small. | NOT pixel art — smooth vector-rendered sprites. Readable and consistent, but would need a pixelize + palette-quantize pass to sit with pixel art. Oversized for the 25–50 px target (see §3). Best craft-per-euro at price zero, wrong medium. | **Yes, best in class.** Ships `Vector/*.svg` sources — recoloring, livery stripes, and reshaping are trivial compared to editing pixels. |
| [Pixel Vehicles by MinZinn](https://minzinn.itch.io/pixelvehicles) | **CC-BY 4.0 — attribution required.** Page text: "it's free to use in any project. license : Creative Commons Attribution v4.0 International"; author confirms commercial use OK in page comments ("am I able to use these commercially" → "yes man"). So: commercial OK, credit mandatory. | **Author-stated:** 16×16 / 32×32 tile grid; 23 vehicle types × 8 color variants (red/green/yellow/blue/magenta/brown/black/white per devlog); 8-directional sprites, 12 fps animations. Per-car px footprint not published on the page. | True pixel art and the only pack clearing the "4+ colorways" bar with margin (8). Catch: **8-directional** sheets suit grid/8-way movement, not free-rotation GT physics — using one heading per car wastes 7/8 of the sheet, and rotating pixel art at runtime smears it. Everyday cars/vans/trucks, no low-slung GT silhouette. | Partially. CC-BY allows derivatives (keep credit); recolors easy, but reshaping 8 directions × 23 types is heavy. Better as placeholder traffic than hero cars. |
| [2D Top Down Pixel Art Car Pack by marcusvh](https://marcusvh.itch.io/2d-cars) | **Custom permissive, attribution requested.** Page text: "Free to use for any project, commercial or non-commerical, however I do kindly ask that you give credit." Not a standard license; credit is asked, not contractually demanded — treat as BY-like in practice. | **Unstated on the page** (no tile size; sprites not measured — single download, no per-sprite spec). 4 cars (compact/coupe/sedan/sports) + semi truck + trailer, each in 4 colors (red/green/blue/special). | True single-heading top-down pixel art — the correct format for free rotation, and the sports-car body is the closest free silhouette to a GT car in this survey. Small set, one artist, consistent shading. Exact px unknown, so §3 fit is unconfirmed. | **Yes, with care.** Permissive terms allow edits; small set means a livery/palette-unification pass is cheap. Confirm dimensions after download before committing. |
| [Free Top Down Car Sprites by Unlucky Studio](https://opengameart.org/content/free-top-down-car-sprites-by-unlucky-studio) (OpenGameArt) | **CC0.** OGA license field reads CC0; page text: "royalty free … use them in your personal or commercial projects. Credits are not necessary"; shipped `Read Me.txt`: "free to use … in your commercial projects". | **Measured:** 256×256 px canvases (`Audi.png`, `Car.png`, `taxi.png` all 256×256), car occupying part of the canvas. 9 vehicles + animated police/ambulance light frames. | NOT pixel art — high-res smooth renders ("high resolution .png format" per author). Single colorway each, inconsistent detail level across vehicles, dated shading. Downscaling 256 px art to 25–50 px destroys readability; these were drawn to be shown large. | Weak base. CC0 allows anything, but there is no clean pixel structure to edit — effectively reference images, not sprites. |

### Road / track tilesets

| Pack | License + terms | Dimensions | Style notes | Base for modification? |
|---|---|---|---|---|
| [Kenney Racing Pack](https://kenney.nl/assets/racing-pack) — `PNG/Tiles/` (asphalt/dirt/sand roads + grass/dirt/sand land) | **CC0** (same grant as above). | **Measured:** road tiles 128×128 px (`road_asphalt01/03.png`); car length ≈ tile width (~131 px car on 128 px tile). Full curve/straight/junction/start-finish coverage in three surfaces plus land fills. | Same vector-smooth caveat as the cars, but roads suffer less: large flat shapes pixelize cleanly, and the piece coverage (corners, crossings, start line) is the completeness story here. Tile grid is 128 px — plan resampling to the chosen px-per-unit (§3), don't author around 128. | **Yes.** SVG road sources included; easiest path to a matching custom track set. |
| [Free CC0 Top Down Tileset Template by rgsdev](https://opengameart.org/content/free-cc0-top-down-tileset-template-pixel-art) (OpenGameArt) | **CC0.** OGA license field CC0; "Credits is not needed." | **Author-stated:** 16×16 px tiles, 5 color variations. | True pixel art but a **generic dungeon-template**, not roads — no asphalt, kerbs, or racing pieces. Author calls it "basic … useful for prototyping". Only useful as grass/infield filler or prototyping grid. | Yes (CC0, tiny tiles easy to extend), but extending it into a racing set means drawing the actual road pieces anyway. |

### Environment props (trees, tire stacks, brake boards)

| Pack | License + terms | Dimensions | Style notes | Base for modification? |
|---|---|---|---|---|
| [Kenney Racing Pack](https://kenney.nl/assets/racing-pack) — `PNG/Objects/`, 39 files | **CC0** (same grant). | **Measured:** tire stacks 56×56 (`tires_white.png`), cones 46×44, barriers 210×62, oil slick 109×95. Also tents, tribunes, arrows, barrels, brake-board-style signs. | The only free source found that covers actual racing furniture (tires/barriers/cones/oil). Consistent with the pack's cars/roads, so the full track reads as one set. Trees are absent — pair with a CC0 foliage source or draw 3–4 pine/canopy blobs in the chosen palette. | **Yes** — SVG sources; trivial to add team-color stripes to barriers/tents. |

### FX sheets (skid marks, dust, smoke)

| Pack | License + terms | Dimensions | Style notes | Base for modification? |
|---|---|---|---|---|
| [Kenney Racing Pack](https://kenney.nl/assets/racing-pack) — `skidmark_short/long_*.png` | **CC0** (same grant). | **Measured:** short skid 60×16 px; long variants in-pack. | Drop-in drift marks. Only gap-free FX in the survey: static decals, so no animation pipeline needed — stamp with fade. | Yes; tintable to asphalt color. |
| [Kenney Smoke Particles](https://kenney.nl/assets/smoke-particles) — 70 files | **CC0** ("Download this package (70 assets) for free, CC0 licensed!"). | **Measured:** white puff ~375×378 px; sets for white puff / black smoke / explosion / flash. | Soft pre-rendered smoke, not pixel art — at drift-puff scale (10–20 px on screen) downscaling reads fine as translucent smoke. Use sparingly; heavy use over pixel art looks pasted-on. | Limited need — puffs are generic; tint via blend, no structural edits needed. |
| [Kenney Particle Pack](https://kenney.nl/assets/particle-pack) — 80 files | **CC0** ("Download this package (80 assets) for free, CC0 licensed!"). | **Author-stated tile size 512×512; measured** `smoke_08.png` 512×512. Circles, sparks, scratches, smoke frames with transparency. | Best dust/drift-smoke donor: small radial sprites scale down to 8–16 px dust cleanly. Includes scratch/spark shapes usable for collision sparks. | Yes; generic shapes, recolor freely. |

### Deliberately excluded

- **Liberated Pixel Cup (LPC) collections** — dual-licensed
  [CC BY-SA 3.0 + GPL 3.0](https://github.com/OpenGameArt/LiberatedPixelCup)
  ("Assets are dual licensed under Creative Commons Attribution-ShareAlike
  version 3.0 & GNU GPL version 3.0"). Excluded on two independent grounds:
  (1) share-alike/copyleft terms are heavier than the CC0/CC-BY budget for
  this project and would need a legal call, not an art call; (2) the repo is
  RPG content (`sprite/character/*`, tilesets — humanoid paper-doll bases),
  with no vehicles at all, so there is nothing to take even if the license
  were acceptable.
- **CraftPix "Top Down Trucks and Cars" and similar freebies** — excluded:
  CraftPix free downloads gate behind accounts and the license tiers mix
  personal/commercial terms per pack; no clean primary-page CC0/CC-BY
  statement was found. Per the contract, unverified = excluded.
- **Aim Studios "Customizable Cars" (itch.io)** — a promising pixel-art
  customizable set, but license/dimensions could not be confirmed on its
  primary page in this pass; marked **unverified**, excluded from the top 3.
  Revisit if the car shortlist needs a fourth.

## 2. Palette standards

All four are free to use (palettes are facts/short color lists, not
licensed art); links are the canonical Lospec entries with downloads in
PNG/ASE/GPL/HEX.

| Palette | Source | Colors | Typical use / why it matters here |
|---|---|---|---|
| [Endesga 32](https://lospec.com/palette-list/endesga-32) | ENDESGA, via Lospec | **32** | Made for the game [NYKRA](http://nykra.com/); warm-leaning generalist with strong red/orange/teal ramps — good GT-livery reds and asphalt teals in one set. ~203k downloads, the most-used 32-color set on Lospec. |
| [DawnBringer 32 (DB32)](https://lospec.com/palette-list/dawnbringer-32) | DawnBringer (PixelJoint), via Lospec | **32** | The game-jam standard: full hue coverage with matched ramps, designed so any two colors in a ramp gradient-blend. Ships as a default preset in major pixel tools. Safe pick when multiple artists/placeholder packs must coexist. |
| [Apollo](https://lospec.com/palette-list/apollo) | AdamCYounis, via Lospec | **46** | Larger, moodier, blue/green-leaning with deep near-blacks — best of the four for night-race readability and grass/asphalt separation, at the cost of more discipline (46 colors invite banding noise on 30 px cars). |
| [PICO-8](https://lospec.com/palette-list/pico-8) | Lexaloffle fantasy console, via Lospec | **16** | Deliberate-constraint pick: 16 colors force posterized, ultra-readable sprites. Viable only if the whole game (cars + track + UI) commits; too tight for shaded GT liveries alongside detailed tracks. |

Recommendation: **Endesga 32 or DB32** as the single project palette
(32 colors is the sweet spot for 25–50 px cars: enough for 3-step body
ramps + glass + tire + outline, few enough to keep liveries consistent).
Quantize any adopted pack art to the chosen palette on import so
placeholders and original art converge instead of drifting apart.

## 3. Sprite-size conventions

Ground truth first — measured numbers from the surveyed packs:

| Source | Car length | Road / tile width | Ratio |
|---|---|---|---|
| Kenney Racing Pack (measured) | ~131 px (full), 70 px (small) | 128 px tiles | ~1 : 1 car-length to road-tile |
| MinZinn Pixel Vehicles (author-stated grid) | fits 16×16 / 32×32 tiles | 16 or 32 px | car ≈ 1–2 tiles long |
| RGSDev template (author-stated) | n/a (no cars) | 16×16 px | 16 px grid baseline |
| Unlucky Studio (measured) | 256 px canvas (art ~150–200 px) | n/a | hi-res, downscale-only |

Classic reference points, for the design language rather than exact px
(old masters don't publish sprite sheets): Atari's **Super Sprint**
(1986) "set the bar for top-down perspective racing games"
([Top Gear retrospective](https://www.topgear.com/car-news/gaming/remembering-classic-games-super-sprint-1986)) —
fixed-screen, whole-track-in-view, chunky single-color cars with strong
silhouettes; and Codemasters' **Micro Machines** (1991), "a top-down
racing game: players observe races from above" with miniaturised toy
vehicles on scrolling household tracks
([Micro Machines](https://en.wikipedia.org/wiki/Micro_Machines_(video_game))) —
the follow-cam school this project belongs to, where cars stay small
relative to the viewport and readability comes from silhouette + livery
contrast, not detail.

Mapping to the 6/8/12 px-per-unit decision (car target 25–50 px long):

| px-per-unit | 25 px car | 50 px car | Kenney full (131 px) becomes | Kenney small (70 px) becomes | 128 px road tile becomes |
|---|---|---|---|---|---|
| 6 | ~4.2 units | ~8.3 units | ~22 units (too big, resample) | ~12 units | ~21 units wide |
| **8 (recommended)** | **~3 units** | **~6 units** | ~16 units (resample to ~0.3×) | ~9 units (resample to ~0.5×) | ~16 units wide |
| 12 | ~2 units | ~4 units | ~11 units (resample) | ~6 units | ~11 units wide |

Notes:

- **8 px/unit is the sanity winner.** A 32–40 px hero car (4–5 units, a
  believable GT length in car-lengths of track width) lands mid-range;
  MinZinn's 32 px grid and the RGSDev 16 px grid both divide evenly into
  8 px/unit math (4 units and 2 units per tile); Kenney's 128 px tiles
  become exactly 16 units — clean numbers for track authoring.
- Car-to-road-width stays near the Kenney 1:1 convention: at 8 px/unit a
  4–5-unit car on an 8–16-unit road reads like the classics (car clearly
  narrower than the road, room for two-wide racing on a 1-tile road).
- 6 px/unit forces a 25 px car into chunkier pixels (good chunk, less
  livery detail); 12 px/unit makes 50 px cars luxurious but doubles sprite
  work and texture memory for identical gameplay. Either is viable; 8 is
  the middle that keeps both options open.
- Whichever density wins, **resample Kenney art once at import**
  (128 px tiles and 131 px cars do not divide into 6/8/12 cleanly except
  128/8) and author original cars natively at the chosen density — never
  scale pixel cars at runtime.

## 4. Recommendation

**Top 3, ranked:**

1. **[Kenney Racing Pack](https://kenney.nl/assets/racing-pack)** (CC0) —
   adopt for everything except hero cars: road/track pieces in three
   surfaces, tire/cone/barrier/oil props, skid marks, plus SVG sources for
   the inevitable custom pieces (start gantry, kerbs in team colors).
   Resample 128 px tiles → 16-unit tiles at 8 px/unit.
2. **[Pixel Vehicles by MinZinn](https://minzinn.itch.io/pixelvehicles)**
   (CC-BY 4.0, credit required) — adopt as **AI-traffic / background**
   cars only: 23 types × 8 liveries is unmatched variety-per-effort, and
   8-directional sheets are fine for slow traffic. Keep the attribution
   line in the credits screen.
3. **[2D Top Down Pixel Art Car Pack by marcusvh](https://marcusvh.itch.io/2d-cars)**
   (permissive + credit asked) — best free **style reference and
   placeholder** for the hero GT car: single-heading top-down pixel art
   with a sports body. Verify its px size on download; either run it as
   the prototype player car or trace over it for proportions.

**Final verdict: packs as a base — but author the hero cars original.**
The honest gap in the free ecosystem is exactly the project's center:
a crisp, palette-unified, 25–50 px GT car with 4+ liveries, drawn
single-heading for free rotation. Kenney covers track/props/FX at CC0
quality no solo artist should redraw; MinZinn covers traffic variety;
neither covers the hero. So: build the world from packs (quantized to
Endesga 32 or DB32 on import), draw 1 GT body × 4–6 palette liveries
natively at 8 px/unit (~32–40 px long) as the first original-art milestone,
and use the marcusvh sports car strictly as proportion reference until
then. FX (Kenney skid marks + smoke/dust) and palettes need no original
work at all.
