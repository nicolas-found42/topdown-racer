use glam::Vec2;
use serde::Deserialize;

/// The bundled sample circuit, shipped as a real data file so every later
/// ticket has something concrete to load.
pub const SAMPLE_CIRCUIT: &str = include_str!("../data/tracks/sample-circuit.json");
pub const HILLSIDE_CIRCUIT: &str = include_str!("../data/tracks/hillside-circuit.json");

/// Shared timing and presentation geometry for a directional road gate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalGate {
    pub center: Vec2,
    pub direction: Vec2,
    pub half_width: f32,
}

impl DirectionalGate {
    /// Swept forward crossing, bounded at the intersection with the gate plane.
    pub fn crossed(self, previous: Vec2, current: Vec2) -> bool {
        let before = (previous - self.center).dot(self.direction);
        let after = (current - self.center).dot(self.direction);
        if before > 0.0 || after <= 0.0 || after <= before {
            return false;
        }
        let intersection = previous.lerp(current, -before / (after - before));
        let normal = Vec2::new(-self.direction.y, self.direction.x);
        (intersection - self.center).dot(normal).abs() <= self.half_width
    }
}

/// A parsed closed circuit: center polyline, per-vertex width ramp, and
/// one surface per polyline segment.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub name: String,
    /// Per-vertex full road width in world units, one entry per entry of
    /// `points` (`widths.len() == points.len()`); the closing entry
    /// repeats the opening one, mirroring the points loop. Tracks authored
    /// with a single global `width` resolve to a uniform array.
    pub widths: Vec<f32>,
    /// Center polyline vertices; the last equals the first (closed loop).
    pub points: Vec<Vec2>,
    /// Surface of segment `i`, which connects `points[i]` to `points[i + 1]`.
    /// One entry per segment (`points.len() - 1`).
    pub surfaces: Vec<Surface>,
    /// Authored decor props (scenery only; never affect handling).
    pub props: Vec<TrackProp>,
    /// Authored ground patches drawn beneath the Track (scenery only).
    pub zones: Vec<TerrainZone>,
    /// Visual theme of the circuit.
    pub theme: Theme,
    /// Index of the polyline segment holding the start/finish line and the
    /// lap-progress origin (0 when the file omits the override).
    pub start_segment: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Road,
    Grass,
    Gravel,
}
/// Kind of an authored decor prop placed near the Track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    Tree,
    TireStack,
    BrakeBoard,
}

/// Kind of an authored Terrain Zone ground patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneKind {
    Sand,
    Dirt,
    DarkGrass,
}

/// Visual theme of the circuit. Only one theme exists for v1, kept as a
/// typed enum so later themes extend the format without retyping the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Hillside,
}

/// One authored decor prop: scenery at a world position, never affecting
/// handling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackProp {
    pub kind: PropKind,
    pub position: Vec2,
    /// Facing in radians, converted from the authored `rotation_degrees`
    /// (0 when the file omits it).
    pub rotation: f32,
}

/// One authored Terrain Zone: a ground patch drawn beneath the Track,
/// never affecting handling. The polygon auto-closes; the first vertex is
/// not repeated at the end.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainZone {
    pub kind: ZoneKind,
    pub polygon: Vec<Vec2>,
}

/// Contact with a Track boundary wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallContact {
    /// Inward unit normal pointing away from the wall, back into the track corridor.
    pub normal: Vec2,
    /// Distance the Car penetrated beyond the boundary wall.
    pub penetration: f32,
}

/// Result of querying the nearest polyline segment to a world position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearestSegment {
    /// Index of the polyline segment (`0..points.len() - 1`).
    pub segment_index: usize,
    /// Closest point on the segment.
    pub closest_point: Vec2,
    /// Distance from the query pose to `closest_point`.
    pub distance: f32,
    /// Fraction along the segment (0 = start vertex, 1 = end vertex) at
    /// which `closest_point` sits; the interpolation parameter for
    /// per-vertex quantities such as width.
    pub t: f32,
}

/// The Car's position expressed in the Track's centerline frame: how far
/// around the loop it is, which way the centerline runs there, and how far
/// off-center it sits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CenterlineFrame {
    /// Arc position along the closed centerline (0 = first vertex, increasing
    /// in the polyline's forward direction, wrapping at the loop length).
    pub arc: f32,
    /// Unit vector of the centerline's forward direction at `arc`.
    pub direction: Vec2,
    /// Signed lateral offset from the centerline; positive is to the left of
    /// `direction`.
    pub lateral: f32,
}

/// Finds the known variant whose name equals `name`, or `None`.
fn known_named<T: Copy>(known: &[T], name: &str, name_of: fn(T) -> &'static str) -> Option<T> {
    known
        .iter()
        .copied()
        .find(|variant| name_of(*variant) == name)
}

/// Every known variant's name, sorted for error messages.
fn sorted_names<T: Copy>(known: &[T], name_of: fn(T) -> &'static str) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = known.iter().copied().map(name_of).collect();
    names.sort_unstable();
    names
}

impl Surface {
    pub const KNOWN: [Surface; 3] = [Surface::Road, Surface::Grass, Surface::Gravel];

    fn from_name(name: &str) -> Option<Surface> {
        known_named(&Surface::KNOWN, name, Self::name)
    }

    fn name(self) -> &'static str {
        match self {
            Surface::Road => "road",
            Surface::Grass => "grass",
            Surface::Gravel => "gravel",
        }
    }
}
impl PropKind {
    pub const KNOWN: [PropKind; 3] = [PropKind::Tree, PropKind::TireStack, PropKind::BrakeBoard];

    fn from_name(name: &str) -> Option<PropKind> {
        known_named(&PropKind::KNOWN, name, Self::name)
    }

    fn name(self) -> &'static str {
        match self {
            PropKind::Tree => "tree",
            PropKind::TireStack => "tire_stack",
            PropKind::BrakeBoard => "brake_board",
        }
    }
}

impl ZoneKind {
    pub const KNOWN: [ZoneKind; 3] = [ZoneKind::Sand, ZoneKind::Dirt, ZoneKind::DarkGrass];

    fn from_name(name: &str) -> Option<ZoneKind> {
        known_named(&ZoneKind::KNOWN, name, Self::name)
    }

    fn name(self) -> &'static str {
        match self {
            ZoneKind::Sand => "sand",
            ZoneKind::Dirt => "dirt",
            ZoneKind::DarkGrass => "dark_grass",
        }
    }
}

impl Theme {
    pub const KNOWN: [Theme; 1] = [Theme::Hillside];

    fn from_name(name: &str) -> Option<Theme> {
        known_named(&Theme::KNOWN, name, Self::name)
    }

    fn name(self) -> &'static str {
        match self {
            Theme::Hillside => "hillside",
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
    /// Consecutive vertices coincide, leaving no direction for this segment.
    ZeroLengthSegment { segment: usize },
    /// Width is zero or negative; a road needs positive width.
    InvalidWidth(f32),
    /// A `widths` array whose entry count differs from the points count;
    /// one width per vertex (closing repeat included) is required.
    WidthsLengthMismatch { points: usize, widths: usize },
    /// A `widths` entry that is zero or negative.
    InvalidVertexWidth { index: usize, value: f32 },
    /// The track JSON names neither a global `width` nor a per-vertex
    /// `widths` array, so no road width can be resolved.
    MissingWidth,
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
    /// A decor prop names a prop type that does not exist.
    UnknownPropType {
        found: String,
        known: Vec<&'static str>,
    },
    /// A terrain zone names a zone kind that does not exist.
    UnknownZoneKind {
        found: String,
        known: Vec<&'static str>,
    },
    /// A terrain zone has fewer than 3 distinct vertices; not a patch.
    DegenerateZone { distinct: usize },
    /// The start-line override names a segment beyond the polyline.
    InvalidStartSegment {
        segment: usize,
        segment_count: usize,
    },
    /// The theme names a theme that does not exist.
    UnknownTheme {
        found: String,
        known: Vec<&'static str>,
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
            TrackParseError::ZeroLengthSegment { segment } => write!(
                f,
                "track segment {segment} has zero length; consecutive vertices must differ"
            ),
            TrackParseError::InvalidWidth(width) => {
                write!(f, "track width must be positive, got {width}")
            }
            TrackParseError::WidthsLengthMismatch { points, widths } => write!(
                f,
                "track widths has {widths} entries but the track has {points} points; \
                 one width per vertex (closing repeat included) is required"
            ),
            TrackParseError::InvalidVertexWidth { index, value } => {
                write!(f, "track widths[{index}] must be positive, got {value}")
            }
            TrackParseError::MissingWidth => write!(
                f,
                "track needs a positive width or a per-vertex widths array"
            ),
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
            TrackParseError::UnknownPropType { found, known } => write!(
                f,
                "unknown prop type '{found}'; known prop types: {}",
                known.join(", ")
            ),
            TrackParseError::UnknownZoneKind { found, known } => write!(
                f,
                "unknown terrain zone kind '{found}'; known zone kinds: {}",
                known.join(", ")
            ),
            TrackParseError::DegenerateZone { distinct } => write!(
                f,
                "terrain zone needs at least 3 distinct vertices, got {distinct}"
            ),
            TrackParseError::InvalidStartSegment {
                segment,
                segment_count,
            } => write!(
                f,
                "start segment {segment} is out of range: the track has only \
                 {segment_count} segments"
            ),
            TrackParseError::UnknownTheme { found, known } => write!(
                f,
                "unknown theme '{found}'; known themes: {}",
                known.join(", ")
            ),
        }
    }
}

impl std::error::Error for TrackParseError {}

#[derive(Deserialize)]
struct TrackSource {
    name: String,
    #[serde(default)]
    widths: Option<Vec<f32>>,
    points: Vec<[f32; 2]>,
    surfaces: Vec<SurfaceSpanSource>,
    #[serde(default)]
    props: Vec<PropSource>,
    #[serde(default)]
    terrain_zones: Vec<ZoneSource>,
    #[serde(default)]
    width: Option<f32>,
    #[serde(default)]
    theme: Option<String>,
    #[serde(default)]
    start_line: Option<StartLineSource>,
}

/// One authored surface run: polyline segments `start..=end` (inclusive;
/// `end` defaults to `start`) get `surface`.
#[derive(Deserialize)]
struct SurfaceSpanSource {
    start: usize,
    end: Option<usize>,
    surface: String,
}

/// One authored decor prop: `rotation_degrees` defaults to 0.
#[derive(Deserialize)]
struct PropSource {
    #[serde(rename = "type")]
    kind: String,
    position: [f32; 2],
    #[serde(default)]
    rotation_degrees: Option<f32>,
}

/// One authored terrain zone: the polygon auto-closes, so the first vertex
/// is not repeated at the end.
#[derive(Deserialize)]
struct ZoneSource {
    kind: String,
    polygon: Vec<[f32; 2]>,
}

/// Start/finish line override: which polyline segment holds the line.
#[derive(Deserialize)]
struct StartLineSource {
    segment: usize,
}

/// Maximum distance between the first and last vertex for the loop to count
/// as closed.
const CLOSURE_TOLERANCE: f32 = 1e-4;

/// A closed circuit needs 3 distinct vertices plus the closing repeat.
const MIN_DISTINCT_POINTS: usize = 3;

impl Track {
    pub fn parse(text: &str) -> Result<Track, TrackParseError> {
        let src: TrackSource = serde_json::from_str(text).map_err(TrackParseError::Malformed)?;

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
                let known = sorted_names(&Surface::KNOWN, Surface::name);
                return Err(TrackParseError::UnknownSurface {
                    found: span.surface.clone(),
                    known,
                });
            };
            for surface_slot in &mut surfaces[span.start..=end] {
                *surface_slot = surface;
            }
        }

        let theme = match &src.theme {
            None => Theme::Hillside,
            Some(name) => Theme::from_name(name).ok_or_else(|| {
                let known = sorted_names(&Theme::KNOWN, Theme::name);
                TrackParseError::UnknownTheme {
                    found: name.clone(),
                    known,
                }
            })?,
        };

        let mut props = Vec::with_capacity(src.props.len());
        for prop in &src.props {
            let Some(kind) = PropKind::from_name(&prop.kind) else {
                let known = sorted_names(&PropKind::KNOWN, PropKind::name);
                return Err(TrackParseError::UnknownPropType {
                    found: prop.kind.clone(),
                    known,
                });
            };
            props.push(TrackProp {
                kind,
                position: Vec2::new(prop.position[0], prop.position[1]),
                rotation: prop.rotation_degrees.unwrap_or(0.0).to_radians(),
            });
        }

        let mut zones = Vec::with_capacity(src.terrain_zones.len());
        for zone in &src.terrain_zones {
            let Some(kind) = ZoneKind::from_name(&zone.kind) else {
                let known = sorted_names(&ZoneKind::KNOWN, ZoneKind::name);
                return Err(TrackParseError::UnknownZoneKind {
                    found: zone.kind.clone(),
                    known,
                });
            };
            let distinct = zone
                .polygon
                .iter()
                .enumerate()
                .filter(|(i, p)| {
                    zone.polygon[..*i].iter().all(|q| {
                        (p[0] - q[0]).abs() > CLOSURE_TOLERANCE
                            || (p[1] - q[1]).abs() > CLOSURE_TOLERANCE
                    })
                })
                .count();
            if distinct < MIN_DISTINCT_POINTS {
                return Err(TrackParseError::DegenerateZone { distinct });
            }
            zones.push(TerrainZone {
                kind,
                polygon: zone.polygon.iter().map(|p| Vec2::new(p[0], p[1])).collect(),
            });
        }

        let start_segment = match &src.start_line {
            None => 0,
            Some(start_line) if start_line.segment < segment_count => start_line.segment,
            Some(start_line) => {
                return Err(TrackParseError::InvalidStartSegment {
                    segment: start_line.segment,
                    segment_count,
                });
            }
        };

        let mut points: Vec<Vec2> = src.points.iter().map(|p| Vec2::new(p[0], p[1])).collect();
        // The tolerant closure check passed; canonicalize the closing vertex
        // so the documented last-equals-first invariant holds exactly.
        let closing = points[0];
        *points.last_mut().unwrap() = closing;
        for (segment, pair) in points.windows(2).enumerate() {
            if pair[0] == pair[1] {
                return Err(TrackParseError::ZeroLengthSegment { segment });
            }
        }

        // Resolve per-vertex widths: an authored `widths` array wins and a
        // co-authored global `width` is ignored; otherwise the global width
        // broadcasts to a uniform ramp. A widths-only file is equally valid.
        let widths = match (src.widths, src.width) {
            (Some(mut raw), _) => {
                if raw.len() != src.points.len() {
                    return Err(TrackParseError::WidthsLengthMismatch {
                        points: src.points.len(),
                        widths: raw.len(),
                    });
                }
                for (index, value) in raw.iter().enumerate() {
                    if !value.is_finite() || *value <= 0.0 {
                        return Err(TrackParseError::InvalidVertexWidth {
                            index,
                            value: *value,
                        });
                    }
                }
                // The closing vertex IS the first vertex; canonicalize its
                // width like the points loop so the invariant holds exactly.
                let opening = raw[0];
                *raw.last_mut().unwrap() = opening;
                raw
            }
            (None, Some(width)) => {
                if !width.is_finite() || width <= 0.0 {
                    return Err(TrackParseError::InvalidWidth(width));
                }
                vec![width; points.len()]
            }
            (None, None) => return Err(TrackParseError::MissingWidth),
        };

        Ok(Track {
            name: src.name,
            widths,
            points,
            surfaces,
            props,
            zones,
            theme,
            start_segment,
        })
    }

    /// Total arc length of the closed centerline polyline.
    pub fn total_length(&self) -> f32 {
        (0..self.points.len() - 1)
            .map(|i| self.points[i].distance(self.points[i + 1]))
            .sum()
    }

    /// Locates the polyline segment containing `arc` and the arc distance
    /// remaining within it, walking the loop exactly as `point_at_arc`
    /// always has. Falls back to the opening segment only when accumulated
    /// float drift overshoots the loop length.
    fn locate_arc(&self, arc: f32) -> (usize, f32) {
        let segments = self.points.len() - 1;
        let mut remaining = arc.rem_euclid(self.total_length());
        for seg in 0..segments {
            let len = self.points[seg].distance(self.points[seg + 1]);
            if remaining <= len {
                return (seg, remaining);
            }
            remaining -= len;
        }
        (0, 0.0)
    }

    /// Point on the centerline at arc position `arc` (0 = first vertex,
    /// increasing along the polyline's forward direction; wraps at the loop).
    pub fn point_at_arc(&self, arc: f32) -> Vec2 {
        let (seg, remaining) = self.locate_arc(arc);
        let a = self.points[seg];
        let b = self.points[seg + 1];
        let len = a.distance(b);
        if len > 1e-6 {
            a + (b - a) / len * remaining
        } else {
            a
        }
    }

    /// Expresses `pose` in the centerline frame: arc position, forward
    /// direction, and signed lateral offset (positive = left of forward).
    pub fn centerline_frame(&self, pose: Vec2) -> CenterlineFrame {
        let nearest = self.nearest_segment(pose);
        let a = self.points[nearest.segment_index];
        let b = self.points[nearest.segment_index + 1];
        let ab = b - a;
        let len = ab.length();
        let direction = if len > 1e-6 { ab / len } else { Vec2::X };
        let along = (nearest.closest_point - a).dot(direction).clamp(0.0, len);
        let mut arc = along;
        for i in 0..nearest.segment_index {
            arc += self.points[i].distance(self.points[i + 1]);
        }
        let delta = pose - nearest.closest_point;
        CenterlineFrame {
            arc,
            direction,
            lateral: direction.x * delta.y - direction.y * delta.x,
        }
    }

    /// Arc position of the start/finish line: the length along the centerline
    /// from the first vertex to the overridden start segment's vertex (0
    /// when the file omits the override).
    pub fn start_arc(&self) -> f32 {
        (0..self.start_segment)
            .map(|i| self.points[i].distance(self.points[i + 1]))
            .sum()
    }

    /// The checker center sits beyond the start vertex's corner exit.
    pub fn finish_gate(&self) -> DirectionalGate {
        let s = self.start_segment;
        let segment = self.points[s + 1] - self.points[s];
        let distance = (self.widths[s] * 0.5 + 3.0).min(segment.length() * 0.5);
        let direction = segment.normalize();
        DirectionalGate {
            center: self.points[s] + direction * distance,
            direction,
            half_width: self.road_half_width_at_arc(self.start_arc() + distance),
        }
    }

    /// Returns a world point `distance` units back along the polyline from
    /// the start line (the overridden start segment's vertex, or the first
    /// vertex by default), following the closed loop backwards.
    pub fn spawn_pose(&self, distance: f32) -> Vec2 {
        if self.start_segment == 0 {
            self.point_at_arc(self.total_length() - distance)
        } else {
            self.point_at_arc(self.start_arc() - distance)
        }
    }

    /// Half road width at `pose`: the local width, linearly interpolated
    /// between the nearest segment's endpoint widths by the projection
    /// along that segment. Lateral offset does not matter.
    pub fn road_half_width_at(&self, pose: Vec2) -> f32 {
        self.road_width_at_nearest(self.nearest_segment(pose)) * 0.5
    }

    /// Distance from the centerline polyline to the boundary wall at
    /// `pose`, following the same interpolated local width as
    /// [`Track::road_half_width_at`].
    pub fn wall_distance_at(&self, pose: Vec2) -> f32 {
        self.road_width_at_nearest(self.nearest_segment(pose))
    }

    /// Full road width at a located nearest segment: linearly interpolated
    /// between the segment's endpoint widths by the stored projection
    /// fraction. Uniform-width tracks return their width exactly.
    fn road_width_at_nearest(&self, nearest: NearestSegment) -> f32 {
        let w0 = self.widths[nearest.segment_index];
        w0 + (self.widths[nearest.segment_index + 1] - w0) * nearest.t
    }

    /// Half road width at arc position `arc`, located the same way
    /// [`Track::point_at_arc`] walks the polyline. Uniform-width tracks
    /// return their half width exactly.
    pub fn road_half_width_at_arc(&self, arc: f32) -> f32 {
        let (seg, remaining) = self.locate_arc(arc);
        let len = self.points[seg].distance(self.points[seg + 1]);
        let frac = if len > 1e-6 { remaining / len } else { 0.0 };
        let w0 = self.widths[seg];
        (w0 + (self.widths[seg + 1] - w0) * frac) * 0.5
    }

    /// Finds the closest polyline segment to `pose`.
    pub fn nearest_segment(&self, pose: Vec2) -> NearestSegment {
        let segment_count = self.points.len() - 1;
        let mut best_seg = 0;
        let mut best_point = self.points[0];
        let mut best_dist_sq = f32::INFINITY;
        let mut best_t = 0.0;

        for i in 0..segment_count {
            let a = self.points[i];
            let b = self.points[i + 1];
            let ab = b - a;
            let len_sq = ab.length_squared();
            let t = if len_sq > 0.0 {
                ((pose - a).dot(ab) / len_sq).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let candidate = a + ab * t;
            let dist_sq = pose.distance_squared(candidate);
            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_point = candidate;
                best_seg = i;
                best_t = t;
            }
        }

        NearestSegment {
            segment_index: best_seg,
            closest_point: best_point,
            distance: best_dist_sq.sqrt(),
            t: best_t,
        }
    }

    /// Samples the Track surface at `pose`. If within the LOCAL road half
    /// width (linearly interpolated between the nearest segment's vertex
    /// widths) of the polyline centerline, returns the segment's authored
    /// surface. Outside the road corridor, returns off-track
    /// [`Surface::Grass`].
    pub fn sample_surface(&self, pose: Vec2) -> Surface {
        let nearest = self.nearest_segment(pose);
        if nearest.distance <= self.road_width_at_nearest(nearest) * 0.5 {
            self.surfaces[nearest.segment_index]
        } else {
            Surface::Grass
        }
    }

    /// Checks if `pose` has reached or penetrated a Track boundary wall at
    /// the LOCAL wall distance (following the interpolated width). If so,
    pub fn wall_contact(&self, pose: Vec2) -> Option<WallContact> {
        let nearest = self.nearest_segment(pose);
        let wall_dist = self.road_width_at_nearest(nearest);
        if nearest.distance > wall_dist {
            let penetration = nearest.distance - wall_dist;
            let normal = if nearest.distance > 1e-6 {
                (nearest.closest_point - pose) / nearest.distance
            } else {
                Vec2::ZERO
            };
            Some(WallContact {
                normal,
                penetration,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_circuit_parses_into_track_with_geometry_and_surfaces() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();

        assert_eq!(track.name, "Sample Circuit");
        assert_eq!(track.widths, vec![14.0; track.points.len()]);

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

    /// Raw track text with an optional per-vertex widths array.
    fn track_text_widths(points: &str, width: &str, widths: &str, surfaces: &str) -> String {
        format!(
            r#"{{"name": "T", "width": {width}, "widths": {widths}, "points": {points}, "surfaces": {surfaces}}}"#
        )
    }

    #[test]
    fn widths_array_wins_over_the_global_width() {
        let text = track_text_widths(
            "[[0,0],[10,0],[10,10],[0,0]]",
            "10",
            "[8.0, 12.0, 16.0, 8.0]",
            "[]",
        );
        let track = Track::parse(&text).unwrap();
        assert_eq!(track.widths, vec![8.0, 12.0, 16.0, 8.0]);
        assert_eq!(track.widths.len(), track.points.len());

        // The global width is a fallback, not a co-field: an authored
        // widths array is used as-is (no minimum is derived anymore).
        let text = track_text_widths(
            "[[0,0],[10,0],[10,10],[0,0]]",
            "10",
            "[8.0, 12.0, 16.0, 99.0]",
            "[]",
        );
        let track = Track::parse(&text).unwrap();
        assert_eq!(track.widths, vec![8.0, 12.0, 16.0, 8.0]);
    }

    #[test]
    fn global_width_without_widths_resolves_a_uniform_array() {
        let track = Track::parse(&closed_triangle_text()).unwrap();
        assert_eq!(track.widths, vec![10.0; track.points.len()]);
    }

    #[test]
    fn widths_only_json_loads_without_a_global_width() {
        let text = r#"{
            "name": "Widths Only",
            "widths": [8.0, 12.0, 16.0, 8.0],
            "points": [[0,0],[10,0],[10,10],[0,0]],
            "surfaces": []
        }"#;
        let track = Track::parse(text).unwrap();
        assert_eq!(track.widths, vec![8.0, 12.0, 16.0, 8.0]);
    }

    #[test]
    fn json_with_neither_width_nor_widths_is_rejected() {
        let text = r#"{
            "name": "No Width",
            "points": [[0,0],[10,0],[10,10],[0,0]],
            "surfaces": []
        }"#;
        let err = Track::parse(text).unwrap_err();
        assert!(matches!(err, TrackParseError::MissingWidth), "{err:?}");
        assert!(err.to_string().contains("widths"));
    }

    #[test]
    fn widths_length_mismatch_is_rejected_with_descriptive_error() {
        // The triangle has 4 points (closing repeat included).
        for widths in ["[]", "[10.0, 10.0]", "[10.0, 10.0, 10.0, 10.0, 10.0]"] {
            let text = track_text_widths("[[0,0],[10,0],[10,10],[0,0]]", "10", widths, "[]");
            let err = Track::parse(&text).unwrap_err();
            assert!(
                matches!(err, TrackParseError::WidthsLengthMismatch { .. }),
                "{widths}: {err:?}"
            );
            let msg = err.to_string();
            assert!(msg.contains("widths"), "message: {msg}");
        }
    }

    #[test]
    fn non_positive_vertex_width_is_rejected_with_index() {
        for (widths, index) in [
            ("[10.0, 0.0, 10.0, 10.0]", 1),
            ("[10.0, 10.0, -3.0, 10.0]", 2),
        ] {
            let text = track_text_widths("[[0,0],[10,0],[10,10],[0,0]]", "10", widths, "[]");
            let err = Track::parse(&text).unwrap_err();
            assert!(
                matches!(err, TrackParseError::InvalidVertexWidth { index: i, .. } if i == index),
                "{widths}: {err:?}"
            );
            let msg = err.to_string();
            assert!(msg.contains(&format!("widths[{index}]")), "message: {msg}");
        }

        // Boundary: small positive vertex widths parse.
        let text = track_text_widths(
            "[[0,0],[10,0],[10,10],[0,0]]",
            "10",
            "[0.5, 10.0, 10.0, 0.5]",
            "[]",
        );
        assert!(Track::parse(&text).is_ok());
    }

    #[test]
    fn road_width_interpolates_between_vertex_widths_and_is_exact_when_uniform() {
        let track = Track::parse(&track_text_widths(
            "[[0,0],[120,0],[120,60],[0,60],[0,0]]",
            "10.0",
            "[10.0, 30.0, 30.0, 10.0, 10.0]",
            "[]",
        ))
        .unwrap();

        // Interpolation follows the nearest segment, parameterized by the
        // projection along it; lateral offset does not change the width.
        assert_eq!(track.road_half_width_at(Vec2::new(0.0, 0.0)), 5.0);
        assert_eq!(track.road_half_width_at(Vec2::new(60.0, 0.0)), 10.0);
        assert_eq!(track.road_half_width_at(Vec2::new(60.0, 5.0)), 10.0);
        assert_eq!(track.road_half_width_at(Vec2::new(120.0, 0.0)), 15.0);
        assert_eq!(track.road_half_width_at(Vec2::new(120.0, 30.0)), 15.0);

        // Wall distance keeps the authored ratio: walls sit at the full
        // local width from the centerline.
        assert_eq!(track.wall_distance_at(Vec2::new(60.0, 0.0)), 20.0);
        assert_eq!(track.wall_distance_at(Vec2::new(120.0, 30.0)), 30.0);

        // Arc-space queries interpolate identically for the AI's line clamp.
        assert_eq!(track.road_half_width_at_arc(0.0), 5.0);
        assert_eq!(track.road_half_width_at_arc(60.0), 10.0);
        assert_eq!(track.road_half_width_at_arc(120.0), 15.0);
        assert_eq!(track.road_half_width_at_arc(180.0), 15.0);
    }

    #[test]
    fn local_width_queries_are_bit_exact_no_ops_on_uniform_width_tracks() {
        let track = Track::parse(&closed_triangle_text()).unwrap();
        for pose in [
            Vec2::new(0.0, 0.0),
            Vec2::new(5.0, 0.0),
            Vec2::new(5.0, 5.0),
            Vec2::new(2.5, 7.0),
        ] {
            assert_eq!(track.road_half_width_at(pose), 5.0);
            assert_eq!(track.wall_distance_at(pose), 10.0);
        }
        for arc in [0.0, 4.25, 11.7, 25.0] {
            assert_eq!(track.road_half_width_at_arc(arc), 5.0);
        }
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

    #[test]
    fn surface_sampling_identifies_road_gravel_and_off_track_grass() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        // Road half-width is 7.0 (uniform widths 14.0).
        assert_eq!(track.road_half_width_at(Vec2::new(50.0, 0.0)), 7.0);

        // Segment 0 is Road: (0,0) -> (120,0).
        // On centerline:
        assert_eq!(track.sample_surface(Vec2::new(50.0, 0.0)), Surface::Road);
        // Within road half-width (y = 5.0 <= 7.0):
        assert_eq!(track.sample_surface(Vec2::new(50.0, 5.0)), Surface::Road);
        assert_eq!(track.sample_surface(Vec2::new(50.0, -6.5)), Surface::Road);

        // Segment 2 is Gravel: (160,30) -> (160,90).
        // Within road half-width on gravel segment:
        assert_eq!(
            track.sample_surface(Vec2::new(160.0, 60.0)),
            Surface::Gravel
        );
        assert_eq!(
            track.sample_surface(Vec2::new(164.0, 60.0)),
            Surface::Gravel
        );

        // Off-road (distance > 7.0) is always off-track Grass:
        // Beyond road edge on segment 0 (y = 9.0 > 7.0):
        assert_eq!(track.sample_surface(Vec2::new(50.0, 9.0)), Surface::Grass);
        assert_eq!(track.sample_surface(Vec2::new(50.0, -10.0)), Surface::Grass);
        // Beyond road edge on gravel segment (x = 170.0, distance 10.0 > 7.0):
        assert_eq!(track.sample_surface(Vec2::new(170.0, 60.0)), Surface::Grass);
    }

    #[test]
    fn surface_classification_uses_local_width_not_the_global_minimum() {
        let track = Track::parse(&track_text_widths(
            "[[0,0],[120,0],[120,60],[0,60],[0,0]]",
            "10.0",
            "[10.0, 30.0, 30.0, 10.0, 10.0]",
            "[]",
        ))
        .unwrap();
        // (local half width ~6.7), on-road at the wide end (local half
        // width ~13.3). The global-minimum floor (half 5) would call both
        // Grass.
        assert_eq!(track.sample_surface(Vec2::new(20.0, 8.0)), Surface::Grass);
        assert_eq!(track.sample_surface(Vec2::new(100.0, 8.0)), Surface::Road);
    }

    #[test]
    fn wall_contact_uses_local_width_not_the_global_minimum() {
        let track = Track::parse(&track_text_widths(
            "[[0,0],[120,0],[120,60],[0,60],[0,0]]",
            "10.0",
            "[10.0, 30.0, 30.0, 10.0, 10.0]",
            "[]",
        ))
        .unwrap();

        // Sixteen units off the centerline: past the wall at the narrow
        // end (local wall ~13.3), clear at the wide end (local wall ~26.7).
        // The global-minimum floor (wall 10) would flag both as contact.
        assert!(track.wall_contact(Vec2::new(20.0, 16.0)).is_some());
        assert!(track.wall_contact(Vec2::new(100.0, 16.0)).is_none());
    }

    #[test]
    fn wall_contact_detects_boundary_penetration_and_inward_normal() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        // Wall distance is the local full width (uniform track: 14.0).
        assert_eq!(track.wall_distance_at(Vec2::new(50.0, 0.0)), 14.0);

        // Within wall boundary: no wall contact.
        assert!(track.wall_contact(Vec2::new(50.0, 0.0)).is_none());
        assert!(track.wall_contact(Vec2::new(50.0, 7.0)).is_none());
        assert!(track.wall_contact(Vec2::new(50.0, 13.9)).is_none());

        // Contact at y = 16.0 on segment 0 (centerline y=0, wall at y=14.0):
        // Penetration is 2.0; inward normal points down towards track centerline (0, -1).
        let contact = track.wall_contact(Vec2::new(50.0, 16.0)).unwrap();
        assert!((contact.penetration - 2.0).abs() < 1e-4);
        assert!((contact.normal.x).abs() < 1e-4);
        assert!((contact.normal.y - (-1.0)).abs() < 1e-4);

        // Contact on the other side (y = -18.0):
        // Penetration is 4.0; inward normal points up (0, 1).
        let contact_other = track.wall_contact(Vec2::new(50.0, -18.0)).unwrap();
        assert!((contact_other.penetration - 4.0).abs() < 1e-4);
        assert!((contact_other.normal.x).abs() < 1e-4);
        assert!((contact_other.normal.y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn arc_queries_round_trip_and_match_spawn_pose() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        let total = track.total_length();
        assert!(
            (total - 498.3).abs() < 0.5,
            "sample circuit length, got {total}"
        );

        // Arc 0 is the start line; advancing the full loop wraps around.
        assert!((track.point_at_arc(0.0) - Vec2::new(0.0, 0.0)).length() < 1e-4);
        assert!((track.point_at_arc(total) - Vec2::new(0.0, 0.0)).length() < 1e-3);
        assert!((track.point_at_arc(120.0) - Vec2::new(120.0, 0.0)).length() < 1e-3);

        // spawn_pose(distance) is the point `distance` back from the start.
        for distance in [0.0, 5.0, 10.0, 130.0] {
            let spawned = track.spawn_pose(distance);
            let expected = track.point_at_arc(total - distance);
            assert!(
                (spawned - expected).length() < 1e-3,
                "spawn {distance}: {spawned:?} vs {expected:?}"
            );
        }

        // The centerline frame of an on-line pose reports arc, direction, zero lateral.
        let frame = track.centerline_frame(Vec2::new(50.0, 0.0));
        assert!((frame.arc - 50.0).abs() < 1e-3, "arc {}", frame.arc);
        assert!((frame.direction - Vec2::X).length() < 1e-4);
        assert!(frame.lateral.abs() < 1e-4);

        // Off-center poses report signed lateral offset: left is positive.
        assert!(track.centerline_frame(Vec2::new(50.0, 3.0)).lateral > 2.9);
        assert!(track.centerline_frame(Vec2::new(50.0, -3.0)).lateral < -2.9);
    }
    /// A triangle circuit exercising every v2 field: props (one rotated, one
    /// default-oriented), one terrain zone, an explicit theme, and a
    /// start-line override.
    fn v2_track_text() -> String {
        r#"{
            "name": "V2",
            "width": 10.0,
            "points": [[0,0],[10,0],[10,10],[0,0]],
            "surfaces": [],
            "props": [
                {"type": "tree", "position": [5.0, 5.0], "rotation_degrees": 90.0},
                {"type": "brake_board", "position": [8.0, 1.0]}
            ],
            "terrain_zones": [
                {"kind": "dirt", "polygon": [[20,20],[30,20],[30,30],[20,30]]}
            ],
            "theme": "hillside",
            "start_line": {"segment": 1}
        }"#
        .to_owned()
    }

    #[test]
    fn v2_fields_parse_into_typed_props_zones_theme_and_start_segment() {
        let track = Track::parse(&v2_track_text()).unwrap();
        assert_eq!(track.props.len(), 2);
        assert_eq!(track.props[0].kind, PropKind::Tree);
        assert_eq!(track.props[0].position, Vec2::new(5.0, 5.0));
        assert!(
            (track.props[0].rotation - std::f32::consts::FRAC_PI_2).abs() < 1e-4,
            "rotation {}",
            track.props[0].rotation
        );
        assert_eq!(track.props[1].kind, PropKind::BrakeBoard);
        assert_eq!(track.props[1].rotation, 0.0);
        assert_eq!(track.zones.len(), 1);
        assert_eq!(track.zones[0].kind, ZoneKind::Dirt);
        assert_eq!(track.zones[0].polygon.len(), 4);
        assert_eq!(track.theme, Theme::Hillside);
        assert_eq!(track.start_segment, 1);
    }

    #[test]
    fn v1_tracks_parse_with_v2_defaults() {
        let track = Track::parse(SAMPLE_CIRCUIT).unwrap();
        assert!(track.props.is_empty());
        assert!(track.zones.is_empty());
        assert_eq!(track.theme, Theme::Hillside);
        assert_eq!(track.start_segment, 0);

        let bare = Track::parse(&closed_triangle_text()).unwrap();
        assert!(bare.props.is_empty());
        assert!(bare.zones.is_empty());
        assert_eq!(bare.theme, Theme::Hillside);
        assert_eq!(bare.start_segment, 0);
    }

    #[test]
    fn unknown_prop_type_is_rejected_with_descriptive_error() {
        let text = v2_track_text().replace(
            r#"{"type": "tree", "position": [5.0, 5.0], "rotation_degrees": 90.0}"#,
            r#"{"type": "rock", "position": [5.0, 5.0]}"#,
        );
        let err = Track::parse(&text).unwrap_err();
        assert!(
            matches!(err, TrackParseError::UnknownPropType { .. }),
            "{err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("unknown prop"), "message: {msg}");
        assert!(msg.contains("'rock'"), "message: {msg}");
        assert!(msg.contains("tree"), "message: {msg}");
    }

    #[test]
    fn unknown_zone_kind_is_rejected_with_descriptive_error() {
        let text = v2_track_text().replace(
            r#"{"kind": "dirt", "polygon": [[20,20],[30,20],[30,30],[20,30]]}"#,
            r#"{"kind": "lava", "polygon": [[20,20],[30,20],[30,30],[20,30]]}"#,
        );
        let err = Track::parse(&text).unwrap_err();
        assert!(
            matches!(err, TrackParseError::UnknownZoneKind { .. }),
            "{err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("unknown terrain zone"), "message: {msg}");
        assert!(msg.contains("'lava'"), "message: {msg}");
        assert!(msg.contains("sand"), "message: {msg}");
    }

    #[test]
    fn degenerate_zone_is_rejected_with_descriptive_error() {
        let text = v2_track_text().replace(
            "[[20,20],[30,20],[30,30],[20,30]]",
            "[[20,20],[30,20],[20,20]]",
        );
        let err = Track::parse(&text).unwrap_err();
        assert!(
            matches!(err, TrackParseError::DegenerateZone { .. }),
            "{err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("distinct vertices"), "message: {msg}");
    }

    #[test]
    fn invalid_start_segment_is_rejected_with_descriptive_error() {
        let text = v2_track_text().replace(
            r#""start_line": {"segment": 1}"#,
            r#""start_line": {"segment": 9}"#,
        );
        let err = Track::parse(&text).unwrap_err();
        assert!(
            matches!(err, TrackParseError::InvalidStartSegment { .. }),
            "{err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("segment"), "message: {msg}");

        // Boundary: the last valid segment index parses.
        let ok = v2_track_text().replace(
            r#""start_line": {"segment": 1}"#,
            r#""start_line": {"segment": 2}"#,
        );
        assert_eq!(Track::parse(&ok).unwrap().start_segment, 2);
    }

    #[test]
    fn unknown_theme_is_rejected_with_descriptive_error() {
        let text = v2_track_text().replace(r#""theme": "hillside""#, r#""theme": "desert""#);
        let err = Track::parse(&text).unwrap_err();
        assert!(
            matches!(err, TrackParseError::UnknownTheme { .. }),
            "{err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("unknown theme"), "message: {msg}");
        assert!(msg.contains("'desert'"), "message: {msg}");
        assert!(msg.contains("hillside"), "message: {msg}");
    }
}
