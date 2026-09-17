use glam::Vec2;
use topdown_racer::{
    awareness::{
        optional_awareness_visible, rival_indicator, CameraLead, CameraMode, OverviewProjection,
        MAX_CAMERA_LEAD,
    },
    bake_world, build_track_geometry, CAMERA_VIEW_SIZE, TEXELS_PER_UNIT,
};
use topdown_racer_core::track::{Track, HILLSIDE_CIRCUIT, SAMPLE_CIRCUIT};

#[test]
fn lead_smooths_velocity_and_keeps_player_visible_through_reversal_and_stop() {
    let mut lead = CameraLead::default();
    for velocity in [Vec2::ZERO, Vec2::new(1.9, 0.0)] {
        assert_eq!(lead.step(velocity, CameraMode::LookAhead), Vec2::ZERO);
    }
    let first = lead.step(Vec2::new(0.0, 30.0), CameraMode::LookAhead);
    assert!(first.y > 0.0 && first.y < MAX_CAMERA_LEAD);
    assert_eq!(lead.interpolated_offset(0, 0.5), first * 0.5);
    for velocity in [
        Vec2::new(0.0, 30.0),
        Vec2::new(-30.0, -30.0),
        Vec2::new(30.0, 0.0),
        Vec2::ZERO,
    ] {
        for _ in 0..400 {
            let previous = lead.offset();
            let offset = lead.step(velocity, CameraMode::LookAhead);
            assert!(offset.is_finite() && offset.length() <= MAX_CAMERA_LEAD + 0.001);
            assert!(offset.distance(previous) <= 1.601);
            // Whole Car plus half-texel snapping slack, even along the short view axis.
            assert!((CAMERA_VIEW_SIZE * 0.5 - offset.abs()).min_element() > 2.0);
        }
        if velocity == Vec2::ZERO {
            assert!(lead.offset().length() < 0.001);
        } else {
            assert!(lead.offset().dot(velocity.normalize()) > 9.99);
        }
    }
    lead.step(Vec2::X * 30.0, CameraMode::LookAhead);
    assert_eq!(
        lead.interpolated_offset(1, 1.0),
        Vec2::ZERO,
        "restart rejects old history before first tick"
    );
    assert_eq!(lead.step(Vec2::X, CameraMode::Centered), Vec2::ZERO);
}

#[test]
fn rival_cues_use_player_distance_but_camera_relative_edges() {
    let player = Vec2::new(100.0, 100.0);
    let camera = player + Vec2::new(0.0, 10.0);
    for direction in [
        Vec2::X,
        Vec2::NEG_X,
        Vec2::Y,
        Vec2::NEG_Y,
        Vec2::ONE.normalize(),
        -Vec2::ONE.normalize(),
    ] {
        let rival = player + direction * 60.0;
        let marker = rival_indicator(player, rival, camera).unwrap() - camera;
        assert!(marker.x.abs() <= 38.001 && marker.y.abs() <= 20.501);
        assert!(marker.dot(rival - camera) > 0.0);
        assert!(marker.perp_dot(rival - camera).abs() < 0.001);
        assert_eq!(
            rival_indicator(player, player + direction * 60.01, camera),
            None
        );
    }
    assert_eq!(
        rival_indicator(player, camera + Vec2::new(40.0, 22.5), camera),
        None
    );
    assert_eq!(rival_indicator(player, camera, camera), None);
    assert!(rival_indicator(player, camera + Vec2::new(0.0, 22.51), camera).is_some());
    assert_eq!(
        rival_indicator(player, camera + Vec2::new(0.0, 22.49), camera),
        None
    );
}

#[test]
fn overview_preserves_geometry_and_finish_seam_continuity() {
    for source in [SAMPLE_CIRCUIT, HILLSIDE_CIRCUIT] {
        let track = Track::parse(source).unwrap();
        let overview = OverviewProjection::new(&track.points);
        for point in &track.points {
            let projected = overview.project(*point);
            assert!((21.0..=37.001).contains(&projected.x));
            assert!((-16.0..=-5.999).contains(&projected.y));
        }
        let seam = track.points[0];
        assert_eq!(
            overview.project(seam),
            overview.project(*track.points.last().unwrap())
        );
        let before = overview.project(seam - Vec2::X * 0.01);
        let after = overview.project(seam + Vec2::X * 0.01);
        assert!(before.distance(after) < 0.02);
        let a = track.points[1];
        let b = track.points[2];
        assert!(
            overview
                .project(a.lerp(b, 0.5))
                .distance(overview.project(a).lerp(overview.project(b), 0.5))
                < 0.001
        );
    }
}

#[test]
fn every_track_boundary_and_allowed_lead_has_baked_coverage() {
    for source in [SAMPLE_CIRCUIT, HILLSIDE_CIRCUIT] {
        let track = Track::parse(source).unwrap();
        let geometry = build_track_geometry(&track);
        let canvas = bake_world(&geometry, track.theme);
        let slack = CAMERA_VIEW_SIZE * 0.5 + Vec2::splat(MAX_CAMERA_LEAD + 0.5 / TEXELS_PER_UNIT);
        let low = canvas.world_origin();
        let high = low + Vec2::new(canvas.width as f32, canvas.height as f32) / TEXELS_PER_UNIT;
        for quad in &geometry.guardrails {
            for point in quad.verts {
                assert!((point - slack).cmpge(low).all());
                assert!((point + slack).cmple(high).all());
            }
        }
    }
}

#[test]
fn optional_information_requires_usable_play_area_not_letterbox_bars() {
    assert!(optional_awareness_visible(Vec2::new(960.0, 540.0)));
    assert!(!optional_awareness_visible(Vec2::new(640.0, 360.0)));
    assert!(!optional_awareness_visible(Vec2::new(320.0, 180.0)));
    assert!(!optional_awareness_visible(Vec2::new(799.0, 1000.0)));
}
