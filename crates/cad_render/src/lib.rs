#![forbid(unsafe_code)]

use cad_core::{EntityGeometry, Project};

#[derive(Clone, Debug, PartialEq)]
pub enum RenderPrimitive {
    Line { start: (f64, f64), end: (f64, f64) },
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
        })
        .collect()
}
