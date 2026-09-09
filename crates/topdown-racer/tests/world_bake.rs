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
