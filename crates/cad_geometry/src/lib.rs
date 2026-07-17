#![forbid(unsafe_code)]

use std::fmt;

use cad_core::{
    AngleRad, Arc, Circle, EntityGeometry, LengthMm, Line, Point2, Polyline, Rectangle,
};

/// Default tolerance for comparing lengths in millimetres.
pub const DEFAULT_EPSILON_MM: f64 = 1.0e-9;

/// Default tolerance for comparing angles in radians.
pub const DEFAULT_ANGLE_EPSILON_RAD: f64 = 1.0e-9;

#[must_use]
pub fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() <= epsilon
}

fn approx_eq_point(a: Point2, b: Point2) -> bool {
    approx_eq(a.x.0, b.x.0, DEFAULT_EPSILON_MM) && approx_eq(a.y.0, b.y.0, DEFAULT_EPSILON_MM)
}

fn midpoint(a: Point2, b: Point2) -> Point2 {
    Point2::new(a.x.0.midpoint(b.x.0), a.y.0.midpoint(b.y.0))
}

// ---------------------------------------------------------------------------
// Distance
// ---------------------------------------------------------------------------

#[must_use]
pub fn distance(a: Point2, b: Point2) -> LengthMm {
    let dx = b.x.0 - a.x.0;
    let dy = b.y.0 - a.y.0;
    LengthMm(dx.hypot(dy))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryError {
    ZeroLengthLine,
    DegeneratePolyline,
    NonPositiveWidth,
    NonPositiveHeight,
    NonPositiveRadius,
    ZeroSweepAngle,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLengthLine => write!(f, "line has zero length"),
            Self::DegeneratePolyline => write!(f, "polyline has fewer than two distinct points"),
            Self::NonPositiveWidth => write!(f, "rectangle width must be positive"),
            Self::NonPositiveHeight => write!(f, "rectangle height must be positive"),
            Self::NonPositiveRadius => write!(f, "radius must be positive"),
            Self::ZeroSweepAngle => write!(f, "arc sweep angle must not be zero"),
        }
    }
}

impl std::error::Error for GeometryError {}

#[must_use]
pub fn line_is_degenerate(line: &Line) -> bool {
    approx_eq(distance(line.start, line.end).0, 0.0, DEFAULT_EPSILON_MM)
}

/// # Errors
///
/// Returns `ZeroLengthLine` when the endpoints coincide within tolerance.
pub fn validate_line(line: &Line) -> Result<(), GeometryError> {
    if line_is_degenerate(line) {
        return Err(GeometryError::ZeroLengthLine);
    }
    Ok(())
}

/// # Errors
///
/// Returns `DegeneratePolyline` when the polyline has fewer than two points
/// or every point coincides with its neighbours.
pub fn validate_polyline(polyline: &Polyline) -> Result<(), GeometryError> {
    if polyline.points.len() < 2 {
        return Err(GeometryError::DegeneratePolyline);
    }
    let has_distinct_segment = polyline
        .points
        .windows(2)
        .any(|pair| !approx_eq_point(pair[0], pair[1]));
    if !has_distinct_segment {
        return Err(GeometryError::DegeneratePolyline);
    }
    Ok(())
}

/// # Errors
///
/// Returns `NonPositiveWidth`/`NonPositiveHeight` when a dimension is not
/// strictly positive.
pub fn validate_rectangle(rect: &Rectangle) -> Result<(), GeometryError> {
    if rect.width.0 <= 0.0 {
        return Err(GeometryError::NonPositiveWidth);
    }
    if rect.height.0 <= 0.0 {
        return Err(GeometryError::NonPositiveHeight);
    }
    Ok(())
}

/// # Errors
///
/// Returns `NonPositiveRadius` when the radius is not strictly positive.
pub fn validate_circle(circle: &Circle) -> Result<(), GeometryError> {
    if circle.radius.0 <= 0.0 {
        return Err(GeometryError::NonPositiveRadius);
    }
    Ok(())
}

/// # Errors
///
/// Returns `NonPositiveRadius` when the radius is not strictly positive, or
/// `ZeroSweepAngle` when the arc sweeps no angle.
pub fn validate_arc(arc: &Arc) -> Result<(), GeometryError> {
    if arc.radius.0 <= 0.0 {
        return Err(GeometryError::NonPositiveRadius);
    }
    if approx_eq(arc.sweep_angle.0, 0.0, DEFAULT_ANGLE_EPSILON_RAD) {
        return Err(GeometryError::ZeroSweepAngle);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Measurement
// ---------------------------------------------------------------------------

#[must_use]
pub fn polyline_length(polyline: &Polyline) -> LengthMm {
    let mut total = polyline
        .points
        .windows(2)
        .map(|pair| distance(pair[0], pair[1]).0)
        .sum::<f64>();
    if polyline.closed {
        if let (Some(&first), Some(&last)) = (polyline.points.first(), polyline.points.last()) {
            total += distance(last, first).0;
        }
    }
    LengthMm(total)
}

#[must_use]
pub fn rectangle_perimeter(rect: &Rectangle) -> LengthMm {
    LengthMm(2.0 * (rect.width.0 + rect.height.0))
}

#[must_use]
pub fn circle_circumference(circle: &Circle) -> LengthMm {
    LengthMm(2.0 * std::f64::consts::PI * circle.radius.0)
}

#[must_use]
pub fn arc_length(arc: &Arc) -> LengthMm {
    LengthMm(arc.radius.0 * arc.sweep_angle.0.abs())
}

fn point_on_circle(center: Point2, radius: LengthMm, angle: AngleRad) -> Point2 {
    Point2::new(
        center.x.0 + radius.0 * angle.0.cos(),
        center.y.0 + radius.0 * angle.0.sin(),
    )
}

#[must_use]
pub fn arc_start_point(arc: &Arc) -> Point2 {
    point_on_circle(arc.center, arc.radius, arc.start_angle)
}

#[must_use]
pub fn arc_end_point(arc: &Arc) -> Point2 {
    point_on_circle(
        arc.center,
        arc.radius,
        AngleRad(arc.start_angle.0 + arc.sweep_angle.0),
    )
}

#[must_use]
pub fn rectangle_corners(rect: &Rectangle) -> [Point2; 4] {
    let x0 = rect.origin.x.0;
    let y0 = rect.origin.y.0;
    let x1 = x0 + rect.width.0;
    let y1 = y0 + rect.height.0;
    [
        Point2::new(x0, y0),
        Point2::new(x1, y0),
        Point2::new(x1, y1),
        Point2::new(x0, y1),
    ]
}

// ---------------------------------------------------------------------------
// Intersection
// ---------------------------------------------------------------------------

/// Intersects two line *segments* (not their infinite extensions).
///
/// Returns `None` when the segments are parallel, coincident, or do not
/// overlap within their finite extents.
#[must_use]
pub fn line_line_intersection(a: &Line, b: &Line) -> Option<Point2> {
    let (x1, y1) = (a.start.x.0, a.start.y.0);
    let (x2, y2) = (a.end.x.0, a.end.y.0);
    let (x3, y3) = (b.start.x.0, b.start.y.0);
    let (x4, y4) = (b.end.x.0, b.end.y.0);

    let denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if approx_eq(denom, 0.0, DEFAULT_EPSILON_MM) {
        return None;
    }

    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / denom;
    let u = ((x1 - x3) * (y1 - y2) - (y1 - y3) * (x1 - x2)) / denom;

    if !(0.0..=1.0).contains(&t) || !(0.0..=1.0).contains(&u) {
        return None;
    }

    Some(Point2::new(x1 + t * (x2 - x1), y1 + t * (y2 - y1)))
}

// ---------------------------------------------------------------------------
// Transform
// ---------------------------------------------------------------------------

#[must_use]
pub fn translate_point(point: Point2, dx: LengthMm, dy: LengthMm) -> Point2 {
    Point2::new(point.x.0 + dx.0, point.y.0 + dy.0)
}

#[must_use]
pub fn rotate_point(point: Point2, pivot: Point2, angle: AngleRad) -> Point2 {
    let dx = point.x.0 - pivot.x.0;
    let dy = point.y.0 - pivot.y.0;
    let (sin, cos) = angle.0.sin_cos();
    Point2::new(
        pivot.x.0 + dx * cos - dy * sin,
        pivot.y.0 + dx * sin + dy * cos,
    )
}

#[must_use]
pub fn scale_point(point: Point2, pivot: Point2, factor: f64) -> Point2 {
    Point2::new(
        pivot.x.0 + (point.x.0 - pivot.x.0) * factor,
        pivot.y.0 + (point.y.0 - pivot.y.0) * factor,
    )
}

// ---------------------------------------------------------------------------
// Snap
// ---------------------------------------------------------------------------

/// Candidate snap points for a piece of geometry: endpoints, midpoints,
/// centers — whichever are meaningful for the given shape.
#[must_use]
pub fn snap_candidates(geometry: &EntityGeometry) -> Vec<Point2> {
    match geometry {
        EntityGeometry::Line(line) => {
            vec![line.start, line.end, midpoint(line.start, line.end)]
        }
        EntityGeometry::Polyline(polyline) => {
            let mut points = polyline.points.clone();
            points.extend(
                polyline
                    .points
                    .windows(2)
                    .map(|pair| midpoint(pair[0], pair[1])),
            );
            points
        }
        EntityGeometry::Rectangle(rect) => rectangle_corners(rect).to_vec(),
        EntityGeometry::Circle(circle) => vec![circle.center],
        EntityGeometry::Arc(arc) => vec![arc.center, arc_start_point(arc), arc_end_point(arc)],
        EntityGeometry::Text(text) => vec![text.origin],
    }
}

/// Returns the candidate closest to `target` that falls within `tolerance`,
/// or `None` when no candidate is close enough. Ties are broken by the order
/// candidates appear in, so the result is deterministic for a given input.
///
/// # Panics
///
/// Panics if a distance is NaN, which cannot happen for finite coordinates.
#[must_use]
pub fn nearest_point(target: Point2, candidates: &[Point2], tolerance: LengthMm) -> Option<Point2> {
    candidates
        .iter()
        .copied()
        .map(|candidate| (candidate, distance(target, candidate).0))
        .filter(|(_, dist)| *dist <= tolerance.0)
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).expect("distances are finite"))
        .map(|(candidate, _)| candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_core::Text;

    fn point(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn calculates_distance_in_mm() {
        assert_eq!(distance(point(0.0, 0.0), point(3.0, 4.0)), LengthMm(5.0));
    }

    #[test]
    fn detects_zero_length_line() {
        assert!(line_is_degenerate(&Line {
            start: point(1.0, 1.0),
            end: point(1.0, 1.0)
        }));
    }

    #[test]
    fn validate_line_rejects_zero_length() {
        let line = Line {
            start: point(1.0, 1.0),
            end: point(1.0, 1.0),
        };
        assert_eq!(validate_line(&line), Err(GeometryError::ZeroLengthLine));
    }

    #[test]
    fn validate_line_accepts_distinct_endpoints() {
        let line = Line {
            start: point(0.0, 0.0),
            end: point(1.0, 0.0),
        };
        assert_eq!(validate_line(&line), Ok(()));
    }

    #[test]
    fn validate_polyline_rejects_single_point() {
        let polyline = Polyline {
            points: vec![point(0.0, 0.0)],
            closed: false,
        };
        assert_eq!(
            validate_polyline(&polyline),
            Err(GeometryError::DegeneratePolyline)
        );
    }

    #[test]
    fn validate_polyline_rejects_all_coincident_points() {
        let polyline = Polyline {
            points: vec![point(2.0, 2.0), point(2.0, 2.0), point(2.0, 2.0)],
            closed: false,
        };
        assert_eq!(
            validate_polyline(&polyline),
            Err(GeometryError::DegeneratePolyline)
        );
    }

    #[test]
    fn validate_rectangle_rejects_non_positive_dimensions() {
        let rect = Rectangle {
            origin: point(0.0, 0.0),
            width: LengthMm(0.0),
            height: LengthMm(10.0),
        };
        assert_eq!(
            validate_rectangle(&rect),
            Err(GeometryError::NonPositiveWidth)
        );
    }

    #[test]
    fn validate_circle_rejects_non_positive_radius() {
        let circle = Circle {
            center: point(0.0, 0.0),
            radius: LengthMm(-1.0),
        };
        assert_eq!(
            validate_circle(&circle),
            Err(GeometryError::NonPositiveRadius)
        );
    }

    #[test]
    fn validate_arc_rejects_zero_sweep() {
        let arc = Arc {
            center: point(0.0, 0.0),
            radius: LengthMm(10.0),
            start_angle: AngleRad(0.0),
            sweep_angle: AngleRad(0.0),
        };
        assert_eq!(validate_arc(&arc), Err(GeometryError::ZeroSweepAngle));
    }

    #[test]
    fn polyline_length_sums_segments() {
        let polyline = Polyline {
            points: vec![point(0.0, 0.0), point(3.0, 4.0), point(3.0, 0.0)],
            closed: false,
        };
        assert_eq!(polyline_length(&polyline), LengthMm(9.0));
    }

    #[test]
    fn polyline_length_includes_closing_segment_when_closed() {
        let polyline = Polyline {
            points: vec![point(0.0, 0.0), point(10.0, 0.0), point(10.0, 10.0)],
            closed: true,
        };
        let expected = 10.0 + 10.0 + distance(point(10.0, 10.0), point(0.0, 0.0)).0;
        assert!(approx_eq(
            polyline_length(&polyline).0,
            expected,
            DEFAULT_EPSILON_MM
        ));
    }

    #[test]
    fn circle_circumference_matches_formula() {
        let circle = Circle {
            center: point(0.0, 0.0),
            radius: LengthMm(1.0),
        };
        assert!(approx_eq(
            circle_circumference(&circle).0,
            2.0 * std::f64::consts::PI,
            DEFAULT_EPSILON_MM
        ));
    }

    #[test]
    fn arc_endpoints_match_start_and_sweep_angle() {
        let arc = Arc {
            center: point(0.0, 0.0),
            radius: LengthMm(1.0),
            start_angle: AngleRad(0.0),
            sweep_angle: AngleRad(std::f64::consts::FRAC_PI_2),
        };
        let start = arc_start_point(&arc);
        let end = arc_end_point(&arc);
        assert!(approx_eq(start.x.0, 1.0, 1.0e-9) && approx_eq(start.y.0, 0.0, 1.0e-9));
        assert!(approx_eq(end.x.0, 0.0, 1.0e-9) && approx_eq(end.y.0, 1.0, 1.0e-9));
    }

    #[test]
    fn line_line_intersection_finds_crossing_point() {
        let a = Line {
            start: point(0.0, 0.0),
            end: point(10.0, 10.0),
        };
        let b = Line {
            start: point(0.0, 10.0),
            end: point(10.0, 0.0),
        };
        let result = line_line_intersection(&a, &b).expect("segments cross");
        assert!(approx_eq(result.x.0, 5.0, DEFAULT_EPSILON_MM));
        assert!(approx_eq(result.y.0, 5.0, DEFAULT_EPSILON_MM));
    }

    #[test]
    fn line_line_intersection_returns_none_for_parallel_lines() {
        let a = Line {
            start: point(0.0, 0.0),
            end: point(10.0, 0.0),
        };
        let b = Line {
            start: point(0.0, 5.0),
            end: point(10.0, 5.0),
        };
        assert_eq!(line_line_intersection(&a, &b), None);
    }

    #[test]
    fn line_line_intersection_returns_none_outside_segment_extents() {
        let a = Line {
            start: point(0.0, 0.0),
            end: point(1.0, 1.0),
        };
        let b = Line {
            start: point(5.0, 0.0),
            end: point(5.0, -1.0),
        };
        assert_eq!(line_line_intersection(&a, &b), None);
    }

    #[test]
    fn translate_point_shifts_by_delta() {
        let result = translate_point(point(1.0, 1.0), LengthMm(2.0), LengthMm(-3.0));
        assert_eq!(result, point(3.0, -2.0));
    }

    #[test]
    fn rotate_point_quarter_turn_about_origin() {
        let result = rotate_point(
            point(1.0, 0.0),
            point(0.0, 0.0),
            AngleRad(std::f64::consts::FRAC_PI_2),
        );
        assert!(approx_eq(result.x.0, 0.0, 1.0e-9));
        assert!(approx_eq(result.y.0, 1.0, 1.0e-9));
    }

    #[test]
    fn scale_point_scales_about_pivot() {
        let result = scale_point(point(4.0, 4.0), point(2.0, 2.0), 2.0);
        assert_eq!(result, point(6.0, 6.0));
    }

    #[test]
    fn rectangle_corners_are_axis_aligned() {
        let rect = Rectangle {
            origin: point(0.0, 0.0),
            width: LengthMm(10.0),
            height: LengthMm(5.0),
        };
        assert_eq!(
            rectangle_corners(&rect),
            [
                point(0.0, 0.0),
                point(10.0, 0.0),
                point(10.0, 5.0),
                point(0.0, 5.0),
            ]
        );
    }

    #[test]
    fn snap_candidates_for_line_include_endpoints_and_midpoint() {
        let geometry = EntityGeometry::Line(Line {
            start: point(0.0, 0.0),
            end: point(10.0, 0.0),
        });
        let candidates = snap_candidates(&geometry);
        assert_eq!(
            candidates,
            vec![point(0.0, 0.0), point(10.0, 0.0), point(5.0, 0.0)]
        );
    }

    #[test]
    fn snap_candidates_for_text_is_its_origin() {
        let geometry = EntityGeometry::Text(Text {
            origin: point(1.0, 2.0),
            content: "label".to_owned(),
            height: LengthMm(5.0),
        });
        assert_eq!(snap_candidates(&geometry), vec![point(1.0, 2.0)]);
    }

    #[test]
    fn nearest_point_returns_closest_candidate_within_tolerance() {
        let candidates = vec![point(0.0, 0.0), point(10.0, 0.0), point(4.9, 0.0)];
        let result = nearest_point(point(5.0, 0.0), &candidates, LengthMm(1.0));
        assert_eq!(result, Some(point(4.9, 0.0)));
    }

    #[test]
    fn nearest_point_returns_none_when_nothing_in_tolerance() {
        let candidates = vec![point(0.0, 0.0), point(10.0, 0.0)];
        let result = nearest_point(point(5.0, 0.0), &candidates, LengthMm(1.0));
        assert_eq!(result, None);
    }
}
