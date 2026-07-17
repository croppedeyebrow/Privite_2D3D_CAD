//! Tool state machine and pure geometry-building/selection helpers. Kept
//! free of `egui` types (screen-space values are plain `(f32, f32)` tuples)
//! so this logic can be unit tested without a running GUI — the same
//! separation `camera.rs` uses.

use cad_core::{
    AngleRad, Arc, Circle, Drawing, EntityGeometry, EntityId, LengthMm, Line, Point2, Rectangle,
    Text,
};

pub const DEFAULT_TEXT_HEIGHT_MM: f64 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Line,
    Rectangle,
    Circle,
    Arc,
    Text,
}

impl Tool {
    pub const DRAWING_TOOLS: [Tool; 5] = [
        Tool::Line,
        Tool::Rectangle,
        Tool::Circle,
        Tool::Arc,
        Tool::Text,
    ];

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "선택",
            Tool::Line => "선",
            Tool::Rectangle => "사각형",
            Tool::Circle => "원",
            Tool::Arc => "호",
            Tool::Text => "텍스트",
        }
    }
}

/// In-progress input for the currently active tool. Cleared whenever the
/// tool changes.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum DrawState {
    #[default]
    Idle,
    LineStart((f64, f64)),
    RectangleStart((f64, f64)),
    CircleCenter((f64, f64)),
    ArcCenter((f64, f64)),
    ArcStart {
        center: (f64, f64),
        start: (f64, f64),
    },
    TextPending {
        origin: (f64, f64),
        content: String,
    },
    /// Dragging the selected entity. `screen_delta` accumulates raw pixel
    /// motion for live preview; it is converted to a world-space
    /// `MoveEntity` only when the drag ends.
    Moving {
        entity_id: EntityId,
        screen_delta: (f32, f32),
    },
}

#[must_use]
pub fn line_geometry(start: (f64, f64), end: (f64, f64)) -> EntityGeometry {
    EntityGeometry::Line(Line {
        start: Point2::new(start.0, start.1),
        end: Point2::new(end.0, end.1),
    })
}

/// Normalizes two arbitrary corner points (the user may drag in any
/// direction) into an origin + positive width/height.
#[must_use]
pub fn rectangle_geometry(a: (f64, f64), b: (f64, f64)) -> EntityGeometry {
    EntityGeometry::Rectangle(Rectangle {
        origin: Point2::new(a.0.min(b.0), a.1.min(b.1)),
        width: LengthMm((a.0 - b.0).abs()),
        height: LengthMm((a.1 - b.1).abs()),
    })
}

#[must_use]
pub fn circle_geometry(center: (f64, f64), edge: (f64, f64)) -> EntityGeometry {
    let radius =
        cad_geometry::distance(Point2::new(center.0, center.1), Point2::new(edge.0, edge.1));
    EntityGeometry::Circle(Circle {
        center: Point2::new(center.0, center.1),
        radius,
    })
}

/// Builds an arc from a center, a point defining the radius and start
/// angle, and a point defining only the end angle (its distance from the
/// center is ignored). The sweep always goes counter-clockwise from start
/// to end — there is no way with this three-click input alone to request
/// the reflex (other-side) arc.
#[must_use]
pub fn arc_geometry(center: (f64, f64), start: (f64, f64), end: (f64, f64)) -> EntityGeometry {
    let radius = cad_geometry::distance(
        Point2::new(center.0, center.1),
        Point2::new(start.0, start.1),
    );
    let start_angle = (start.1 - center.1).atan2(start.0 - center.0);
    let end_angle = (end.1 - center.1).atan2(end.0 - center.0);

    let mut sweep = end_angle - start_angle;
    while sweep <= 0.0 {
        sweep += std::f64::consts::TAU;
    }

    EntityGeometry::Arc(Arc {
        center: Point2::new(center.0, center.1),
        radius,
        start_angle: AngleRad(start_angle),
        sweep_angle: AngleRad(sweep),
    })
}

#[must_use]
pub fn text_geometry(origin: (f64, f64), content: String) -> EntityGeometry {
    EntityGeometry::Text(Text {
        origin: Point2::new(origin.0, origin.1),
        content,
        height: LengthMm(DEFAULT_TEXT_HEIGHT_MM),
    })
}

/// Finds the entity whose nearest snap candidate point (endpoint, midpoint,
/// center, corner — see `cad_geometry::snap_candidates`) is closest to
/// `world_point`, within `tolerance_mm`. Entities on a hidden layer are not
/// selectable, matching what's actually visible on screen.
#[must_use]
pub fn hit_test(drawing: &Drawing, world_point: (f64, f64), tolerance_mm: f64) -> Option<EntityId> {
    let target = Point2::new(world_point.0, world_point.1);
    let tolerance = LengthMm(tolerance_mm);
    let mut best: Option<(EntityId, f64)> = None;

    for entity in &drawing.entities {
        if !cad_render::is_layer_visible(drawing, entity.layer_id) {
            continue;
        }
        let candidates = cad_geometry::snap_candidates(&entity.geometry);
        if let Some(hit) = cad_geometry::nearest_point(target, &candidates, tolerance) {
            let dist = cad_geometry::distance(target, hit).0;
            if best.is_none_or(|(_, best_dist)| dist < best_dist) {
                best = Some((entity.id, dist));
            }
        }
    }

    best.map(|(id, _)| id)
}

/// Snaps `raw` to the nearest visible entity's snap candidate within
/// `tolerance_mm`, or returns `raw` unchanged if nothing is close enough.
#[must_use]
pub fn snap_point(drawing: &Drawing, raw: (f64, f64), tolerance_mm: f64) -> (f64, f64) {
    let target = Point2::new(raw.0, raw.1);
    let mut candidates = Vec::new();
    for entity in &drawing.entities {
        if cad_render::is_layer_visible(drawing, entity.layer_id) {
            candidates.extend(cad_geometry::snap_candidates(&entity.geometry));
        }
    }
    cad_geometry::nearest_point(target, &candidates, LengthMm(tolerance_mm))
        .map_or(raw, |p| (p.x.0, p.y.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_core::{Entity, Layer, LayerId, Project, DEFAULT_LAYER_ID};

    fn drawing_with_line(start: (f64, f64), end: (f64, f64)) -> Drawing {
        let mut project = Project::default();
        project
            .drawing
            .add_entity(Entity {
                id: EntityId::new(1),
                layer_id: DEFAULT_LAYER_ID,
                geometry: line_geometry(start, end),
            })
            .unwrap();
        project.drawing
    }

    #[test]
    fn line_geometry_builds_expected_line() {
        let geometry = line_geometry((0.0, 0.0), (10.0, 5.0));
        assert_eq!(
            geometry,
            EntityGeometry::Line(Line {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(10.0, 5.0),
            })
        );
    }

    #[test]
    fn rectangle_geometry_normalizes_corners_dragged_up_left() {
        // Dragged from bottom-right to top-left, i.e. "backwards".
        let geometry = rectangle_geometry((10.0, 10.0), (0.0, 0.0));
        assert_eq!(
            geometry,
            EntityGeometry::Rectangle(Rectangle {
                origin: Point2::new(0.0, 0.0),
                width: LengthMm(10.0),
                height: LengthMm(10.0),
            })
        );
    }

    #[test]
    fn circle_geometry_uses_distance_to_edge_as_radius() {
        let geometry = circle_geometry((0.0, 0.0), (3.0, 4.0));
        assert_eq!(
            geometry,
            EntityGeometry::Circle(Circle {
                center: Point2::new(0.0, 0.0),
                radius: LengthMm(5.0),
            })
        );
    }

    #[test]
    fn arc_geometry_sweeps_positive_ccw_between_start_and_end() {
        let geometry = arc_geometry((0.0, 0.0), (1.0, 0.0), (0.0, 1.0));
        let EntityGeometry::Arc(arc) = geometry else {
            panic!("expected an arc");
        };
        assert!((arc.start_angle.0 - 0.0).abs() < 1.0e-9);
        assert!((arc.sweep_angle.0 - std::f64::consts::FRAC_PI_2).abs() < 1.0e-9);
    }

    #[test]
    fn arc_geometry_wraps_when_end_angle_precedes_start_angle() {
        // start at 90°, end at 0°: going CCW must wrap almost all the way around.
        let geometry = arc_geometry((0.0, 0.0), (0.0, 1.0), (1.0, 0.0));
        let EntityGeometry::Arc(arc) = geometry else {
            panic!("expected an arc");
        };
        assert!(arc.sweep_angle.0 > 0.0);
        assert!(
            (arc.sweep_angle.0 - (std::f64::consts::TAU - std::f64::consts::FRAC_PI_2)).abs()
                < 1.0e-9
        );
    }

    #[test]
    fn text_geometry_uses_default_height() {
        let geometry = text_geometry((1.0, 2.0), "label".to_owned());
        assert_eq!(
            geometry,
            EntityGeometry::Text(Text {
                origin: Point2::new(1.0, 2.0),
                content: "label".to_owned(),
                height: LengthMm(DEFAULT_TEXT_HEIGHT_MM),
            })
        );
    }

    #[test]
    fn hit_test_finds_entity_within_tolerance() {
        let drawing = drawing_with_line((0.0, 0.0), (10.0, 0.0));
        assert_eq!(hit_test(&drawing, (0.2, 0.2), 1.0), Some(EntityId::new(1)));
    }

    #[test]
    fn hit_test_returns_none_outside_tolerance() {
        let drawing = drawing_with_line((0.0, 0.0), (10.0, 0.0));
        assert_eq!(hit_test(&drawing, (5.0, 5.0), 1.0), None);
    }

    #[test]
    fn hit_test_skips_entities_on_hidden_layers() {
        let mut project = Project::default();
        let hidden = LayerId::new(1);
        let mut layer = Layer::new(hidden, "hidden");
        layer.visible = false;
        project.drawing.add_layer(layer).unwrap();
        project
            .drawing
            .add_entity(Entity {
                id: EntityId::new(1),
                layer_id: hidden,
                geometry: line_geometry((0.0, 0.0), (10.0, 0.0)),
            })
            .unwrap();

        assert_eq!(hit_test(&project.drawing, (0.0, 0.0), 1.0), None);
    }

    #[test]
    fn snap_point_snaps_to_a_nearby_candidate() {
        let drawing = drawing_with_line((0.0, 0.0), (10.0, 0.0));
        assert_eq!(snap_point(&drawing, (0.3, 0.3), 1.0), (0.0, 0.0));
    }

    #[test]
    fn snap_point_returns_raw_point_when_nothing_in_range() {
        let drawing = drawing_with_line((0.0, 0.0), (10.0, 0.0));
        assert_eq!(snap_point(&drawing, (5.0, 5.0), 1.0), (5.0, 5.0));
    }
}
