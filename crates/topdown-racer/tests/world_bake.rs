use topdown_racer::{bake_world, build_track_geometry, TEXELS_PER_UNIT};
use topdown_racer_core::track::Track;

#[test]
fn caller_can_bake_render_geometry_without_starting_the_game() {
    let track = Track::parse(
        r#"{
            "name": "Headless bake",
            "width": 12,
            "points": [[0,0],[100,0],[100,60],[0,60],[0,0]],
            "surfaces": []
        }"#,
    )
    .unwrap();
    let geometry = build_track_geometry(&track);
    let canvas = bake_world(&geometry, track.theme);

    assert_eq!(TEXELS_PER_UNIT, 8.0);
    assert_eq!(
        canvas.pixels.len(),
        canvas.width as usize * canvas.height as usize * 4
    );
    // Inside the bottom straight, clear of markings: project asphalt.
    assert_eq!(
        canvas.sample_world(glam::Vec2::new(45.7, 2.0)),
        Some([0x23, 0x20, 0x2D, 0xFF])
    );
    assert_eq!(canvas, bake_world(&geometry, track.theme));
}

#[test]
fn grass_canvas_covers_the_fixed_camera_view_at_outer_track_bounds() {
    let track = Track::parse(
        r#"{"name":"Camera margin","width":12,
            "points":[[0,0],[100,0],[100,60],[0,60],[0,0]],"surfaces":[]}"#,
    )
    .unwrap();
    let canvas = bake_world(&build_track_geometry(&track), track.theme);
    // A Car at the right-hand wall can look 40 units farther right. Include
    // half a texel of slack for the camera's world-grid snapping.
    assert!(canvas
        .sample_world(glam::Vec2::new(152.0625, 30.0))
        .is_some());
    assert!(canvas
        .sample_world(glam::Vec2::new(-52.0625, 30.0))
        .is_some());
}

/// Issue #42: the baked canvas the shell uploads must render every world
/// layer — grass field, Terrain Zones, road, kerbs, edge lines, centerline
/// dashes, and the checkered start — fully opaque (no void shows through).
#[test]
fn baked_canvas_renders_every_world_layer() {
    use topdown_racer::palette;

    let track = Track::parse(
        r#"{
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
        }"#,
    )
    .unwrap();
    let geometry = build_track_geometry(&track);
    assert!(!geometry.dashes.is_empty(), "fixture needs a centerline");
    assert!(!geometry.edge_lines.is_empty(), "fixture needs edge lines");
    assert!(!geometry.kerbs.is_empty(), "fixture needs kerbs");
    assert!(
        !geometry.start_line.is_empty(),
        "fixture needs a start line"
    );
    assert!(!geometry.zones.is_empty(), "fixture needs Terrain Zones");
    let canvas = bake_world(&geometry, track.theme);

    let rgba = |rgb: u32| [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 0xFF];
    let sample = |x: f32, y: f32| canvas.sample_world(glam::Vec2::new(x, y));
    let centroid = |verts: &[glam::Vec2; 4]| {
        glam::Vec2::new(
            (verts[0].x + verts[1].x + verts[2].x + verts[3].x) / 4.0,
            (verts[0].y + verts[1].y + verts[2].y + verts[3].y) / 4.0,
        )
    };

    // Grass field far from the Track, and each Terrain Zone interior.
    assert_eq!(
        sample(75.0, 83.0),
        Some(rgba(palette::GRASS_BASE)),
        "grass field"
    );
    assert_eq!(
        sample(75.0, 90.0),
        Some(rgba(palette::DRY_GOLD)),
        "sand zone"
    );
    assert_eq!(
        sample(117.5, 90.0),
        Some(rgba(palette::DIRT_BASE)),
        "dirt zone"
    );
    assert_eq!(
        sample(20.0, 90.0),
        Some(rgba(palette::GRASS_SHADOW)),
        "dark grass zone"
    );
    // Road asphalt and the gravel segment.
    let asphalt = [
        palette::ASPHALT_DARKEST,
        palette::ASPHALT_BASE,
        palette::ASPHALT_LIGHT,
    ];
    assert!(
        asphalt
            .iter()
            .any(|&rgb| sample(45.7, 2.0) == Some(rgba(rgb))),
        "asphalt road, got {:?}",
        sample(45.7, 2.0)
    );
    assert_eq!(
        sample(141.125, 30.5),
        Some(rgba(palette::DIRT_LIGHT)),
        "gravel surface"
    );
    // Kerb stripes alternate red and bone at the road edge.
    assert_eq!(
        sample(-6.45, 0.8),
        Some(rgba(palette::KERB_BASE)),
        "kerb red"
    );
    assert_eq!(
        sample(-6.45, 2.4),
        Some(rgba(palette::KERB_BONE)),
        "kerb bone"
    );
    // Centerline dashes and edge lines bake cream.
    assert_eq!(
        sample(
            centroid(&geometry.dashes[0].verts).x,
            centroid(&geometry.dashes[0].verts).y
        ),
        Some(rgba(palette::CREAM_HIGHLIGHT)),
        "centerline dash"
    );
    assert_eq!(
        sample(
            centroid(&geometry.edge_lines[0].verts).x,
            centroid(&geometry.edge_lines[0].verts).y
        ),
        Some(rgba(palette::CREAM_HIGHLIGHT)),
        "edge line"
    );
    // Checkered start: every cell is cream or void-shadow, and both appear.
    let mut cream = 0u32;
    let mut shadow = 0u32;
    for cell in &geometry.start_line {
        let c = centroid(&cell.verts);
        match sample(c.x, c.y) {
            Some(px) if px == rgba(palette::CREAM_HIGHLIGHT) => cream += 1,
            Some(px) if px == rgba(palette::VOID_SHADOW) => shadow += 1,
            other => panic!("start cell at {c:?} baked {other:?}, want checker"),
        }
    }
    assert!(cream > 0 && shadow > 0, "checker needs both squares");
    // No void: every texel opaque.
    assert!(canvas.pixels.iter().skip(3).step_by(4).all(|&a| a == 0xFF));
}
