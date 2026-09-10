//! Pure track render geometry: mitered road edges, edge lines, centerline
//! dashes, corner kerbs, guardrails, and the checkered start line.
//!
//! No Bevy types — every function returns plain vertex quads so the math is
//! testable headless. The Bevy spawn system is a thin adapter that consumes
//! this geometry and adds meshes/materials.

use glam::Vec2;
use topdown_racer_core::track::{Surface, Track, ZoneKind};

/// One planar quad: top-left, top-right, bottom-right, bottom-left corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
    pub verts: [Vec2; 4],
}
impl Quad {
    fn new(tl: Vec2, tr: Vec2, br: Vec2, bl: Vec2) -> Self {
        Self {
            verts: [tl, tr, br, bl],
        }
    }
}

/// A road quad tagged with the segment's authored surface (material choice).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadQuad {
    pub quad: Quad,
    pub surface: Surface,
}

/// A terrain-zone fill quad tagged with the zone kind (material choice).
/// Fan-triangulated from the authored polygon; a triangle zone emits one
/// degenerate quad with two coincident corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneQuad {
    pub quad: Quad,
    pub kind: ZoneKind,
}

/// All render geometry for one Track, in deterministic spawn order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrackRenderGeometry {
    /// Mitered road surface quads (one per non-Grass segment).
    pub road: Vec<RoadQuad>,
    /// White edge lines, two per rendered segment (left, right).
    pub edge_lines: Vec<Quad>,
    /// Dashed centerline segments.
    pub dashes: Vec<Quad>,
    /// Corner kerbs, alternating red/white by index.
    pub kerbs: Vec<Quad>,
    /// Outer boundary guardrail segments.
    pub guardrails: Vec<Quad>,
    /// Checkered start/finish squares, alternating white/black by (row, col).
    pub start_line: Vec<Quad>,
    /// Terrain-zone fill quads in author order, fan-triangulated per zone.
    pub zones: Vec<ZoneQuad>,
}

/// Computes all render geometry for a closed polyline circuit.
pub fn build_track_geometry(track: &Track) -> TrackRenderGeometry {
    let points = &track.points;
    let n = points.len().saturating_sub(1);
    let mut geo = TrackRenderGeometry::default();

    // Per-vertex half road width: every quad follows the width ramp.
    let half_at: Vec<f32> = track.widths.iter().map(|w| w * 0.5).collect();

    // Pre-calculate segment directions and normals
    let mut seg_dirs = Vec::with_capacity(n);
    let mut seg_normals = Vec::with_capacity(n);
    for i in 0..n {
        let dir = (points[i + 1] - points[i]).normalize_or_zero();
        let normal = Vec2::new(-dir.y, dir.x);
        seg_dirs.push(dir);
        seg_normals.push(normal);
    }

    // Pre-calculate corner miter vectors for each vertex i (0..=n)
    // Vertex i is at the junction between incoming segment (i + n - 1) % n and
    // outgoing segment i % n.
    let mut miter_normals = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let prev_idx = if i == 0 { n - 1 } else { (i - 1) % n };
        let curr_idx = i % n;
        let n_prev = seg_normals[prev_idx];
        let n_curr = seg_normals[curr_idx];
        let dot = n_prev.dot(n_curr);
        let m = if dot > -0.999 {
            (n_prev + n_curr) / (1.0 + dot)
        } else {
            n_curr
        };
        miter_normals.push(m);
    }

    // Cumulative arc length at each vertex, for local-width lookups along
    // segments (kerbs hug the ramping edge, the start line spans it).
    let mut vertex_arc = vec![0.0f32; n + 1];
    for i in 0..n {
        vertex_arc[i + 1] = vertex_arc[i] + (points[i + 1] - points[i]).length();
    }

    // Mitered road perimeter points (left and right)
    let edge_line_w = 0.35;
    let mut left_outer = Vec::with_capacity(n + 1);
    let mut right_outer = Vec::with_capacity(n + 1);
    let mut left_inner = Vec::with_capacity(n + 1);
    let mut right_inner = Vec::with_capacity(n + 1);

    for i in 0..=n {
        let p = points[i];
        let m = miter_normals[i];
        left_outer.push(p + m * half_at[i]);
        right_outer.push(p - m * half_at[i]);
        left_inner.push(p + m * (half_at[i] - edge_line_w));
        right_inner.push(p - m * (half_at[i] - edge_line_w));
    }

    for i in 0..n {
        let p0 = points[i];
        let p1 = points[i + 1];
        let dir = seg_dirs[i];
        let normal = seg_normals[i];
        let seg_len = (p1 - p0).length();

        let surface = track.surfaces.get(i).copied().unwrap_or(Surface::Road);
        if surface == Surface::Grass {
            continue;
        }

        // 1. Continuous mitered road surface quad
        geo.road.push(RoadQuad {
            quad: Quad::new(
                left_outer[i],
                right_outer[i],
                right_outer[i + 1],
                left_outer[i + 1],
            ),
            surface,
        });

        // 2. Continuous mitered white edge lines (flush joints, no overlaps)
        geo.edge_lines.push(Quad::new(
            left_outer[i],
            left_inner[i],
            left_inner[i + 1],
            left_outer[i + 1],
        ));
        geo.edge_lines.push(Quad::new(
            right_inner[i],
            right_outer[i],
            right_outer[i + 1],
            right_inner[i + 1],
        ));

        // 3. Dashed centerline (stops before corner junctions)
        let dash_step = 5.0;
        let dash_len = 2.4;
        let start_d = (half_at[i] + 1.0).min(seg_len * 0.5);
        let end_d = (seg_len - half_at[i + 1] - 1.0).max(start_d);
        let mut d = start_d;
        while d + dash_len <= end_d {
            let d0 = p0 + dir * d;
            let d1 = p0 + dir * (d + dash_len);
            geo.dashes.push(Quad::new(
                d0 + normal * 0.18,
                d0 - normal * 0.18,
                d1 - normal * 0.18,
                d1 + normal * 0.18,
            ));
            d += dash_step;
        }

        // 4. Red-and-white rumble strip kerbs along corner apexes
        let prev_idx = if i == 0 { n - 1 } else { i - 1 };
        let norm_prev = seg_normals[prev_idx];
        let dir_prev = seg_dirs[prev_idx];
        if (norm_prev - normal).length() > 0.001 {
            let kerb_w = 0.9;
            let kerb_len = 1.6;
            // Place kerbs on the outer side (-normal side for CCW loop) so they
            // sit in the runoff instead of overlapping the crossing leg's asphalt.
            // Incoming stretch towards corner
            for step in 0..4 {
                let d_end = (step as f32) * kerb_len;
                let d_start = (step as f32 + 1.0) * kerb_len;
                let pt0 = p0 - dir_prev * d_start;
                let pt1 = p0 - dir_prev * d_end;
                let inner0 = track.road_half_width_at_arc(vertex_arc[i] - d_start);
                let inner1 = track.road_half_width_at_arc(vertex_arc[i] - d_end);
                geo.kerbs.push(Quad::new(
                    pt0 - norm_prev * (inner0 + kerb_w),
                    pt0 - norm_prev * inner0,
                    pt1 - norm_prev * inner1,
                    pt1 - norm_prev * (inner1 + kerb_w),
                ));
            }
            // Outgoing stretch from corner
            for step in 0..4 {
                let d_start = (step as f32) * kerb_len;
                let d_end = (step as f32 + 1.0) * kerb_len;
                let pt0 = p0 + dir * d_start;
                let pt1 = p0 + dir * d_end;
                let inner0 = track.road_half_width_at_arc(vertex_arc[i] + d_start);
                let inner1 = track.road_half_width_at_arc(vertex_arc[i] + d_end);
                geo.kerbs.push(Quad::new(
                    pt0 - normal * (inner0 + kerb_w),
                    pt0 - normal * inner0,
                    pt1 - normal * inner1,
                    pt1 - normal * (inner1 + kerb_w),
                ));
            }
        }

        // 5. Outer boundary safety barrier (steel guardrail, outer perimeter only)
        // Outer side is -normal (-miter_normals), preventing walls from crossing infield
        let wall_w = 0.8;
        let w0 = points[i] - miter_normals[i] * track.widths[i];
        let w1 = points[i + 1] - miter_normals[i + 1] * track.widths[i + 1];
        let w_dir = (w1 - w0).normalize_or_zero();
        let w_norm = Vec2::new(-w_dir.y, w_dir.x);
        geo.guardrails.push(Quad::new(
            w0 + w_norm * (wall_w * 0.5),
            w0 - w_norm * (wall_w * 0.5),
            w1 - w_norm * (wall_w * 0.5),
            w1 + w_norm * (wall_w * 0.5),
        ));
    }

    // Checkered start/finish line across the start segment's straight, just
    // past the corner exit: clear of the crossing leg's asphalt, and the
    // whole grid stages behind it.
    if n > 0 {
        let gate = track.finish_gate();
        let dir = gate.direction;
        let normal = Vec2::new(-dir.y, dir.x);
        let line_center = gate.center;
        let half_line = gate.half_width;
        let checkers_count = 10;
        let checker_w = (half_line * 2.0) / checkers_count as f32;
        for row in 0..2 {
            let row_offset = (row as f32 - 0.5) * 0.8;
            for col in 0..checkers_count {
                let col_offset = -half_line + (col as f32 + 0.5) * checker_w;
                let center = line_center + dir * row_offset + normal * col_offset;
                geo.start_line.push(Quad::new(
                    center + dir * 0.4 + normal * (checker_w * 0.5),
                    center + dir * 0.4 - normal * (checker_w * 0.5),
                    center - dir * 0.4 - normal * (checker_w * 0.5),
                    center - dir * 0.4 + normal * (checker_w * 0.5),
                ));
            }
        }
    }
    // Terrain zones fan-triangulate in author order: each (v0, vi, vi+1)
    // triangle emits one quad with two coincident corners so the existing
    // quad spawn path works unchanged.
    for zone in &track.zones {
        let poly = &zone.polygon;
        if poly.len() < 3 {
            continue;
        }
        for i in 1..poly.len() - 1 {
            geo.zones.push(ZoneQuad {
                quad: Quad::new(poly[0], poly[i], poly[i + 1], poly[i + 1]),
                kind: zone.kind,
            });
        }
    }

    geo
}

#[cfg(test)]
mod tests {
    use super::*;
    use topdown_racer_core::track::{ZoneKind, SAMPLE_CIRCUIT};

    #[test]
    fn sample_circuit_geometry_counts_match_segments_and_grid() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let n = track.points.len() - 1; // 7 closed-loop segments
        let geo = build_track_geometry(&track);

        // No Grass segments in the sample circuit: every segment renders.
        assert_eq!(geo.road.len(), n);
        assert_eq!(geo.edge_lines.len(), 2 * n);
        assert_eq!(geo.guardrails.len(), n);
        // 2 rows x 10 columns of checkered squares.
        assert_eq!(geo.start_line.len(), 20);
        // The circuit has corners, so kerbs exist in complete 8-quad corners
        // (4 incoming + 4 outgoing stretches).
        assert!(!geo.kerbs.is_empty());
        assert_eq!(geo.kerbs.len() % 8, 0);
        assert!(!geo.dashes.is_empty());
    }
    #[test]
    fn every_quad_has_finite_vertices() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let geo = build_track_geometry(&track);
        let all: Vec<Quad> = geo
            .road
            .iter()
            .map(|r| r.quad)
            .chain(geo.edge_lines.iter().copied())
            .chain(geo.dashes.iter().copied())
            .chain(geo.kerbs.iter().copied())
            .chain(geo.guardrails.iter().copied())
            .chain(geo.start_line.iter().copied())
            .chain(geo.zones.iter().map(|z| z.quad))
            .collect();
        assert!(!all.is_empty());
        for quad in all {
            for v in quad.verts {
                assert!(v.is_finite(), "quad vertex must be finite: {v:?}");
            }
        }
    }

    #[test]
    fn geometry_is_deterministic() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        assert_eq!(build_track_geometry(&track), build_track_geometry(&track));
    }

    #[test]
    fn grass_segments_render_nothing() {
        // Same circuit but with the first two segments authored as Grass
        // (span end is inclusive): they contribute no road, edge, dash, kerb,
        // or guardrail quads.
        let text = r#"{
            "name": "Grass Gap",
            "width": 14.0,
            "points": [[0.0, 0.0], [120.0, 0.0], [160.0, 30.0], [160.0, 90.0],
                       [110.0, 120.0], [40.0, 120.0], [0.0, 90.0], [0.0, 0.0]],
            "surfaces": [{"start": 0, "end": 1, "surface": "grass"}]
        }"#;
        let track = Track::parse(text).unwrap();
        let geo = build_track_geometry(&track);
        assert_eq!(geo.road.len(), 5);
        assert_eq!(geo.guardrails.len(), 5);
    }
    /// A v2 track: quad sand zone, triangle dirt zone, two props, and the
    /// start line overridden to segment 1. The props parse but emit no
    /// geometry: scenery sprites read track.props directly.
    fn v2_track() -> Track {
        let text = r#"{
            "name": "V2 Geo",
            "width": 10.0,
            "points": [[0,0],[40,0],[40,40],[0,0]],
            "surfaces": [],
            "props": [
                {"type": "tree", "position": [50.0, 50.0]},
                {"type": "tire_stack", "position": [60.0, 60.0], "rotation_degrees": 90.0}
            ],
            "terrain_zones": [
                {"kind": "sand", "polygon": [[100,100],[110,100],[110,110],[100,110]]},
                {"kind": "dirt", "polygon": [[200,200],[210,200],[205,210]]}
            ],
            "theme": "hillside",
            "start_line": {"segment": 1}
        }"#;
        Track::parse(text).unwrap()
    }

    fn quad_center(quad: Quad) -> Vec2 {
        (quad.verts[0] + quad.verts[1] + quad.verts[2] + quad.verts[3]) * 0.25
    }

    #[test]
    fn zones_fan_triangulate_in_author_order() {
        let geo = build_track_geometry(&v2_track());
        // Quad zone fans into 2 quads, triangle zone into 1 degenerate quad
        // (two coincident corners) so the quad spawn path works unchanged.
        assert_eq!(geo.zones.len(), 3);
        assert_eq!(geo.zones[0].kind, ZoneKind::Sand);
        assert_eq!(geo.zones[1].kind, ZoneKind::Sand);
        assert_eq!(geo.zones[2].kind, ZoneKind::Dirt);
        // Fan order: first quad spans v0/v1/v2 of the sand polygon.
        assert_eq!(geo.zones[0].quad.verts[0], Vec2::new(100.0, 100.0));
        assert_eq!(geo.zones[0].quad.verts[1], Vec2::new(110.0, 100.0));
        assert_eq!(geo.zones[0].quad.verts[2], Vec2::new(110.0, 110.0));
        // Triangle zone emits one quad with two coincident corners.
        assert_eq!(geo.zones[2].quad.verts[2], geo.zones[2].quad.verts[3]);
        // Tracks without zones render none; geometry stays deterministic.
        let plain = Track::parse(SAMPLE_CIRCUIT).unwrap();
        assert!(build_track_geometry(&plain).zones.is_empty());
        assert_eq!(build_track_geometry(&v2_track()), geo);
    }

    #[test]
    fn start_line_moves_to_the_overridden_segment() {
        let track = v2_track();
        let geo = build_track_geometry(&track);
        assert_eq!(geo.start_line.len(), 20);
        // Segment 1 runs (40,0) -> (40,40): line center sits 8 units past
        // the vertex along +y (road half-width 5 + 3).
        let centroid = geo
            .start_line
            .iter()
            .fold(Vec2::ZERO, |acc, q| acc + quad_center(*q))
            / geo.start_line.len() as f32;
        assert!(
            (centroid - Vec2::new(40.0, 8.0)).length() < 1e-3,
            "start line must center on segment 1, got {centroid:?}"
        );

        // Default tracks keep the line on the opening straight.
        let plain = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let plain_geo = build_track_geometry(&plain);
        let plain_centroid = plain_geo
            .start_line
            .iter()
            .fold(Vec2::ZERO, |acc, q| acc + quad_center(*q))
            / plain_geo.start_line.len() as f32;
        assert!(
            (plain_centroid - Vec2::new(10.0, 0.0)).length() < 1e-3,
            "default start line must stay on segment 0, got {plain_centroid:?}"
        );
    }

    #[test]
    fn road_and_edge_perimeter_follows_the_width_ramp() {
        // Triangle with a narrow opening (10) widening to 30: seg 0 runs
        // (0,0) -> (120,0) with widths 10 -> 30. The mitered corners must
        // sit at each vertex's local half width.
        let text = r#"{
            "name": "Ramp",
            "width": 10.0,
            "widths": [10.0, 30.0, 30.0, 10.0],
            "points": [[0,0],[120,0],[120,60],[0,0]],
            "surfaces": []
        }"#;
        let track = Track::parse(text).unwrap();
        let geo = build_track_geometry(&track);

        // The road quad on the ramping segment: its far corners sit at the
        // wide vertex's half width 15 (miter at v1 is (-1, 1)), not the
        // global-minimum half width 5.
        let road = geo.road[0].quad;
        let br = road.verts[2];
        assert!(
            (br - Vec2::new(135.0, -15.0)).length() < 1e-3,
            "road corner at the wide vertex must follow the ramp, got {br:?}"
        );

        // The right edge line of the same segment tucks in by the local
        // edge-line width at the wide end: 15 - 0.35.
        let edge = geo.edge_lines[1].verts[3];
        assert!(
            (edge - Vec2::new(134.65, -14.65)).length() < 1e-3,
            "edge line at the wide vertex must follow the ramp, got {edge:?}"
        );
    }

    /// Parses the ramp triangle used by the ramp pins above.
    fn ramp_track() -> Track {
        let text = r#"{
            "name": "Ramp",
            "width": 10.0,
            "widths": [10.0, 30.0, 30.0, 10.0],
            "points": [[0,0],[120,0],[120,60],[0,0]],
            "surfaces": []
        }"#;
        Track::parse(text).unwrap()
    }

    #[test]
    fn guardrails_follow_the_local_wall_distance() {
        let geo = build_track_geometry(&ramp_track());
        // Guardrail over the ramping segment: its far end hangs off vertex
        // 1 at the LOCAL wall distance 30 (miter (-1, 1) there), not the
        // global-minimum 10. The end midpoint of the quad strip is exactly
        // the offset vertex.
        let end = (geo.guardrails[0].verts[2] + geo.guardrails[0].verts[3]) * 0.5;
        assert!(
            (end - Vec2::new(150.0, -30.0)).length() < 1e-3,
            "guardrail end must follow the local wall distance, got {end:?}"
        );
    }

    #[test]
    fn kerbs_hug_the_local_road_edge() {
        let geo = build_track_geometry(&ramp_track());
        // Corners at every vertex emit 8 kerb quads each; the corner at
        // vertex 1 is the second group (indices 8..16). Its outermost
        // incoming kerb (verts[1] is its far edge, 6.4 units back along
        // seg 0 at (113.6, 0)) must hug the local half width 14.47 there,
        // not the global-minimum 5.
        let kerb = geo.kerbs[8 + 3];
        let inner = kerb.verts[1];
        assert!(
            (inner - Vec2::new(113.6, -14.4667)).length() < 1e-2,
            "kerb must hug the local road edge, got {inner:?}"
        );
    }

    #[test]
    fn start_line_spans_the_local_width() {
        let geo = build_track_geometry(&ramp_track());
        // The line sits 8 units past vertex 0 (half 5 + 3), where the ramp
        // has widened to width ~11.33 (half ~5.67): the first column
        // centers at -half + checker_w/2 = -5.1, not the global-minimum
        // -4.5.
        let center = quad_center(geo.start_line[0]);
        assert!(
            (center - Vec2::new(7.6, -5.1)).length() < 1e-2,
            "start line must span the local width, got {center:?}"
        );
    }
}
