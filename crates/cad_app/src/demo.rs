//! A hardcoded starter drawing so the canvas has something to render before
//! Phase 8d wires up "new project" / "open project". Not a real feature —
//! replaced once file loading exists.

use cad_core::{
    AngleRad, Arc, Circle, Dimension, DimensionId, DimensionKind, Entity, EntityGeometry, EntityId,
    Layer, LayerId, LengthMm, Line, Point2, Project, Rectangle, Text, DEFAULT_LAYER_ID,
};

#[must_use]
pub fn seed_demo_project() -> Project {
    let mut project = Project::default();

    let annotation_layer = LayerId::new(1);
    project
        .drawing
        .add_layer(Layer::new(annotation_layer, "annotations"))
        .expect("annotation layer id is unused");

    // A hidden layer, so the canvas visibly proves that hidden-layer
    // filtering (Phase 5) also applies inside the GUI.
    let hidden_layer = LayerId::new(2);
    let mut hidden = Layer::new(hidden_layer, "hidden-demo");
    hidden.visible = false;
    project
        .drawing
        .add_layer(hidden)
        .expect("hidden layer id is unused");

    let mut next_id = 1;
    let mut add = |project: &mut Project, layer_id: LayerId, geometry: EntityGeometry| {
        project
            .drawing
            .add_entity(Entity {
                id: EntityId::new(next_id),
                layer_id,
                geometry,
            })
            .expect("demo entity ids are unique");
        next_id += 1;
    };

    add(
        &mut project,
        DEFAULT_LAYER_ID,
        EntityGeometry::Line(Line {
            start: Point2::new(-40.0, -20.0),
            end: Point2::new(40.0, -20.0),
        }),
    );
    add(
        &mut project,
        DEFAULT_LAYER_ID,
        EntityGeometry::Rectangle(Rectangle {
            origin: Point2::new(-30.0, 0.0),
            width: LengthMm(60.0),
            height: LengthMm(30.0),
        }),
    );
    add(
        &mut project,
        DEFAULT_LAYER_ID,
        EntityGeometry::Circle(Circle {
            center: Point2::new(0.0, 50.0),
            radius: LengthMm(15.0),
        }),
    );
    add(
        &mut project,
        DEFAULT_LAYER_ID,
        EntityGeometry::Arc(Arc {
            center: Point2::new(60.0, 15.0),
            radius: LengthMm(15.0),
            start_angle: AngleRad(0.0),
            sweep_angle: AngleRad(std::f64::consts::PI),
        }),
    );
    add(
        &mut project,
        annotation_layer,
        EntityGeometry::Text(Text {
            origin: Point2::new(-30.0, -30.0),
            content: "CAD Studio".to_owned(),
            height: LengthMm(6.0),
        }),
    );
    add(
        &mut project,
        hidden_layer,
        EntityGeometry::Circle(Circle {
            center: Point2::new(100.0, 0.0),
            radius: LengthMm(10.0),
        }),
    );

    project
        .drawing
        .add_dimension(Dimension {
            id: DimensionId::new(1),
            kind: DimensionKind::Linear,
            start: Point2::new(-30.0, 0.0),
            end: Point2::new(30.0, 0.0),
            offset: LengthMm(-15.0),
            layer_id: annotation_layer,
        })
        .expect("demo dimension id is unique");

    project
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_demo_project_is_internally_valid() {
        let project = seed_demo_project();
        let report = project.drawing.validate();
        assert!(
            !report.has_errors(),
            "demo project should satisfy cad_core's own validation: {report:?}"
        );
    }

    #[test]
    fn seed_demo_project_includes_a_hidden_layer_entity() {
        let project = seed_demo_project();
        let hidden_layer = project
            .drawing
            .layers
            .iter()
            .find(|layer| !layer.visible)
            .expect("demo project should include a hidden layer");
        assert!(project
            .drawing
            .entities
            .iter()
            .any(|entity| entity.layer_id == hidden_layer.id));
    }

    #[test]
    fn seed_demo_project_covers_every_geometry_variant() {
        let project = seed_demo_project();
        let has = |predicate: fn(&EntityGeometry) -> bool| {
            project
                .drawing
                .entities
                .iter()
                .any(|e| predicate(&e.geometry))
        };
        assert!(has(|g| matches!(g, EntityGeometry::Line(_))));
        assert!(has(|g| matches!(g, EntityGeometry::Rectangle(_))));
        assert!(has(|g| matches!(g, EntityGeometry::Circle(_))));
        assert!(has(|g| matches!(g, EntityGeometry::Arc(_))));
        assert!(has(|g| matches!(g, EntityGeometry::Text(_))));
        assert_eq!(project.drawing.dimensions.len(), 1);
    }
}
