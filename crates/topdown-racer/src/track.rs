use glam::Vec2;
use serde::Deserialize;

/// The bundled sample circuit, shipped as a real data file so every later
/// ticket has something concrete to load.
pub const SAMPLE_CIRCUIT: &str = include_str!("../data/tracks/sample-circuit.json");

/// A parsed closed circuit: center polyline, width, and one surface per
/// polyline segment.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub name: String,
    /// Full road width in world units.
    pub width: f32,
    /// Center polyline vertices; the last equals the first (closed loop).
    pub points: Vec<Vec2>,
    /// Surface of segment `i`, which connects `points[i]` to `points[i + 1]`.
    /// One entry per segment (`points.len() - 1`).
    pub surfaces: Vec<Surface>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Road,
    Grass,
    Gravel,
}

impl Surface {
    pub const KNOWN: [Surface; 3] = [Surface::Road, Surface::Grass, Surface::Gravel];

    fn from_name(name: &str) -> Option<Surface> {
        Surface::KNOWN
            .iter()
            .copied()
            .find(|surface| surface.name() == name)
    }

    fn name(self) -> &'static str {
        match self {
            Surface::Road => "road",
            Surface::Grass => "grass",
            Surface::Gravel => "gravel",
        }
    }
}

/// A parsed Track or a descriptive reason the file was rejected.
#[derive(Debug)]
pub enum TrackParseError {
    /// The text is not a well-formed track document.
    Malformed(serde_json::Error),
    /// The polyline does not loop back to its first vertex.
    UnclosedLoop { first: [f32; 2], last: [f32; 2] },
    /// Fewer than 3 distinct vertices; not a closed circuit.
    TooFewPoints(usize),
    /// Width is zero or negative; a road needs positive width.
    InvalidWidth(f32),
    /// A surface span names a surface that does not exist.
    UnknownSurface {
        found: String,
        known: Vec<&'static str>,
    },
    /// A surface span covers segments beyond the polyline.
    InvalidSurfaceSpan {
        start: usize,
        end: usize,
        segment_count: usize,
    },
}

impl std::fmt::Display for TrackParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackParseError::Malformed(e) => write!(f, "malformed track text: {e}"),
            TrackParseError::UnclosedLoop { first, last } => write!(
                f,
                "track is not closed: last point ({}, {}) does not match first point ({}, {})",
                last[0], last[1], first[0], first[1]
            ),
            TrackParseError::TooFewPoints(count) => write!(
                f,
                "closed track needs at least 3 distinct points, got {count}"
            ),
            TrackParseError::InvalidWidth(width) => {
                write!(f, "track width must be positive, got {width}")
            }
            TrackParseError::UnknownSurface { found, known } => write!(
                f,
                "unknown surface '{found}'; known surfaces: {}",
                known.join(", ")
            ),
            TrackParseError::InvalidSurfaceSpan {
                start,
                end,
                segment_count,
            } => write!(
                f,
                "surface span covers segments {start}..={end} but the track has only \
                 {segment_count} segments"
            ),
        }
    }
}

impl std::error::Error for TrackParseError {}

#[derive(Deserialize)]
struct TrackSource {
    name: String,
    width: f32,
    points: Vec<[f32; 2]>,
    surfaces: Vec<SurfaceSpanSource>,
}

/// One authored surface run: polyline segments `start..=end` (inclusive;
/// `end` defaults to `start`) get `surface`.
#[derive(Deserialize)]
struct SurfaceSpanSource {
    start: usize,
    end: Option<usize>,
    surface: String,
}

/// Maximum distance between the first and last vertex for the loop to count
/// as closed.
const CLOSURE_TOLERANCE: f32 = 1e-4;

/// A closed circuit needs 3 distinct vertices plus the closing repeat.
const MIN_DISTINCT_POINTS: usize = 3;

impl Track {
    pub fn parse(text: &str) -> Result<Track, TrackParseError> {
        let src: TrackSource = serde_json::from_str(text).map_err(TrackParseError::Malformed)?;

        if !src.width.is_finite() || src.width <= 0.0 {
            return Err(TrackParseError::InvalidWidth(src.width));
        }

        if src.points.is_empty() {
            return Err(TrackParseError::TooFewPoints(0));
        }

        let first = src.points.first().copied().unwrap_or([0.0; 2]);
        let last = src.points.last().copied().unwrap_or([0.0; 2]);
        let gap = Vec2::new(first[0] - last[0], first[1] - last[1]).length();
        if gap > CLOSURE_TOLERANCE {
            return Err(TrackParseError::UnclosedLoop { first, last });
        }
        let open = &src.points[..src.points.len() - 1];
        let distinct = open
            .iter()
            .enumerate()
            .filter(|(i, p)| {
                open[..*i].iter().all(|q| {
                    (p[0] - q[0]).abs() > CLOSURE_TOLERANCE
                        || (p[1] - q[1]).abs() > CLOSURE_TOLERANCE
                })
            })
            .count();
        if distinct < MIN_DISTINCT_POINTS {
            return Err(TrackParseError::TooFewPoints(distinct));
        }

        let segment_count = src.points.len() - 1;
        let mut surfaces = vec![Surface::Road; segment_count];
        for span in &src.surfaces {
            let end = span.end.unwrap_or(span.start);
            if span.start >= segment_count || end >= segment_count || end < span.start {
                return Err(TrackParseError::InvalidSurfaceSpan {
                    start: span.start,
                    end,
                    segment_count,
                });
            }
            let Some(surface) = Surface::from_name(&span.surface) else {
                let mut known: Vec<&'static str> =
                    Surface::KNOWN.iter().map(|s| s.name()).collect();
                known.sort_unstable();
                return Err(TrackParseError::UnknownSurface {
                    found: span.surface.clone(),
                    known,
                });
            };
            for surface_slot in &mut surfaces[span.start..=end] {
                *surface_slot = surface;
            }
        }

        let mut points: Vec<Vec2> = src.points.iter().map(|p| Vec2::new(p[0], p[1])).collect();
        // The tolerant closure check passed; canonicalize the closing vertex
        // so the documented last-equals-first invariant holds exactly.
        let closing = points[0];
        *points.last_mut().unwrap() = closing;

        Ok(Track {
            name: src.name,
            width: src.width,
            points,
            surfaces,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_circuit_parses_into_track_with_geometry_and_surfaces() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();

        assert_eq!(track.name, "Sample Circuit");
        assert_eq!(track.width, 14.0);

        // Geometry: closed loop with the first vertex repeated as the last.
        let n = track.points.len();
        assert_eq!(track.points[0], Vec2::new(0.0, 0.0));
        assert_eq!(track.points[1], Vec2::new(120.0, 0.0));
        assert_eq!(track.points[n - 1], track.points[0]);

        // Surfaces: one resolved surface per polyline segment, road by
        // default, the authored span overriding the right straight.
        assert_eq!(track.surfaces.len(), n - 1);
        assert_eq!(track.surfaces[0], Surface::Road);
        assert_eq!(track.surfaces[2], Surface::Gravel);
        assert_eq!(track.surfaces[3], Surface::Gravel);
        assert_eq!(track.surfaces[5], Surface::Road);
    }

    /// Raw track text with substituted points, width, and surface spans.
    fn track_text(points: &str, width: &str, surfaces: &str) -> String {
        format!(r#"{{"name": "T", "width": {width}, "points": {points}, "surfaces": {surfaces}}}"#)
    }

    /// A minimal closed triangle circuit.
    fn closed_triangle_text() -> String {
        track_text("[[0,0],[10,0],[10,10],[0,0]]", "10", "[]")
    }

    fn unclosed_text() -> String {
        track_text("[[0,0],[10,0],[10,10],[5,20]]", "10", "[]")
    }

    #[test]
    fn unclosed_loop_is_rejected_with_descriptive_error() {
        let err = Track::parse(&unclosed_text()).unwrap_err();
        assert!(
            matches!(err, TrackParseError::UnclosedLoop { .. }),
            "{err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("not closed"), "message: {msg}");
        assert!(msg.contains("(5, 20)"), "message: {msg}");

        // Boundary: the same shape closed explicitly parses.
        let closed = Track::parse(&closed_triangle_text()).unwrap();
        assert_eq!(closed.points.len(), 4);
    }

    #[test]
    fn degenerate_loop_is_rejected_with_descriptive_error() {
        for points in ["[[0,0],[0,0]]", "[[0,0],[10,0],[0,0]]", "[[0,0]]"] {
            let text = track_text(points, "10", "[]");
            let err = Track::parse(&text).unwrap_err();
            assert!(
                matches!(err, TrackParseError::TooFewPoints(_)),
                "{points}: {err:?}"
            );
            let msg = err.to_string();
            assert!(msg.contains("distinct points"), "message: {msg}");
        }
    }

    #[test]
    fn near_closed_loop_canonicalizes_the_closing_point() {
        // Within CLOSURE_TOLERANCE, so it parses — and the parsed Track then
        // satisfies last point == first point exactly.
        let text = track_text("[[0,0],[10,0],[10,10],[0.00005,0]]", "10", "[]");
        let track = Track::parse(&text).unwrap();
        assert_eq!(track.points.last(), Some(&track.points[0]));
    }

    #[test]
    fn non_positive_width_is_rejected_with_descriptive_error() {
        for width in ["0", "-5"] {
            let text = track_text("[[0,0],[10,0],[10,10],[0,0]]", width, "[]");
            let err = Track::parse(&text).unwrap_err();
            assert!(
                matches!(err, TrackParseError::InvalidWidth { .. }),
                "width {width}: {err:?}"
            );
            let msg = err.to_string();
            assert!(msg.contains("width"), "message: {msg}");
        }

        // Boundary: a small positive width parses.
        let text = track_text("[[0,0],[10,0],[10,10],[0,0]]", "0.5", "[]");
        assert!(Track::parse(&text).is_ok());
    }

    #[test]
    fn unknown_surface_is_rejected_with_descriptive_error() {
        let text = track_text(
            "[[0,0],[10,0],[10,10],[0,0]]",
            "10",
            r#"[{"start": 0, "surface": "ice"}]"#,
        );
        let err = Track::parse(&text).unwrap_err();
        assert!(
            matches!(err, TrackParseError::UnknownSurface { .. }),
            "{err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("unknown surface"), "message: {msg}");
        assert!(msg.contains("'ice'"), "message: {msg}");
        assert!(msg.contains("road"), "message: {msg}");

        // Distinctness: every known surface name parses and resolves.
        let text = track_text(
            "[[0,0],[10,0],[10,10],[0,0]]",
            "10",
            r#"[{"start": 0, "end": 1, "surface": "grass"}]"#,
        );
        let track = Track::parse(&text).unwrap();
        assert_eq!(track.surfaces[1], Surface::Grass);
    }

    #[test]
    fn surface_span_out_of_range_is_rejected_with_descriptive_error() {
        for surfaces in [
            r#"[{"start": 9, "surface": "gravel"}]"#,
            r#"[{"start": 0, "end": 9, "surface": "gravel"}]"#,
            r#"[{"start": 2, "end": 1, "surface": "gravel"}]"#,
        ] {
            let text = track_text("[[0,0],[10,0],[10,10],[0,0]]", "10", surfaces);
            let err = Track::parse(&text).unwrap_err();
            assert!(
                matches!(err, TrackParseError::InvalidSurfaceSpan { .. }),
                "{surfaces}: {err:?}"
            );
            let msg = err.to_string();
            assert!(msg.contains("segment"), "message: {msg}");
        }
    }

    #[test]
    fn malformed_text_is_rejected_with_descriptive_error() {
        for text in [
            "{not json",
            r#"{"name": "T", "width": "wide", "points": [], "surfaces": []}"#,
            r#"{"name": "T", "width": 10}"#,
            r#"{"name": "T", "width": 10, "points": [[0,0],[10,0],[10,10],[0,0]]}"#,
        ] {
            let err = Track::parse(text).unwrap_err();
            assert!(
                matches!(err, TrackParseError::Malformed(_)),
                "{text}: {err:?}"
            );
            let msg = err.to_string();
            assert!(msg.contains("malformed track text"), "message: {msg}");
            assert!(msg.contains("line 1"), "message: {msg}");
        }
    }
}
