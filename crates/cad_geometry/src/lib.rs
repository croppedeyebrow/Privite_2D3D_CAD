#![forbid(unsafe_code)]

use cad_core::{LengthMm, Line, Point2};

const DEFAULT_EPSILON_MM: f64 = 1.0e-9;

#[must_use]
pub fn distance(a: Point2, b: Point2) -> LengthMm {
    let dx = b.x.0 - a.x.0;
    let dy = b.y.0 - a.y.0;
    LengthMm(dx.hypot(dy))
}

#[must_use]
pub fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() <= epsilon
}

#[must_use]
pub fn line_is_degenerate(line: Line) -> bool {
    approx_eq(distance(line.start, line.end).0, 0.0, DEFAULT_EPSILON_MM)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64, y: f64) -> Point2 {
        Point2 {
            x: LengthMm(x),
            y: LengthMm(y),
        }
    }

    #[test]
    fn calculates_distance_in_mm() {
        assert_eq!(distance(point(0.0, 0.0), point(3.0, 4.0)), LengthMm(5.0));
    }

    #[test]
    fn detects_zero_length_line() {
        assert!(line_is_degenerate(Line {
            start: point(1.0, 1.0),
            end: point(1.0, 1.0)
        }));
    }
}
