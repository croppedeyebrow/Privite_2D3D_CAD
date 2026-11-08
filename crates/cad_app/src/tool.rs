//! Tool state machine and pure geometry-building/selection helpers. Kept
//! free of `egui` types (screen-space values are plain `(f32, f32)` tuples)
//! so this logic can be unit tested without a running GUI — the same
//! separation `camera.rs` uses.

use std::collections::HashSet;

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
    /// Dragging the current selection (one or many entities together).
    /// `screen_delta` accumulates raw pixel motion for live preview; it is
    /// converted to a world-space `MoveEntities` only when the drag ends.
    Moving {
        entity_ids: Vec<EntityId>,
        screen_delta: (f32, f32),
    },
    /// Dragging a selection box from `anchor` to the current pointer
    /// position on empty canvas. Resolved into a selection set
    /// (`entities_in_box`) when the drag ends.
    BoxSelect {
        anchor: (f64, f64),
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

/// Normalizes `end_angle - start_angle` into a positive counter-clockwise
/// sweep, matching `cad_core::Arc`'s "positive sweep = CCW" convention.
fn ccw_sweep(start_angle: f64, end_angle: f64) -> f64 {
    let mut sweep = end_angle - start_angle;
    while sweep <= 0.0 {
        sweep += std::f64::consts::TAU;
    }
    sweep
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

    EntityGeometry::Arc(Arc {
        center: Point2::new(center.0, center.1),
        radius,
        start_angle: AngleRad(start_angle),
        sweep_angle: AngleRad(ccw_sweep(start_angle, end_angle)),
    })
}

/// A short, human-readable measurement of the shape currently being drawn,
/// updated live against the (already-snapped) pointer position. `None` when
/// the tool isn't mid-draw (nothing to measure yet) or is the select tool.
#[must_use]
pub fn preview_measurement(draw_state: &DrawState, hover: (f64, f64)) -> Option<String> {
    let dist = |a: (f64, f64), b: (f64, f64)| {
        cad_geometry::distance(Point2::new(a.0, a.1), Point2::new(b.0, b.1)).0
    };

    match draw_state {
        DrawState::LineStart(start) => Some(format!("길이: {:.2} mm", dist(*start, hover))),
        DrawState::RectangleStart(start) => Some(format!(
            "폭: {:.2} mm, 높이: {:.2} mm",
            (hover.0 - start.0).abs(),
            (hover.1 - start.1).abs()
        )),
        DrawState::CircleCenter(center) | DrawState::ArcCenter(center) => {
            Some(format!("반지름: {:.2} mm", dist(*center, hover)))
        }
        DrawState::ArcStart { center, start } => {
            let start_angle = (start.1 - center.1).atan2(start.0 - center.0);
            let end_angle = (hover.1 - center.1).atan2(hover.0 - center.0);
            let sweep = ccw_sweep(start_angle, end_angle);
            Some(format!(
                "반지름: {:.2} mm, 각도: {:.1}°",
                dist(*center, *start),
                sweep.to_degrees()
            ))
        }
        _ => None,
    }
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

fn normalize_box(a: (f64, f64), b: (f64, f64)) -> ((f64, f64), (f64, f64)) {
    ((a.0.min(b.0), a.1.min(b.1)), (a.0.max(b.0), a.1.max(b.1)))
}

/// Entities with at least one snap-candidate point (endpoint, midpoint,
/// center, corner) inside the axis-aligned box spanned by `corner_a`/
/// `corner_b` — corner order doesn't matter. Hidden-layer entities are
/// excluded, matching what's actually visible and matching [`hit_test`].
#[must_use]
pub fn entities_in_box(
    drawing: &Drawing,
    corner_a: (f64, f64),
    corner_b: (f64, f64),
) -> HashSet<EntityId> {
    let (min, max) = normalize_box(corner_a, corner_b);
    drawing
        .entities
        .iter()
        .filter(|entity| cad_render::is_layer_visible(drawing, entity.layer_id))
        .filter(|entity| {
            cad_geometry::snap_candidates(&entity.geometry)
                .into_iter()
                .any(|p| p.x.0 >= min.0 && p.x.0 <= max.0 && p.y.0 >= min.1 && p.y.0 <= max.1)
        })
        .map(|entity| entity.id)
        .collect()
}

/// If `id` belongs to a group, returns every member of that group (so
/// clicking one grouped entity selects the whole group); otherwise returns
/// just `id` on its own.
#[must_use]
pub fn expand_group(drawing: &Drawing, id: EntityId) -> HashSet<EntityId> {
    match drawing.group_containing(id) {
        Some(group) => group.member_ids.iter().copied().collect(),
        None => std::iter::once(id).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_core::{Entity, Group, GroupId, Layer, LayerId, Project, DEFAULT_LAYER_ID};

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

    #[test]
    fn entities_in_box_finds_entities_with_a_point_inside() {
        let mut project = Project::default();
        project
            .drawing
            .add_entity(Entity {
                id: EntityId::new(1),
                layer_id: DEFAULT_LAYER_ID,
                geometry: line_geometry((0.0, 0.0), (1.0, 0.0)),
            })
            .unwrap();
        project
            .drawing
            .add_entity(Entity {
                id: EntityId::new(2),
                layer_id: DEFAULT_LAYER_ID,
                geometry: line_geometry((100.0, 100.0), (101.0, 100.0)),
            })
            .unwrap();

        let found = entities_in_box(&project.drawing, (-5.0, -5.0), (5.0, 5.0));
        assert_eq!(found, [EntityId::new(1)].into_iter().collect());
    }

    #[test]
    fn entities_in_box_ignores_corner_order() {
        let drawing = drawing_with_line((0.0, 0.0), (1.0, 0.0));
        // Dragged from bottom-right to top-left, same as Phase 8b's
        // rectangle_geometry normalization test.
        let found = entities_in_box(&drawing, (5.0, 5.0), (-5.0, -5.0));
        assert_eq!(found, [EntityId::new(1)].into_iter().collect());
    }

    #[test]
    fn entities_in_box_skips_hidden_layers() {
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
                geometry: line_geometry((0.0, 0.0), (1.0, 0.0)),
            })
            .unwrap();

        let found = entities_in_box(&project.drawing, (-5.0, -5.0), (5.0, 5.0));
        assert!(found.is_empty());
    }

    #[test]
    fn expand_group_returns_all_members_when_grouped() {
        let mut project = Project::default();
        project.drawing.add_entity(make_entity(1)).unwrap();
        project.drawing.add_entity(make_entity(2)).unwrap();
        project
            .drawing
            .add_group(Group {
                id: GroupId::new(1),
                name: "part-a".to_owned(),
                member_ids: vec![EntityId::new(1), EntityId::new(2)],
            })
            .unwrap();

        let expanded = expand_group(&project.drawing, EntityId::new(1));
        assert_eq!(
            expanded,
            [EntityId::new(1), EntityId::new(2)].into_iter().collect()
        );
    }

    #[test]
    fn expand_group_returns_just_the_entity_when_ungrouped() {
        let mut project = Project::default();
        project.drawing.add_entity(make_entity(1)).unwrap();

        let expanded = expand_group(&project.drawing, EntityId::new(1));
        assert_eq!(expanded, [EntityId::new(1)].into_iter().collect());
    }

    fn make_entity(id: u64) -> Entity {
        Entity {
            id: EntityId::new(id),
            layer_id: DEFAULT_LAYER_ID,
            geometry: line_geometry((0.0, 0.0), (1.0, 0.0)),
        }
    }

    #[test]
    fn preview_measurement_reports_line_length() {
        let state = DrawState::LineStart((0.0, 0.0));
        assert_eq!(
            preview_measurement(&state, (3.0, 4.0)),
            Some("길이: 5.00 mm".to_owned())
        );
    }

    #[test]
    fn preview_measurement_reports_rectangle_width_and_height() {
        let state = DrawState::RectangleStart((0.0, 0.0));
        assert_eq!(
            preview_measurement(&state, (10.0, -5.0)),
            Some("폭: 10.00 mm, 높이: 5.00 mm".to_owned())
        );
    }

    #[test]
    fn preview_measurement_reports_circle_radius() {
        let state = DrawState::CircleCenter((0.0, 0.0));
        assert_eq!(
            preview_measurement(&state, (3.0, 4.0)),
            Some("반지름: 5.00 mm".to_owned())
        );
    }

    #[test]
    fn preview_measurement_reports_arc_radius_and_angle() {
        let state = DrawState::ArcStart {
            center: (0.0, 0.0),
            start: (1.0, 0.0),
        };
        assert_eq!(
            preview_measurement(&state, (0.0, 1.0)),
            Some("반지름: 1.00 mm, 각도: 90.0°".to_owned())
        );
    }

    #[test]
    fn preview_measurement_is_none_when_idle() {
        assert_eq!(preview_measurement(&DrawState::Idle, (1.0, 1.0)), None);
    }
}
