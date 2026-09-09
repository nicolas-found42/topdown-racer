//! The world bake: a pure, headless, deterministic rasterization of the
//! render geometry + theme into the world pixel canvas at 8 texels per
//! world unit (the spec's fixed density).
//!
//! [`bake_world`] consumes only a [`TrackRenderGeometry`] (the tested
//! geometry seam) and a `Theme` — no Bevy types, no window, no RNG, no
//! clock — so identical input yields byte-identical pixels, assertable
//! without a window. Layers paint in painter's order (grass field, terrain
//! zones, road, centerline dashes, kerbs, edge lines, start line,
//! guardrails), mirroring the quad z-order of the flat-quad renderer the
//! baked canvas will replace; wiring into the game is a later ticket.
//!
//! Art rules (all colors from the project palette, see [`crate::palette`]):
//! - grass field: mowing bands 8 units tall along world Y, alternating
//!   GRASS_BASE / GRASS_SHADOW, speckled on the 4x4 ordered-dither lattice
//!   with GRASS_LIGHT (base bands) or GRASS_BASE (shadow bands)
//! - terrain zones: Sand = DRY_GOLD speckled DIRT_LIGHT; Dirt = DIRT_BASE
//!   speckled DIRT_SHADOW; DarkGrass = GRASS_SHADOW speckled GRASS_BASE
//! - road: ASPHALT_BASE grain, speckled ASPHALT_DARKEST and highlighted
//!   ASPHALT_LIGHT; gravel segments: DIRT_LIGHT with DIRT_BASE speckle and
//!   a SUNLIT_STRAW highlight
//! - edge lines and centerline dashes: CREAM_HIGHLIGHT
//! - kerbs: alternating KERB_BASE / KERB_BONE by quad index (the stripes)
//! - start line: (row + col) even -> CREAM_HIGHLIGHT, else VOID_SHADOW
//! - guardrails: WORN_EDGE steel

use glam::Vec2;
use topdown_racer_core::track::{Surface, Theme, ZoneKind};

use crate::palette::{
    ASPHALT_BASE, ASPHALT_DARKEST, ASPHALT_LIGHT, CREAM_HIGHLIGHT, DIRT_BASE, DIRT_LIGHT,
    DIRT_SHADOW, DRY_GOLD, GRASS_BASE, GRASS_LIGHT, GRASS_SHADOW, KERB_BASE, KERB_BONE,
    SUNLIT_STRAW, VOID_SHADOW, WORN_EDGE,
};
use crate::track_geometry::{Quad, TrackRenderGeometry};

/// Native texel density of the baked world canvas and the sprite pipeline.
pub const TEXELS_PER_UNIT: f32 = 8.0;

/// Grass (and world) margin around the geometry's bounding box, in units.
pub const CANVAS_MARGIN_UNITS: f32 = 16.0;

/// Height of one mowing band on the grass field, in world units.
const MOWING_BAND_UNITS: f32 = 8.0;

/// Ordered-dither threshold for the fine speckle (grass, asphalt).
const SPECKLE: u8 = 2;
/// Ordered-dither threshold for the coarse speckle (zones, gravel).
const SPECKLE_COARSE: u8 = 3;
/// Ordered-dither threshold above which granular patterns highlight.
const HIGHLIGHT: u8 = 14;

/// Classic 4x4 ordered-dither (Bayer) lattice: the speckle rule for every
/// granular pattern, keyed on canvas texel coordinates so patterns never
/// seam across quad joints.
const BAYER_4X4: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

fn bayer(px: u32, py: u32) -> u8 {
    BAYER_4X4[((py % 4) << 2 | (px % 4)) as usize]
}

/// Palette hex -> opaque RGBA8 texel.
fn rgba(rgb: u32) -> [u8; 4] {
    [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 0xFF]
}

/// The baked world: one RGBA8 pixel canvas covering the circuit at
/// [`TEXELS_PER_UNIT`], rows top-down (row 0 = max world Y, matching the
/// image layouts the render adapter will consume).
#[derive(Debug, Clone, PartialEq)]
pub struct WorldCanvas {
    /// Texels across (world X).
    pub width: u32,
    /// Texels tall (world Y).
    pub height: u32,
    /// RGBA8, row-major, row 0 = top.
    pub pixels: Vec<u8>,
    /// World position of the low corner (min X, min Y) of texel
    /// (0, height - 1).
    origin: Vec2,
}

impl WorldCanvas {
    /// Canvas bounds for a geometry: the bounding box of every quad,
    /// expanded by [`CANVAS_MARGIN_UNITS`] per side and snapped outward to
    /// whole texels.
    fn for_geometry(geometry: &TrackRenderGeometry) -> WorldCanvas {
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        let mut grow = |quad: &Quad| {
            for v in quad.verts {
                min = min.min(v);
                max = max.max(v);
            }
        };
        for road in &geometry.road {
            grow(&road.quad);
        }
        for quad in &geometry.edge_lines {
            grow(quad);
        }
        for quad in &geometry.dashes {
            grow(quad);
        }
        for quad in &geometry.kerbs {
            grow(quad);
        }
        for quad in &geometry.guardrails {
            grow(quad);
        }
        for quad in &geometry.start_line {
            grow(quad);
        }
        for zone in &geometry.zones {
            grow(&zone.quad);
        }
        if !min.is_finite() {
            min = Vec2::ZERO;
            max = Vec2::ZERO;
        }
        let tpu = TEXELS_PER_UNIT;
        let low_x = ((min.x - CANVAS_MARGIN_UNITS) * tpu).floor();
        let low_y = ((min.y - CANVAS_MARGIN_UNITS) * tpu).floor();
        let high_x = ((max.x + CANVAS_MARGIN_UNITS) * tpu).ceil();
        let high_y = ((max.y + CANVAS_MARGIN_UNITS) * tpu).ceil();
        let width = ((high_x - low_x) as i64).max(1) as u32;
        let height = ((high_y - low_y) as i64).max(1) as u32;
        WorldCanvas {
            width,
            height,
            pixels: vec![0; (width * height) as usize * 4],
            origin: Vec2::new(low_x / tpu, low_y / tpu),
        }
    }

    /// World position of the low corner of the texel grid.
    pub fn world_origin(&self) -> Vec2 {
        self.origin
    }

    /// Texel (column, row) covering `world`, or `None` outside the canvas.
    pub fn texel_of(&self, world: Vec2) -> Option<(u32, u32)> {
        let px = ((world.x - self.origin.x) * TEXELS_PER_UNIT).floor();
        let py = self.height as f32 - 1.0 - ((world.y - self.origin.y) * TEXELS_PER_UNIT).floor();
        if px < 0.0 || py < 0.0 || px >= self.width as f32 || py >= self.height as f32 {
            None
        } else {
            Some((px as u32, py as u32))
        }
    }

    /// World-space center of a texel.
    pub fn texel_center(&self, px: u32, py: u32) -> Vec2 {
        Vec2::new(
            self.origin.x + (px as f32 + 0.5) / TEXELS_PER_UNIT,
            self.origin.y + (self.height as f32 - 1.0 - py as f32 + 0.5) / TEXELS_PER_UNIT,
        )
    }

    /// The RGBA color baked at the texel covering `world`.
    pub fn sample_world(&self, world: Vec2) -> Option<[u8; 4]> {
        let (px, py) = self.texel_of(world)?;
        let i = (py as usize * self.width as usize + px as usize) * 4;
        Some(self.pixels[i..i + 4].try_into().unwrap())
    }

    fn set(&mut self, px: u32, py: u32, rgba: [u8; 4]) {
        let i = (py as usize * self.width as usize + px as usize) * 4;
        self.pixels[i..i + 4].copy_from_slice(&rgba);
    }

    /// Texel bounding box of a quad (1 texel of slack), clamped to the
    /// canvas; `None` when the quad misses the canvas.
    fn texel_bounds(&self, quad: &Quad) -> Option<(u32, u32, u32, u32)> {
        let (mut min, mut max) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
        for v in quad.verts {
            min = min.min(v);
            max = max.max(v);
        }
        let px0 = (((min.x - self.origin.x) * TEXELS_PER_UNIT).floor() as i64 - 1).max(0);
        let px1 = (((max.x - self.origin.x) * TEXELS_PER_UNIT).ceil() as i64 + 1)
            .min(self.width as i64 - 1);
        let py0 = (self.height as i64
            - 1
            - ((max.y - self.origin.y) * TEXELS_PER_UNIT).ceil() as i64
            - 1)
        .max(0);
        let py1 =
            (self.height as i64 - 1 - ((min.y - self.origin.y) * TEXELS_PER_UNIT).floor() as i64
                + 1)
            .min(self.height as i64 - 1);
        if px0 > px1 || py0 > py1 {
            None
        } else {
            Some((px0 as u32, px1 as u32, py0 as u32, py1 as u32))
        }
    }

    /// Paints `color_of(px, py)` wherever the quad covers the texel center.
    fn paint_quad(&mut self, quad: &Quad, color_of: impl Fn(u32, u32) -> [u8; 4]) {
        let Some((px0, px1, py0, py1)) = self.texel_bounds(quad) else {
            return;
        };
        for py in py0..=py1 {
            for px in px0..=px1 {
                if point_in_quad(self.texel_center(px, py), quad) {
                    self.set(px, py, color_of(px, py));
                }
            }
        }
    }
}

/// Bakes the static world into one pixel canvas: grass field, terrain
/// zones, road, dashes, kerbs, edge lines, start line, guardrails — in
/// that painter's order, matching the flat-quad z-order.
pub fn bake_world(geometry: &TrackRenderGeometry, theme: Theme) -> WorldCanvas {
    let mut canvas = WorldCanvas::for_geometry(geometry);
    match theme {
        Theme::Hillside => paint_hillside(&mut canvas, geometry),
    }
    canvas
}

/// The hillside-golden art: the one theme v1 ships.
fn paint_hillside(canvas: &mut WorldCanvas, geometry: &TrackRenderGeometry) {
    // Grass field covers the whole canvas: mowing bands with dither.
    for py in 0..canvas.height {
        let world_y = canvas.texel_center(0, py).y;
        for px in 0..canvas.width {
            canvas.set(px, py, grass_hillside(px, py, world_y));
        }
    }
    for zone in &geometry.zones {
        let kind = zone.kind;
        canvas.paint_quad(&zone.quad, move |px, py| match kind {
            ZoneKind::Sand => sand(px, py),
            ZoneKind::Dirt => dirt(px, py),
            ZoneKind::DarkGrass => dark_grass(px, py),
        });
    }
    for road in &geometry.road {
        match road.surface {
            Surface::Grass => continue, // geometry never emits it
            surface => canvas.paint_quad(&road.quad, move |px, py| match surface {
                Surface::Road => asphalt(px, py),
                Surface::Gravel => gravel(px, py),
                Surface::Grass => unreachable!("filtered above"),
            }),
        }
    }
    for quad in &geometry.dashes {
        canvas.paint_quad(quad, |_, _| rgba(CREAM_HIGHLIGHT));
    }
    for (i, quad) in geometry.kerbs.iter().enumerate() {
        let rgb = if i % 2 == 0 { KERB_BASE } else { KERB_BONE };
        canvas.paint_quad(quad, move |_, _| rgba(rgb));
    }
    for quad in &geometry.edge_lines {
        canvas.paint_quad(quad, |_, _| rgba(CREAM_HIGHLIGHT));
    }
    for (i, quad) in geometry.start_line.iter().enumerate() {
        let rgb = if (i / 10 + i % 10) % 2 == 0 {
            CREAM_HIGHLIGHT
        } else {
            VOID_SHADOW
        };
        canvas.paint_quad(quad, move |_, _| rgba(rgb));
    }
    for quad in &geometry.guardrails {
        canvas.paint_quad(quad, |_, _| rgba(WORN_EDGE));
    }
}

/// Sand zone: dry gold with a light speckle.
fn sand(px: u32, py: u32) -> [u8; 4] {
    if bayer(px, py) < SPECKLE_COARSE {
        rgba(DIRT_LIGHT)
    } else {
        rgba(DRY_GOLD)
    }
}

/// Dirt zone: bare earth with a dark speckle.
fn dirt(px: u32, py: u32) -> [u8; 4] {
    if bayer(px, py) < SPECKLE_COARSE {
        rgba(DIRT_SHADOW)
    } else {
        rgba(DIRT_BASE)
    }
}

/// Dark grass zone: shaded grass mottled with the base green.
fn dark_grass(px: u32, py: u32) -> [u8; 4] {
    if bayer(px, py) < SPECKLE_COARSE {
        rgba(GRASS_BASE)
    } else {
        rgba(GRASS_SHADOW)
    }
}

/// Mowing bands [`MOWING_BAND_UNITS`] tall; each band base-colored with a
/// light speckle on the dither lattice's two lowest cells.
fn grass_hillside(px: u32, py: u32, world_y: f32) -> [u8; 4] {
    let band = (world_y / MOWING_BAND_UNITS).floor() as i64;
    if band.rem_euclid(2) == 0 {
        if bayer(px, py) < SPECKLE {
            rgba(GRASS_LIGHT)
        } else {
            rgba(GRASS_BASE)
        }
    } else if bayer(px, py) < SPECKLE {
        rgba(GRASS_BASE)
    } else {
        rgba(GRASS_SHADOW)
    }
}

/// Asphalt grain: base charcoal, speckled darkest, highlighted light.
fn asphalt(px: u32, py: u32) -> [u8; 4] {
    match bayer(px, py) {
        d if d < SPECKLE => rgba(ASPHALT_DARKEST),
        d if d > HIGHLIGHT => rgba(ASPHALT_LIGHT),
        _ => rgba(ASPHALT_BASE),
    }
}

/// Gravel: light tan base, dark speckle, straw highlight.
fn gravel(px: u32, py: u32) -> [u8; 4] {
    match bayer(px, py) {
        d if d < SPECKLE_COARSE => rgba(DIRT_BASE),
        d if d > HIGHLIGHT => rgba(SUNLIT_STRAW),
        _ => rgba(DIRT_LIGHT),
    }
}

/// Point-in-convex-quad by sign consistency of the edge cross products;
/// zero-length edges (fan-triangulated zones) and on-edge points count as
/// inside, so shared quad joints are painted by both neighbors.
fn point_in_quad(p: Vec2, quad: &Quad) -> bool {
    const EPS: f32 = 1e-5;
    let v = &quad.verts;
    let (mut pos, mut neg) = (false, false);
    for i in 0..4 {
        let a = v[i];
        let b = v[(i + 1) & 3];
        let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        if cross > EPS {
            pos = true;
        } else if cross < -EPS {
            neg = true;
        }
    }
    !(pos && neg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track_geometry::RoadQuad;
    use topdown_racer_core::track::{Track, SAMPLE_CIRCUIT};

    /// 2x2-unit road quad at the origin: the minimal geometry the canvas
    /// math is hand-checkable against (origin (-16,-16), 272x272 texels).
    fn flat_road_geometry() -> TrackRenderGeometry {
        TrackRenderGeometry {
            road: vec![RoadQuad {
                quad: Quad {
                    verts: [
                        Vec2::ZERO,
                        Vec2::new(2.0, 0.0),
                        Vec2::new(2.0, 2.0),
                        Vec2::new(0.0, 2.0),
                    ],
                },
                surface: Surface::Road,
            }],
            ..TrackRenderGeometry::default()
        }
    }

    #[test]
    fn canvas_covers_geometry_with_margin_at_native_density() {
        let canvas = bake_world(&flat_road_geometry(), Theme::Hillside);
        // Bounding box [-16, 18]^2 (2x2 quad + 16-unit margin), 8 texels
        // per unit.
        assert_eq!((canvas.width, canvas.height), (272, 272));
        assert_eq!(canvas.world_origin(), Vec2::new(-16.0, -16.0));
        // Texel <-> world round-trips inside half a texel.
        let world = Vec2::new(1.0, 1.0);
        let (px, py) = canvas.texel_of(world).unwrap();
        let center = canvas.texel_center(px, py);
        let half_texel = 0.5 / TEXELS_PER_UNIT;
        assert!(
            (center.x - world.x).abs() <= half_texel && (center.y - world.y).abs() <= half_texel
        );
        assert!(canvas.texel_of(Vec2::new(-100.0, 0.0)).is_none());
        // Every texel opaque.
        assert!(canvas.pixels.iter().skip(3).step_by(4).all(|&a| a == 0xFF));
    }

    #[test]
    fn grass_field_bands_and_dither() {
        let canvas = bake_world(&flat_road_geometry(), Theme::Hillside);
        // Origin (-16,-16) puts texel centers on -16 + odd/16, so these
        // hand-picked samples miss the speckle lattice (Bayer >= 2) and
        // pin the raw band colors: bands 8 units tall along world Y.
        assert_eq!(
            canvas.sample_world(Vec2::new(-14.875, -13.0)),
            Some(rgba(GRASS_BASE))
        ); // band -2
        assert_eq!(
            canvas.sample_world(Vec2::new(-14.875, -4.0)),
            Some(rgba(GRASS_SHADOW))
        ); // band -1
        assert_eq!(
            canvas.sample_world(Vec2::new(-14.875, 5.0)),
            Some(rgba(GRASS_BASE))
        ); // band 0
        assert_eq!(
            canvas.sample_world(Vec2::new(-14.875, 9.5)),
            Some(rgba(GRASS_SHADOW))
        ); // band 1
        assert_eq!(
            canvas.sample_world(Vec2::new(-14.875, 16.5)),
            Some(rgba(GRASS_BASE))
        ); // band 2
           // Dither: any 4x4 texel window inside one band holds exactly two
           // speckle texels (the lattice's two lowest cells) tinted with the
           // band's light color, the rest the band base.
        let (px0, py0) = (8, 200); // rows 200..=203 all inside the band -1 stretch
        let (mut speckle, mut base) = (0, 0);
        for py in py0..py0 + 4 {
            for px in px0..px0 + 4 {
                match canvas.sample_world(canvas.texel_center(px, py)).unwrap() {
                    c if c == rgba(GRASS_BASE) => speckle += 1,
                    c if c == rgba(GRASS_SHADOW) => base += 1,
                    c => panic!("unexpected grass texel {c:?}"),
                }
            }
        }
        assert_eq!((speckle, base), (2, 14));
    }

    /// The bake fixture: an axis-aligned circuit with a uniform bottom
    /// straight (width 12), a width ramp along the bottom-right stretch
    /// (12 -> 24 over seg 1), a gravel segment (seg 2), three terrain
    /// zones above the track, and the start line overridden to the
    /// uniform top straight (seg 7). Ramp vertices sit on axis-aligned
    /// runs, so golden texel positions are hand-derivable.
    fn bake_fixture() -> (Track, TrackRenderGeometry, WorldCanvas) {
        let text = r#"{
            "name": "Bake Fixture",
            "widths": [12.0, 12.0, 24.0, 24.0, 24.0, 20.0, 16.0, 12.0, 12.0, 12.0],
            "points": [[0,0],[100,0],[140,0],[140,60],[100,60],[75,60],[50,60],[25,60],[0,60],[0,0]],
            "surfaces": [{"start": 2, "surface": "gravel"}],
            "terrain_zones": [
                {"kind": "sand", "polygon": [[60,85],[90,85],[90,95],[60,95]]},
                {"kind": "dirt", "polygon": [[110,85],[125,85],[125,95],[110,95]]},
                {"kind": "dark_grass", "polygon": [[10,85],[30,85],[30,95],[10,95]]}
            ],
            "start_line": {"segment": 7}
        }"#;
        let track = Track::parse(text).expect("fixture parses");
        let geo = crate::track_geometry::build_track_geometry(&track);
        let canvas = bake_world(&geo, track.theme);
        (track, geo, canvas)
    }

    /// Contiguous texels of road-family color (asphalt grain + cream
    /// lines) along the vertical scan at `x`, walked outward from the
    /// segment's centerline `y`.
    fn road_run(canvas: &WorldCanvas, x: f32, y: f32) -> u32 {
        let family = [
            ASPHALT_DARKEST,
            ASPHALT_BASE,
            ASPHALT_LIGHT,
            CREAM_HIGHLIGHT,
        ];
        let is_family = |texel: [u8; 4]| family.iter().any(|&rgb| texel == rgba(rgb));
        let (px, cy) = canvas
            .texel_of(Vec2::new(x, y))
            .expect("scan inside canvas");
        assert!(
            is_family(canvas.sample_world(Vec2::new(x, y)).unwrap()),
            "scan start must be on the road"
        );
        let mut run = 1u32;
        for py in cy + 1..canvas.height {
            if !is_family(canvas.sample_world(canvas.texel_center(px, py)).unwrap()) {
                break;
            }
            run += 1;
        }
        for py in (0..cy).rev() {
            if !is_family(canvas.sample_world(canvas.texel_center(px, py)).unwrap()) {
                break;
            }
            run += 1;
        }
        run
    }

    #[test]
    fn straight_road_bakes_to_full_width_in_texels() {
        let (track, _geo, canvas) = bake_fixture();
        // Vertical scan across the uniform bottom straight (width 12) at
        // an x clear of dashes and corner miters: exactly 96 texels of
        // road (asphalt + cream edge lines); kerbs and guardrails beyond
        // stay out of the run.
        assert_eq!(road_run(&canvas, 45.7, 0.0), 96);
        assert_eq!(
            road_run(&canvas, 45.7, 0.0) as f32,
            2.0 * track.road_half_width_at_arc(45.7) * TEXELS_PER_UNIT
        );
        // Edge lines cream at the rim, asphalt inside; a dash bakes cream.
        assert_eq!(
            canvas.sample_world(Vec2::new(45.7, 5.9)),
            Some(rgba(CREAM_HIGHLIGHT))
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(45.7, -5.9)),
            Some(rgba(CREAM_HIGHLIGHT))
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(45.7, 5.5)),
            Some(rgba(ASPHALT_BASE))
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(8.0, 0.0)),
            Some(rgba(CREAM_HIGHLIGHT))
        );
        // Past the road's rim: plain grass, the run stops there.
        assert_eq!(
            canvas.sample_world(Vec2::new(45.7, -6.45)),
            Some(rgba(GRASS_SHADOW))
        );
    }

    #[test]
    fn ramp_road_bakes_to_the_ramped_width_in_texels() {
        let (_track, _geo, canvas) = bake_fixture();
        // Segment 1 has mitered edges (100,6)->(128,12) and
        // (100,-6)->(152,-12). At texel centers x=114.0625 and
        // x=124.0625, their vertical spans contain 133 and 159 texels.
        // These literal goldens are independent of the rasterizer.
        for (x, expected) in [(114.0, 133u32), (124.0, 159u32)] {
            assert_eq!(road_run(&canvas, x, 0.0), expected, "scan at x={x}");
        }
    }

    #[test]
    fn kerbs_alternate_red_and_bone_at_the_road_edge() {
        let (_track, _geo, canvas) = bake_fixture();
        // Corner at (0,0), global kerb indices 0..7 (uniform width 12 at
        // the opening corner). Even index -> KERB_BASE, odd -> KERB_BONE.
        // Incoming steps climb the left edge at x in [-6.9, -6] (the
        // negative-arc width lookup wraps the loop); outgoing steps run
        // along the bottom straight at y in [-6.9, -6].
        for (x, y, expected) in [
            (-6.45, 0.8, KERB_BASE),
            (-6.45, 2.4, KERB_BONE),
            (-6.45, 4.0, KERB_BASE),
            (-6.45, 5.6, KERB_BONE),
        ] {
            assert_eq!(
                canvas.sample_world(Vec2::new(x, y)),
                Some(rgba(expected)),
                "kerb stripe at ({x},{y})"
            );
        }
        assert_eq!(
            canvas.sample_world(Vec2::new(0.8, -6.45)),
            Some(rgba(KERB_BASE))
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(2.4, -6.45)),
            Some(rgba(KERB_BONE))
        );
        // The kerb sits outside the road: asphalt inside the edge, grass
        // past the kerb's outer rim.
        assert_eq!(
            canvas.sample_world(Vec2::new(0.8, -5.5)),
            Some(rgba(ASPHALT_BASE))
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(0.8, -7.5)),
            Some(rgba(GRASS_SHADOW))
        );
    }

    #[test]
    fn checker_line_bakes_at_the_overridden_segment() {
        let (_track, _geo, canvas) = bake_fixture();
        // Override = segment 7 ((25,60) -> (0,60), uniform width 12):
        // line center (16,60), 10 columns of 1.2 units spanning y in
        // [54,66], two 0.8-unit rows at x in [15.2,16.8]; (row+col) even
        // -> CREAM_HIGHLIGHT, odd -> VOID_SHADOW.
        assert_eq!(
            canvas.sample_world(Vec2::new(16.4, 65.4)),
            Some(rgba(CREAM_HIGHLIGHT)),
            "row 0, col 0"
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(15.6, 65.4)),
            Some(rgba(VOID_SHADOW)),
            "row 1, col 0"
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(16.4, 64.2)),
            Some(rgba(VOID_SHADOW)),
            "row 0, col 1"
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(15.6, 64.2)),
            Some(rgba(CREAM_HIGHLIGHT)),
            "row 1, col 1"
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(15.6, 54.6)),
            Some(rgba(CREAM_HIGHLIGHT)),
            "row 1, col 9"
        );
        // The default placement (segment 0, line center (9,0)) stays
        // plain road: no checker there.
        assert_eq!(
            canvas.sample_world(Vec2::new(8.6, -5.2)),
            Some(rgba(ASPHALT_BASE))
        );
    }
    #[test]
    fn bake_is_byte_identical_for_identical_input() {
        // The bundled circuit parsed twice -> same geometry ->
        // byte-identical canvases, every pixel and dimension.
        let bake = || {
            let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
            bake_world(
                &crate::track_geometry::build_track_geometry(&track),
                track.theme,
            )
        };
        assert_eq!(bake(), bake());
        // The fixture too: an independent rebuild bakes the same bytes.
        let (_, geo, canvas) = bake_fixture();
        assert_eq!(canvas, bake_world(&geo, Theme::Hillside));
    }

    #[test]
    fn sample_circuit_bakes_the_full_world() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let geo = crate::track_geometry::build_track_geometry(&track);
        let canvas = bake_world(&geo, track.theme);
        // The canvas bounds contain every emitted quad (inclusive).
        let origin = canvas.world_origin();
        let extent = Vec2::new(
            canvas.width as f32 / TEXELS_PER_UNIT,
            canvas.height as f32 / TEXELS_PER_UNIT,
        );
        for quad in geo
            .road
            .iter()
            .map(|r| r.quad)
            .chain(geo.edge_lines.iter().copied())
            .chain(geo.dashes.iter().copied())
            .chain(geo.kerbs.iter().copied())
            .chain(geo.guardrails.iter().copied())
            .chain(geo.start_line.iter().copied())
            .chain(geo.zones.iter().map(|z| z.quad))
        {
            for v in quad.verts {
                assert!(
                    v.x >= origin.x
                        && v.x <= origin.x + extent.x
                        && v.y >= origin.y
                        && v.y <= origin.y + extent.y,
                    "vertex {v:?} outside the canvas"
                );
            }
        }
        // Color-family census on the real circuit: every layer paints.
        let (mut asphalt, mut gravel, mut grass, mut kerb, mut cream) =
            (0u32, 0u32, 0u32, 0u32, 0u32);
        for i in (0..canvas.pixels.len()).step_by(4) {
            let rgb = ((canvas.pixels[i] as u32) << 16)
                | ((canvas.pixels[i + 1] as u32) << 8)
                | canvas.pixels[i + 2] as u32;
            match rgb {
                ASPHALT_DARKEST | ASPHALT_BASE | ASPHALT_LIGHT => asphalt += 1,
                DIRT_LIGHT | DIRT_BASE | SUNLIT_STRAW => gravel += 1,
                GRASS_BASE | GRASS_SHADOW | GRASS_LIGHT => grass += 1,
                KERB_BASE | KERB_BONE => kerb += 1,
                CREAM_HIGHLIGHT => cream += 1,
                _ => {}
            }
        }
        assert!(asphalt > 100_000, "asphalt {asphalt}");
        assert!(gravel > 10_000, "gravel {gravel}");
        assert!(grass > 1_000_000, "grass {grass}");
        assert!(kerb > 400, "kerb {kerb}");
        assert!(cream > 10_000, "cream {cream}");
        // Alpha opaque everywhere.
        assert!(canvas.pixels.iter().skip(3).step_by(4).all(|&a| a == 0xFF));
    }
    #[test]
    fn terrain_zones_and_gravel_paint_their_patterns() {
        let (_track, _geo, canvas) = bake_fixture();
        // Zone interiors take the zone palette at Bayer-neutral samples;
        // the grass field shows through just outside the sand patch.
        assert_eq!(
            canvas.sample_world(Vec2::new(75.0, 90.0)),
            Some(rgba(DRY_GOLD)),
            "sand zone"
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(117.5, 90.0)),
            Some(rgba(DIRT_BASE)),
            "dirt zone"
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(20.0, 90.0)),
            Some(rgba(GRASS_SHADOW)),
            "dark grass zone"
        );
        assert_eq!(
            canvas.sample_world(Vec2::new(75.0, 83.0)),
            Some(rgba(GRASS_BASE)),
            "grass outside the sand patch"
        );
        // The gravel segment (right vertical) bakes the gravel palette,
        // not asphalt.
        assert_eq!(
            canvas.sample_world(Vec2::new(141.125, 30.5)),
            Some(rgba(DIRT_LIGHT)),
            "gravel surface"
        );
    }
}
