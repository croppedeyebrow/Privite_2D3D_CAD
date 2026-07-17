#![forbid(unsafe_code)]

use cad_core::{EntityGeometry, Project};

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
}

#[must_use]
pub fn build_render_model(project: &Project) -> Vec<RenderPrimitive> {
    project
        .drawing
        .entities
        .iter()
        .map(|entity| match &entity.geometry {
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
        })
        .collect()
}
