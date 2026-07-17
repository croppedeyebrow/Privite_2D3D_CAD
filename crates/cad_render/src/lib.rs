#![forbid(unsafe_code)]

use cad_core::{DimensionKind, Drawing, EntityGeometry, LayerId, Project};

#[derive(Clone, Debug, PartialEq)]
pub enum RenderPrimitive {
    Line {
        start: (f64, f64),
        end: (f64, f64),
    },
    Polyline {
        points: Vec<(f64, f64)>,
        closed: bool,
    },
    Rectangle {
        origin: (f64, f64),
        width: f64,
        height: f64,
    },
    Circle {
        center: (f64, f64),
        radius: f64,
    },
    Arc {
        center: (f64, f64),
        radius: f64,
        start_angle: f64,
        sweep_angle: f64,
    },
    Text {
        origin: (f64, f64),
        content: String,
        height: f64,
    },
    /// Raw dimension geometry. Laying out the actual dimension line and
    /// extension lines from `start`/`end`/`offset` is a rendering-backend
    /// decision left to the UI layer (Phase 8).
    Dimension {
        kind: DimensionKind,
        start: (f64, f64),
        end: (f64, f64),
        offset: f64,
    },
}

fn is_layer_visible(drawing: &Drawing, layer_id: LayerId) -> bool {
    drawing.layer(layer_id).is_none_or(|layer| layer.visible)
}

fn entity_primitive(geometry: &EntityGeometry) -> RenderPrimitive {
    match geometry {
        EntityGeometry::Line(line) => RenderPrimitive::Line {
            start: (line.start.x.0, line.start.y.0),
            end: (line.end.x.0, line.end.y.0),
        },
        EntityGeometry::Polyline(polyline) => RenderPrimitive::Polyline {
            points: polyline.points.iter().map(|p| (p.x.0, p.y.0)).collect(),
            closed: polyline.closed,
        },
        EntityGeometry::Rectangle(rect) => RenderPrimitive::Rectangle {
            origin: (rect.origin.x.0, rect.origin.y.0),
            width: rect.width.0,
            height: rect.height.0,
        },
        EntityGeometry::Circle(circle) => RenderPrimitive::Circle {
            center: (circle.center.x.0, circle.center.y.0),
            radius: circle.radius.0,
        },
        EntityGeometry::Arc(arc) => RenderPrimitive::Arc {
            center: (arc.center.x.0, arc.center.y.0),
            radius: arc.radius.0,
            start_angle: arc.start_angle.0,
            sweep_angle: arc.sweep_angle.0,
        },
        EntityGeometry::Text(text) => RenderPrimitive::Text {
            origin: (text.origin.x.0, text.origin.y.0),
            content: text.content.clone(),
            height: text.height.0,
        },
    }
}

/// Converts a project's drawing into a flat, ordered list of render
/// primitives. Entities and dimensions on a hidden layer (`Layer::visible ==
/// false`) are excluded. The output order always follows the drawing's
/// entity order followed by its dimension order, so the same project
/// produces the same primitive list every time.
#[must_use]
pub fn build_render_model(project: &Project) -> Vec<RenderPrimitive> {
    let drawing = &project.drawing;
    let mut primitives = Vec::with_capacity(drawing.entities.len() + drawing.dimensions.len());

    for entity in &drawing.entities {
        if is_layer_visible(drawing, entity.layer_id) {
            primitives.push(entity_primitive(&entity.geometry));
        }
    }

    for dimension in &drawing.dimensions {
        if is_layer_visible(drawing, dimension.layer_id) {
            primitives.push(RenderPrimitive::Dimension {
                kind: dimension.kind,
                start: (dimension.start.x.0, dimension.start.y.0),
                end: (dimension.end.x.0, dimension.end.y.0),
                offset: dimension.offset.0,
            });
        }
    }

    primitives
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_core::{
        Circle, Dimension, DimensionId, Entity, EntityId, Layer, LengthMm, Line, Point2, Project,
        DEFAULT_LAYER_ID,
    };

    fn line_entity(id: u64, layer_id: LayerId) -> Entity {
        Entity {
            id: EntityId::new(id),
            layer_id,
            geometry: EntityGeometry::Line(Line {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(10.0, 0.0),
            }),
        }
    }

    #[test]
    fn line_entity_maps_to_line_primitive() {
        let mut project = Project::default();
        project
            .drawing
            .add_entity(line_entity(1, DEFAULT_LAYER_ID))
            .unwrap();

        let primitives = build_render_model(&project);
        assert_eq!(
            primitives,
            vec![RenderPrimitive::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            }]
        );
    }

    #[test]
    fn circle_entity_maps_to_circle_primitive() {
        let mut project = Project::default();
        project
            .drawing
            .add_entity(Entity {
                id: EntityId::new(1),
                layer_id: DEFAULT_LAYER_ID,
                geometry: EntityGeometry::Circle(Circle {
                    center: Point2::new(5.0, 5.0),
                    radius: LengthMm(2.5),
                }),
            })
            .unwrap();

        let primitives = build_render_model(&project);
        assert_eq!(
            primitives,
            vec![RenderPrimitive::Circle {
                center: (5.0, 5.0),
                radius: 2.5,
            }]
        );
    }

    #[test]
    fn dimension_maps_to_dimension_primitive() {
        let mut project = Project::default();
        project
            .drawing
            .add_dimension(Dimension {
                id: DimensionId::new(1),
                kind: DimensionKind::Linear,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(100.0, 0.0),
                offset: LengthMm(10.0),
                layer_id: DEFAULT_LAYER_ID,
            })
            .unwrap();

        let primitives = build_render_model(&project);
        assert_eq!(
            primitives,
            vec![RenderPrimitive::Dimension {
                kind: DimensionKind::Linear,
                start: (0.0, 0.0),
                end: (100.0, 0.0),
                offset: 10.0,
            }]
        );
    }

    #[test]
    fn entities_on_hidden_layer_are_excluded() {
        let mut project = Project::default();
        let hidden_layer = LayerId::new(1);
        let mut layer = Layer::new(hidden_layer, "hidden");
        layer.visible = false;
        project.drawing.add_layer(layer).unwrap();
        project
            .drawing
            .add_entity(line_entity(1, hidden_layer))
            .unwrap();
        project
            .drawing
            .add_entity(line_entity(2, DEFAULT_LAYER_ID))
            .unwrap();

        let primitives = build_render_model(&project);
        assert_eq!(primitives.len(), 1);
    }

    #[test]
    fn dimensions_on_hidden_layer_are_excluded() {
        let mut project = Project::default();
        let hidden_layer = LayerId::new(1);
        let mut layer = Layer::new(hidden_layer, "hidden");
        layer.visible = false;
        project.drawing.add_layer(layer).unwrap();
        project
            .drawing
            .add_dimension(Dimension {
                id: DimensionId::new(1),
                kind: DimensionKind::Aligned,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(1.0, 1.0),
                offset: LengthMm(1.0),
                layer_id: hidden_layer,
            })
            .unwrap();

        assert!(build_render_model(&project).is_empty());
    }

    #[test]
    fn render_order_follows_entities_then_dimensions() {
        let mut project = Project::default();
        project
            .drawing
            .add_entity(line_entity(1, DEFAULT_LAYER_ID))
            .unwrap();
        project
            .drawing
            .add_dimension(Dimension {
                id: DimensionId::new(1),
                kind: DimensionKind::Linear,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(10.0, 0.0),
                offset: LengthMm(5.0),
                layer_id: DEFAULT_LAYER_ID,
            })
            .unwrap();
        project
            .drawing
            .add_entity(line_entity(2, DEFAULT_LAYER_ID))
            .unwrap();

        let primitives = build_render_model(&project);
        assert!(matches!(primitives[0], RenderPrimitive::Line { .. }));
        assert!(matches!(primitives[1], RenderPrimitive::Line { .. }));
        assert!(matches!(primitives[2], RenderPrimitive::Dimension { .. }));
    }
}
