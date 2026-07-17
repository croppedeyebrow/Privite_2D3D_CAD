#![forbid(unsafe_code)]

use std::fmt;

// ---------------------------------------------------------------------------
// Stable identifiers
// ---------------------------------------------------------------------------

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub struct $name(u64);

        impl $name {
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn value(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

stable_id!(ProjectId);
stable_id!(DrawingId);
stable_id!(EntityId);
stable_id!(LayerId);
stable_id!(DimensionId);
stable_id!(SheetId);

// ---------------------------------------------------------------------------
// Units
// ---------------------------------------------------------------------------

/// Internal length unit. All calculations use millimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LengthMm(pub f64);

/// Angle in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AngleRad(pub f64);

// ---------------------------------------------------------------------------
// Geometry primitives
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point2 {
    pub x: LengthMm,
    pub y: LengthMm,
}

impl Point2 {
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self {
            x: LengthMm(x),
            y: LengthMm(y),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub start: Point2,
    pub end: Point2,
}

/// Open or closed sequence of connected line segments.
#[derive(Clone, Debug, PartialEq)]
pub struct Polyline {
    pub points: Vec<Point2>,
    pub closed: bool,
}

/// Axis-aligned rectangle defined by origin corner, width, and height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    /// Lower-left corner in drawing space.
    pub origin: Point2,
    pub width: LengthMm,
    pub height: LengthMm,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub center: Point2,
    pub radius: LengthMm,
}

/// Arc defined by centre, radius, and counter-clockwise angle range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arc {
    pub center: Point2,
    pub radius: LengthMm,
    /// Start angle measured CCW from the positive X axis.
    pub start_angle: AngleRad,
    /// Sweep angle (positive = CCW). Must not be zero.
    pub sweep_angle: AngleRad,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub origin: Point2,
    pub content: String,
    /// Text height in mm.
    pub height: LengthMm,
}

// ---------------------------------------------------------------------------
// Entity
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum EntityGeometry {
    Line(Line),
    Polyline(Polyline),
    Rectangle(Rectangle),
    Circle(Circle),
    Arc(Arc),
    Text(Text),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub id: EntityId,
    pub layer_id: LayerId,
    pub geometry: EntityGeometry,
}

// ---------------------------------------------------------------------------
// Layer
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
}

impl Layer {
    #[must_use]
    pub fn new(id: LayerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            locked: false,
        }
    }
}

/// The default layer that always exists in a new drawing.
pub const DEFAULT_LAYER_ID: LayerId = LayerId::new(0);

// ---------------------------------------------------------------------------
// Dimension
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DimensionKind {
    /// Horizontal or vertical linear dimension.
    Linear,
    /// Dimension along the true distance between two points.
    Aligned,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dimension {
    pub id: DimensionId,
    pub kind: DimensionKind,
    pub start: Point2,
    pub end: Point2,
    /// Perpendicular offset of the dimension line from the measured segment (mm).
    pub offset: LengthMm,
    pub layer_id: LayerId,
}

// ---------------------------------------------------------------------------
// Drawing and Project
// ---------------------------------------------------------------------------

/// The first drawing created inside a new project.
pub const DEFAULT_DRAWING_ID: DrawingId = DrawingId::new(0);

/// The first project created by the application.
pub const DEFAULT_PROJECT_ID: ProjectId = ProjectId::new(0);

#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    pub id: DrawingId,
    pub entities: Vec<Entity>,
    pub layers: Vec<Layer>,
    pub dimensions: Vec<Dimension>,
}

impl Default for Drawing {
    fn default() -> Self {
        Self {
            id: DEFAULT_DRAWING_ID,
            entities: Vec::new(),
            layers: vec![Layer::new(DEFAULT_LAYER_ID, "0")],
            dimensions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub id: ProjectId,
    pub drawing: Drawing,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            id: DEFAULT_PROJECT_ID,
            drawing: Drawing::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreError {
    DuplicateEntityId(EntityId),
    MissingEntity(EntityId),
    DuplicateLayerId(LayerId),
    MissingLayer(LayerId),
    DuplicateDimensionId(DimensionId),
    MissingDimension(DimensionId),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateEntityId(id) => write!(f, "duplicate entity id: {id}"),
            Self::MissingEntity(id) => write!(f, "missing entity id: {id}"),
            Self::DuplicateLayerId(id) => write!(f, "duplicate layer id: {id}"),
            Self::MissingLayer(id) => write!(f, "missing layer id: {id}"),
            Self::DuplicateDimensionId(id) => write!(f, "duplicate dimension id: {id}"),
            Self::MissingDimension(id) => write!(f, "missing dimension id: {id}"),
        }
    }
}

impl std::error::Error for CoreError {}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Distinguishes problems that block save/export/command execution from
/// problems the user must acknowledge but can still work around.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

/// The persistent object a `ValidationIssue` applies to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationTarget {
    Project(ProjectId),
    Drawing(DrawingId),
    Entity(EntityId),
    Layer(LayerId),
    Dimension(DimensionId),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub target: ValidationTarget,
    pub message: String,
    pub suggestion: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Error)
    }
}

// ---------------------------------------------------------------------------
// Drawing operations
// ---------------------------------------------------------------------------

impl Drawing {
    /// Adds an entity while preserving stable-ID uniqueness.
    ///
    /// # Errors
    ///
    /// Returns `DuplicateEntityId` when the drawing already contains the entity ID.
    pub fn add_entity(&mut self, entity: Entity) -> Result<(), CoreError> {
        if self.entities.iter().any(|e| e.id == entity.id) {
            return Err(CoreError::DuplicateEntityId(entity.id));
        }
        self.entities.push(entity);
        Ok(())
    }

    /// Removes and returns an entity by stable ID.
    ///
    /// # Errors
    ///
    /// Returns `MissingEntity` when no entity has the requested ID.
    pub fn remove_entity(&mut self, id: EntityId) -> Result<Entity, CoreError> {
        let index = self
            .entities
            .iter()
            .position(|e| e.id == id)
            .ok_or(CoreError::MissingEntity(id))?;
        Ok(self.entities.remove(index))
    }

    /// Adds a layer while preserving stable-ID uniqueness.
    ///
    /// # Errors
    ///
    /// Returns `DuplicateLayerId` when the layer ID is already present.
    pub fn add_layer(&mut self, layer: Layer) -> Result<(), CoreError> {
        if self.layers.iter().any(|l| l.id == layer.id) {
            return Err(CoreError::DuplicateLayerId(layer.id));
        }
        self.layers.push(layer);
        Ok(())
    }

    /// Returns a shared reference to the layer with the given ID.
    #[must_use]
    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// Removes and returns a layer by stable ID.
    ///
    /// # Errors
    ///
    /// Returns `MissingLayer` when no layer has the requested ID.
    pub fn remove_layer(&mut self, id: LayerId) -> Result<Layer, CoreError> {
        let index = self
            .layers
            .iter()
            .position(|l| l.id == id)
            .ok_or(CoreError::MissingLayer(id))?;
        Ok(self.layers.remove(index))
    }

    /// Adds a dimension while preserving stable-ID uniqueness.
    ///
    /// # Errors
    ///
    /// Returns `DuplicateDimensionId` when the dimension ID is already present.
    pub fn add_dimension(&mut self, dim: Dimension) -> Result<(), CoreError> {
        if self.dimensions.iter().any(|d| d.id == dim.id) {
            return Err(CoreError::DuplicateDimensionId(dim.id));
        }
        self.dimensions.push(dim);
        Ok(())
    }

    /// Removes and returns a dimension by stable ID.
    ///
    /// # Errors
    ///
    /// Returns `MissingDimension` when no dimension has the requested ID.
    pub fn remove_dimension(&mut self, id: DimensionId) -> Result<Dimension, CoreError> {
        let index = self
            .dimensions
            .iter()
            .position(|d| d.id == id)
            .ok_or(CoreError::MissingDimension(id))?;
        Ok(self.dimensions.remove(index))
    }

    /// Checks structural invariants that saving, exporting, or running a
    /// command must be able to rely on: every entity and dimension must
    /// reference a layer that exists in this drawing.
    #[must_use]
    pub fn validate(&self) -> ValidationReport {
        let mut issues = Vec::new();

        for entity in &self.entities {
            if self.layer(entity.layer_id).is_none() {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    target: ValidationTarget::Entity(entity.id),
                    message: format!("entity references missing layer {}", entity.layer_id),
                    suggestion:
                        "Create the missing layer or reassign the entity to an existing layer."
                            .to_owned(),
                });
            }
        }

        for dim in &self.dimensions {
            if self.layer(dim.layer_id).is_none() {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    target: ValidationTarget::Dimension(dim.id),
                    message: format!("dimension references missing layer {}", dim.layer_id),
                    suggestion:
                        "Create the missing layer or reassign the dimension to an existing layer."
                            .to_owned(),
                });
            }
        }

        ValidationReport { issues }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_line_entity(id: u64) -> Entity {
        Entity {
            id: EntityId::new(id),
            layer_id: DEFAULT_LAYER_ID,
            geometry: EntityGeometry::Line(Line {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(10.0, 0.0),
            }),
        }
    }

    #[test]
    fn stable_ids_prevent_duplicate_entities() {
        let mut drawing = Drawing::default();
        drawing
            .add_entity(make_line_entity(1))
            .expect("first insert ok");
        assert_eq!(
            drawing.add_entity(make_line_entity(1)),
            Err(CoreError::DuplicateEntityId(EntityId::new(1)))
        );
    }

    #[test]
    fn remove_entity_returns_the_entity() {
        let mut drawing = Drawing::default();
        drawing.add_entity(make_line_entity(2)).unwrap();
        let removed = drawing.remove_entity(EntityId::new(2)).unwrap();
        assert_eq!(removed.id, EntityId::new(2));
        assert!(drawing.entities.is_empty());
    }

    #[test]
    fn drawing_default_contains_layer_zero() {
        let drawing = Drawing::default();
        assert!(drawing.layer(DEFAULT_LAYER_ID).is_some());
        assert_eq!(drawing.layers.len(), 1);
    }

    #[test]
    fn duplicate_layer_id_is_rejected() {
        let mut drawing = Drawing::default();
        let result = drawing.add_layer(Layer::new(DEFAULT_LAYER_ID, "duplicate"));
        assert_eq!(result, Err(CoreError::DuplicateLayerId(DEFAULT_LAYER_ID)));
    }

    #[test]
    fn remove_layer_returns_the_layer() {
        let mut drawing = Drawing::default();
        let extra = LayerId::new(1);
        drawing.add_layer(Layer::new(extra, "dimensions")).unwrap();
        let removed = drawing.remove_layer(extra).unwrap();
        assert_eq!(removed.id, extra);
        assert!(drawing.layer(extra).is_none());
    }

    #[test]
    fn remove_layer_reports_missing_layer() {
        let mut drawing = Drawing::default();
        let missing = LayerId::new(99);
        assert_eq!(
            drawing.remove_layer(missing),
            Err(CoreError::MissingLayer(missing))
        );
    }

    #[test]
    fn add_and_remove_dimension() {
        let mut drawing = Drawing::default();
        let dim = Dimension {
            id: DimensionId::new(1),
            kind: DimensionKind::Linear,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(100.0, 0.0),
            offset: LengthMm(10.0),
            layer_id: DEFAULT_LAYER_ID,
        };
        drawing.add_dimension(dim).unwrap();
        assert_eq!(drawing.dimensions.len(), 1);
        let removed = drawing.remove_dimension(DimensionId::new(1)).unwrap();
        assert_eq!(removed.id, DimensionId::new(1));
        assert!(drawing.dimensions.is_empty());
    }

    #[test]
    fn all_geometry_variants_can_be_added() {
        let mut drawing = Drawing::default();
        let layer = DEFAULT_LAYER_ID;

        let entities: Vec<Entity> = vec![
            Entity {
                id: EntityId::new(10),
                layer_id: layer,
                geometry: EntityGeometry::Polyline(Polyline {
                    points: vec![Point2::new(0.0, 0.0), Point2::new(5.0, 5.0)],
                    closed: false,
                }),
            },
            Entity {
                id: EntityId::new(11),
                layer_id: layer,
                geometry: EntityGeometry::Rectangle(Rectangle {
                    origin: Point2::new(0.0, 0.0),
                    width: LengthMm(100.0),
                    height: LengthMm(50.0),
                }),
            },
            Entity {
                id: EntityId::new(12),
                layer_id: layer,
                geometry: EntityGeometry::Circle(Circle {
                    center: Point2::new(0.0, 0.0),
                    radius: LengthMm(25.0),
                }),
            },
            Entity {
                id: EntityId::new(13),
                layer_id: layer,
                geometry: EntityGeometry::Arc(Arc {
                    center: Point2::new(0.0, 0.0),
                    radius: LengthMm(25.0),
                    start_angle: AngleRad(0.0),
                    sweep_angle: AngleRad(std::f64::consts::FRAC_PI_2),
                }),
            },
            Entity {
                id: EntityId::new(14),
                layer_id: layer,
                geometry: EntityGeometry::Text(Text {
                    origin: Point2::new(10.0, 10.0),
                    content: "CAD Studio".to_owned(),
                    height: LengthMm(5.0),
                }),
            },
        ];

        for e in entities {
            drawing.add_entity(e).expect("entity should be added");
        }
        assert_eq!(drawing.entities.len(), 5);
    }

    #[test]
    fn default_project_and_drawing_use_id_zero() {
        let project = Project::default();
        assert_eq!(project.id, DEFAULT_PROJECT_ID);
        assert_eq!(project.drawing.id, DEFAULT_DRAWING_ID);
    }

    #[test]
    fn validate_reports_no_issues_for_a_default_drawing() {
        let drawing = Drawing::default();
        let report = drawing.validate();
        assert!(!report.has_errors());
        assert!(report.issues.is_empty());
    }

    #[test]
    fn validate_detects_entity_referencing_missing_layer() {
        let mut drawing = Drawing::default();
        let missing_layer = LayerId::new(99);
        drawing
            .add_entity(Entity {
                id: EntityId::new(1),
                layer_id: missing_layer,
                geometry: EntityGeometry::Line(Line {
                    start: Point2::new(0.0, 0.0),
                    end: Point2::new(1.0, 1.0),
                }),
            })
            .unwrap();

        let report = drawing.validate();
        assert!(report.has_errors());
        assert_eq!(report.issues.len(), 1);
        assert_eq!(
            report.issues[0].target,
            ValidationTarget::Entity(EntityId::new(1))
        );
        assert_eq!(report.issues[0].severity, ValidationSeverity::Error);
    }

    #[test]
    fn validate_detects_dimension_referencing_missing_layer() {
        let mut drawing = Drawing::default();
        let missing_layer = LayerId::new(99);
        drawing
            .add_dimension(Dimension {
                id: DimensionId::new(1),
                kind: DimensionKind::Linear,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(100.0, 0.0),
                offset: LengthMm(10.0),
                layer_id: missing_layer,
            })
            .unwrap();

        let report = drawing.validate();
        assert!(report.has_errors());
        assert_eq!(
            report.issues[0].target,
            ValidationTarget::Dimension(DimensionId::new(1))
        );
    }
}
