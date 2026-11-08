#![forbid(unsafe_code)]

use cad_core::{
    Arc, Circle, CoreError, Dimension, DimensionId, Drawing, Entity, EntityGeometry, EntityId,
    Group, GroupId, Layer, LayerId, LengthMm, Line, Point2, Polyline, Project, Rectangle, Text,
    ValidationIssue, ValidationReport, ValidationSeverity, ValidationTarget,
};

#[derive(Clone, Debug, PartialEq)]
pub enum DrawingCommand {
    AddEntity(Entity),
    DeleteEntity {
        id: EntityId,
    },
    MoveEntity {
        id: EntityId,
        dx: LengthMm,
        dy: LengthMm,
    },
    /// Moves several entities by the same delta as one undo step — used
    /// when dragging a multi-selection (box select) on the canvas.
    MoveEntities {
        ids: Vec<EntityId>,
        dx: LengthMm,
        dy: LengthMm,
    },
    SetEntityLayer {
        id: EntityId,
        layer_id: LayerId,
    },
    /// Replaces an entity's geometry wholesale (property panel edits).
    SetEntityGeometry {
        id: EntityId,
        geometry: EntityGeometry,
    },
    AddLayer(Layer),
    RemoveLayer {
        id: LayerId,
    },
    SetLayerVisible {
        id: LayerId,
        visible: bool,
    },
    SetLayerLocked {
        id: LayerId,
        locked: bool,
    },
    AddDimension(Dimension),
    RemoveDimension {
        id: DimensionId,
    },
    /// Creates a group record from an already-built `Group` (id, name, and
    /// member entity IDs). The UI is responsible for only offering this on
    /// entities that aren't already grouped — see `tool.rs` in `cad_app`.
    GroupEntities(Group),
    UngroupEntities {
        group_id: GroupId,
    },
}

#[derive(Debug, PartialEq)]
pub enum CommandError {
    Core(CoreError),
    Validation(ValidationReport),
}

impl From<CoreError> for CommandError {
    fn from(value: CoreError) -> Self {
        Self::Core(value)
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core(e) => write!(f, "{e}"),
            Self::Validation(report) => {
                write!(f, "validation failed with {} issue(s)", report.issues.len())
            }
        }
    }
}

impl std::error::Error for CommandError {}

#[derive(Default)]
pub struct CommandHistory {
    undo: Vec<HistoryEntry>,
    redo: Vec<DrawingCommand>,
}

struct HistoryEntry {
    command: DrawingCommand,
    inverse: DrawingCommand,
}

impl CommandHistory {
    /// Executes a command, recording its inverse for undo.
    ///
    /// After applying the command, the resulting drawing is checked against
    /// `cad_core`'s structural rules and `cad_geometry`'s per-shape rules. If
    /// either check fails, the command is rolled back and the drawing is left
    /// exactly as it was before `execute` was called.
    ///
    /// # Errors
    ///
    /// Returns the core-model error when applying or rolling back fails, or
    /// `CommandError::Validation` when the resulting drawing is invalid.
    pub fn execute(
        &mut self,
        project: &mut Project,
        command: DrawingCommand,
    ) -> Result<(), CommandError> {
        let inverse = inverse(project, &command)?;
        apply(project, &command)?;

        if let Err(report) = validate_drawing(&project.drawing) {
            apply(project, &inverse)?;
            return Err(CommandError::Validation(report));
        }

        self.undo.push(HistoryEntry { command, inverse });
        self.redo.clear();
        Ok(())
    }

    /// Whether [`Self::undo`] would have an effect right now.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether [`Self::redo`] would have an effect right now.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Undoes the latest successful command.
    ///
    /// # Errors
    ///
    /// Returns the core-model error when the inverse cannot be applied.
    pub fn undo(&mut self, project: &mut Project) -> Result<bool, CommandError> {
        let Some(entry) = self.undo.pop() else {
            return Ok(false);
        };
        apply(project, &entry.inverse)?;
        self.redo.push(entry.command);
        Ok(true)
    }

    /// Reapplies the latest undone command.
    ///
    /// # Errors
    ///
    /// Returns the core-model error when the command cannot be reapplied.
    pub fn redo(&mut self, project: &mut Project) -> Result<bool, CommandError> {
        let Some(command) = self.redo.pop() else {
            return Ok(false);
        };
        let inverse = inverse(project, &command)?;
        apply(project, &command)?;
        self.undo.push(HistoryEntry { command, inverse });
        Ok(true)
    }
}

/// Rebuilds a project from scratch by replaying an ordered command log.
/// Used for deterministic recovery, e.g. reconstructing a drawing from an
/// autosave journal.
///
/// # Errors
///
/// Returns the error from the first command that fails to apply or validate.
pub fn replay(commands: &[DrawingCommand]) -> Result<Project, CommandError> {
    let mut project = Project::default();
    let mut history = CommandHistory::default();
    for command in commands {
        history.execute(&mut project, command.clone())?;
    }
    Ok(project)
}

fn drawing(project: &mut Project) -> &mut Drawing {
    &mut project.drawing
}

fn find_entity_mut(drawing: &mut Drawing, id: EntityId) -> Result<&mut Entity, CoreError> {
    drawing
        .entities
        .iter_mut()
        .find(|entity| entity.id == id)
        .ok_or(CoreError::MissingEntity(id))
}

fn find_layer_mut(drawing: &mut Drawing, id: LayerId) -> Result<&mut Layer, CoreError> {
    drawing
        .layers
        .iter_mut()
        .find(|layer| layer.id == id)
        .ok_or(CoreError::MissingLayer(id))
}

fn translate_geometry(geometry: &EntityGeometry, dx: LengthMm, dy: LengthMm) -> EntityGeometry {
    let shift = |p: Point2| cad_geometry::translate_point(p, dx, dy);
    match geometry {
        EntityGeometry::Line(line) => EntityGeometry::Line(Line {
            start: shift(line.start),
            end: shift(line.end),
        }),
        EntityGeometry::Polyline(polyline) => EntityGeometry::Polyline(Polyline {
            points: polyline.points.iter().copied().map(shift).collect(),
            closed: polyline.closed,
        }),
        EntityGeometry::Rectangle(rect) => EntityGeometry::Rectangle(Rectangle {
            origin: shift(rect.origin),
            width: rect.width,
            height: rect.height,
        }),
        EntityGeometry::Circle(circle) => EntityGeometry::Circle(Circle {
            center: shift(circle.center),
            radius: circle.radius,
        }),
        EntityGeometry::Arc(arc) => EntityGeometry::Arc(Arc {
            center: shift(arc.center),
            radius: arc.radius,
            start_angle: arc.start_angle,
            sweep_angle: arc.sweep_angle,
        }),
        EntityGeometry::Text(text) => EntityGeometry::Text(Text {
            origin: shift(text.origin),
            content: text.content.clone(),
            height: text.height,
        }),
    }
}

fn apply(project: &mut Project, command: &DrawingCommand) -> Result<(), CommandError> {
    match command {
        DrawingCommand::AddEntity(entity) => drawing(project).add_entity(entity.clone())?,
        DrawingCommand::DeleteEntity { id } => {
            let d = drawing(project);
            // Drop the entity from any group first, so it doesn't leave a
            // dangling member reference the post-apply validation would
            // reject. This is a one-way cleanup: undoing a delete restores
            // the entity but not its group membership.
            for group in &mut d.groups {
                group.member_ids.retain(|member_id| *member_id != *id);
            }
            d.remove_entity(*id)?;
        }
        DrawingCommand::MoveEntity { id, dx, dy } => {
            let entity = find_entity_mut(drawing(project), *id)?;
            entity.geometry = translate_geometry(&entity.geometry, *dx, *dy);
        }
        DrawingCommand::MoveEntities { ids, dx, dy } => {
            for id in ids {
                let entity = find_entity_mut(drawing(project), *id)?;
                entity.geometry = translate_geometry(&entity.geometry, *dx, *dy);
            }
        }
        DrawingCommand::SetEntityLayer { id, layer_id } => {
            find_entity_mut(drawing(project), *id)?.layer_id = *layer_id;
        }
        DrawingCommand::SetEntityGeometry { id, geometry } => {
            find_entity_mut(drawing(project), *id)?.geometry = geometry.clone();
        }
        DrawingCommand::AddLayer(layer) => drawing(project).add_layer(layer.clone())?,
        DrawingCommand::RemoveLayer { id } => {
            drawing(project).remove_layer(*id)?;
        }
        DrawingCommand::SetLayerVisible { id, visible } => {
            find_layer_mut(drawing(project), *id)?.visible = *visible;
        }
        DrawingCommand::SetLayerLocked { id, locked } => {
            find_layer_mut(drawing(project), *id)?.locked = *locked;
        }
        DrawingCommand::AddDimension(dim) => drawing(project).add_dimension(dim.clone())?,
        DrawingCommand::RemoveDimension { id } => {
            drawing(project).remove_dimension(*id)?;
        }
        DrawingCommand::GroupEntities(group) => drawing(project).add_group(group.clone())?,
        DrawingCommand::UngroupEntities { group_id } => {
            drawing(project).remove_group(*group_id)?;
        }
    }
    Ok(())
}

fn inverse(
    project: &mut Project,
    command: &DrawingCommand,
) -> Result<DrawingCommand, CommandError> {
    Ok(match command {
        DrawingCommand::AddEntity(entity) => DrawingCommand::DeleteEntity { id: entity.id },
        DrawingCommand::DeleteEntity { id } => DrawingCommand::AddEntity(
            project
                .drawing
                .entities
                .iter()
                .find(|entity| entity.id == *id)
                .cloned()
                .ok_or(CoreError::MissingEntity(*id))?,
        ),
        DrawingCommand::MoveEntity { id, dx, dy } => DrawingCommand::MoveEntity {
            id: *id,
            dx: LengthMm(-dx.0),
            dy: LengthMm(-dy.0),
        },
        DrawingCommand::MoveEntities { ids, dx, dy } => DrawingCommand::MoveEntities {
            ids: ids.clone(),
            dx: LengthMm(-dx.0),
            dy: LengthMm(-dy.0),
        },
        DrawingCommand::SetEntityLayer { id, .. } => {
            let current_layer_id = project
                .drawing
                .entities
                .iter()
                .find(|entity| entity.id == *id)
                .ok_or(CoreError::MissingEntity(*id))?
                .layer_id;
            DrawingCommand::SetEntityLayer {
                id: *id,
                layer_id: current_layer_id,
            }
        }
        DrawingCommand::SetEntityGeometry { id, .. } => {
            let current_geometry = project
                .drawing
                .entities
                .iter()
                .find(|entity| entity.id == *id)
                .ok_or(CoreError::MissingEntity(*id))?
                .geometry
                .clone();
            DrawingCommand::SetEntityGeometry {
                id: *id,
                geometry: current_geometry,
            }
        }
        DrawingCommand::AddLayer(layer) => DrawingCommand::RemoveLayer { id: layer.id },
        DrawingCommand::RemoveLayer { id } => DrawingCommand::AddLayer(
            project
                .drawing
                .layer(*id)
                .cloned()
                .ok_or(CoreError::MissingLayer(*id))?,
        ),
        DrawingCommand::SetLayerVisible { id, .. } => {
            let current = project
                .drawing
                .layer(*id)
                .ok_or(CoreError::MissingLayer(*id))?
                .visible;
            DrawingCommand::SetLayerVisible {
                id: *id,
                visible: current,
            }
        }
        DrawingCommand::SetLayerLocked { id, .. } => {
            let current = project
                .drawing
                .layer(*id)
                .ok_or(CoreError::MissingLayer(*id))?
                .locked;
            DrawingCommand::SetLayerLocked {
                id: *id,
                locked: current,
            }
        }
        DrawingCommand::AddDimension(dim) => DrawingCommand::RemoveDimension { id: dim.id },
        DrawingCommand::RemoveDimension { id } => DrawingCommand::AddDimension(
            project
                .drawing
                .dimensions
                .iter()
                .find(|dim| dim.id == *id)
                .cloned()
                .ok_or(CoreError::MissingDimension(*id))?,
        ),
        DrawingCommand::GroupEntities(group) => {
            DrawingCommand::UngroupEntities { group_id: group.id }
        }
        DrawingCommand::UngroupEntities { group_id } => DrawingCommand::GroupEntities(
            project
                .drawing
                .group(*group_id)
                .cloned()
                .ok_or(CoreError::MissingGroup(*group_id))?,
        ),
    })
}

/// Combines `cad_core`'s structural validation (dangling layer references)
/// with `cad_geometry`'s per-shape validation (zero-length lines, non-
/// positive radii, ...) into a single report.
fn validate_drawing(drawing: &Drawing) -> Result<(), ValidationReport> {
    let mut report = drawing.validate();

    for entity in &drawing.entities {
        let geometry_error = match &entity.geometry {
            EntityGeometry::Line(line) => cad_geometry::validate_line(line).err(),
            EntityGeometry::Polyline(polyline) => cad_geometry::validate_polyline(polyline).err(),
            EntityGeometry::Rectangle(rect) => cad_geometry::validate_rectangle(rect).err(),
            EntityGeometry::Circle(circle) => cad_geometry::validate_circle(circle).err(),
            EntityGeometry::Arc(arc) => cad_geometry::validate_arc(arc).err(),
            EntityGeometry::Text(_) => None,
        };
        if let Some(error) = geometry_error {
            report.issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                target: ValidationTarget::Entity(entity.id),
                message: error.to_string(),
                suggestion: "Adjust the entity geometry so it satisfies validation rules."
                    .to_owned(),
            });
        }
    }

    if report.has_errors() {
        Err(report)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_core::DEFAULT_LAYER_ID;

    fn line_entity(id: u64, start: (f64, f64), end: (f64, f64)) -> Entity {
        Entity {
            id: EntityId::new(id),
            layer_id: DEFAULT_LAYER_ID,
            geometry: EntityGeometry::Line(Line {
                start: Point2::new(start.0, start.1),
                end: Point2::new(end.0, end.1),
            }),
        }
    }

    #[test]
    fn add_entity_can_be_undone_and_redone() {
        let entity = line_entity(1, (0.0, 0.0), (1.0, 1.0));
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        assert!(!history.can_undo());
        assert!(!history.can_redo());

        history
            .execute(&mut project, DrawingCommand::AddEntity(entity))
            .expect("command succeeds");
        assert_eq!(project.drawing.entities.len(), 1);
        assert!(history.can_undo());
        assert!(!history.can_redo());

        assert!(history.undo(&mut project).expect("undo succeeds"));
        assert!(project.drawing.entities.is_empty());
        assert!(!history.can_undo());
        assert!(history.can_redo());

        assert!(history.redo(&mut project).expect("redo succeeds"));
        assert_eq!(project.drawing.entities.len(), 1);
        assert!(history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn move_entity_translates_geometry_and_can_be_undone() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        history
            .execute(
                &mut project,
                DrawingCommand::AddEntity(line_entity(1, (0.0, 0.0), (10.0, 0.0))),
            )
            .unwrap();

        history
            .execute(
                &mut project,
                DrawingCommand::MoveEntity {
                    id: EntityId::new(1),
                    dx: LengthMm(5.0),
                    dy: LengthMm(2.0),
                },
            )
            .unwrap();

        let moved = &project.drawing.entities[0];
        assert_eq!(
            moved.geometry,
            EntityGeometry::Line(Line {
                start: Point2::new(5.0, 2.0),
                end: Point2::new(15.0, 2.0),
            })
        );

        assert!(history.undo(&mut project).unwrap());
        let restored = &project.drawing.entities[0];
        assert_eq!(
            restored.geometry,
            EntityGeometry::Line(Line {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(10.0, 0.0),
            })
        );
    }

    #[test]
    fn set_entity_layer_changes_layer_and_can_be_undone() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        let dimensions_layer = LayerId::new(1);
        history
            .execute(
                &mut project,
                DrawingCommand::AddLayer(Layer::new(dimensions_layer, "dimensions")),
            )
            .unwrap();
        history
            .execute(
                &mut project,
                DrawingCommand::AddEntity(line_entity(1, (0.0, 0.0), (1.0, 0.0))),
            )
            .unwrap();

        history
            .execute(
                &mut project,
                DrawingCommand::SetEntityLayer {
                    id: EntityId::new(1),
                    layer_id: dimensions_layer,
                },
            )
            .unwrap();
        assert_eq!(project.drawing.entities[0].layer_id, dimensions_layer);

        assert!(history.undo(&mut project).unwrap());
        assert_eq!(project.drawing.entities[0].layer_id, DEFAULT_LAYER_ID);
    }

    #[test]
    fn set_entity_geometry_replaces_geometry_and_can_be_undone() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        history
            .execute(
                &mut project,
                DrawingCommand::AddEntity(line_entity(1, (0.0, 0.0), (10.0, 0.0))),
            )
            .unwrap();

        let new_geometry = EntityGeometry::Line(Line {
            start: Point2::new(0.0, 0.0),
            end: Point2::new(99.0, 0.0),
        });
        history
            .execute(
                &mut project,
                DrawingCommand::SetEntityGeometry {
                    id: EntityId::new(1),
                    geometry: new_geometry.clone(),
                },
            )
            .unwrap();
        assert_eq!(project.drawing.entities[0].geometry, new_geometry);

        assert!(history.undo(&mut project).unwrap());
        assert_eq!(
            project.drawing.entities[0].geometry,
            EntityGeometry::Line(Line {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(10.0, 0.0),
            })
        );
    }

    #[test]
    fn set_entity_geometry_rejects_invalid_result_and_rolls_back() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        history
            .execute(
                &mut project,
                DrawingCommand::AddEntity(line_entity(1, (0.0, 0.0), (10.0, 0.0))),
            )
            .unwrap();

        let degenerate = EntityGeometry::Line(Line {
            start: Point2::new(5.0, 5.0),
            end: Point2::new(5.0, 5.0),
        });
        let result = history.execute(
            &mut project,
            DrawingCommand::SetEntityGeometry {
                id: EntityId::new(1),
                geometry: degenerate,
            },
        );
        assert!(matches!(result, Err(CommandError::Validation(_))));
        assert_eq!(
            project.drawing.entities[0].geometry,
            EntityGeometry::Line(Line {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(10.0, 0.0),
            })
        );
    }

    #[test]
    fn set_layer_visible_and_locked_can_be_undone() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();

        history
            .execute(
                &mut project,
                DrawingCommand::SetLayerVisible {
                    id: DEFAULT_LAYER_ID,
                    visible: false,
                },
            )
            .unwrap();
        assert!(!project.drawing.layer(DEFAULT_LAYER_ID).unwrap().visible);

        history
            .execute(
                &mut project,
                DrawingCommand::SetLayerLocked {
                    id: DEFAULT_LAYER_ID,
                    locked: true,
                },
            )
            .unwrap();
        assert!(project.drawing.layer(DEFAULT_LAYER_ID).unwrap().locked);

        assert!(history.undo(&mut project).unwrap());
        assert!(!project.drawing.layer(DEFAULT_LAYER_ID).unwrap().locked);

        assert!(history.undo(&mut project).unwrap());
        assert!(project.drawing.layer(DEFAULT_LAYER_ID).unwrap().visible);
    }

    #[test]
    fn move_entities_translates_all_members_as_one_undo_step() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        history
            .execute(
                &mut project,
                DrawingCommand::AddEntity(line_entity(1, (0.0, 0.0), (10.0, 0.0))),
            )
            .unwrap();
        history
            .execute(
                &mut project,
                DrawingCommand::AddEntity(line_entity(2, (0.0, 5.0), (10.0, 5.0))),
            )
            .unwrap();

        history
            .execute(
                &mut project,
                DrawingCommand::MoveEntities {
                    ids: vec![EntityId::new(1), EntityId::new(2)],
                    dx: LengthMm(3.0),
                    dy: LengthMm(1.0),
                },
            )
            .unwrap();
        assert_eq!(
            project.drawing.entities[0].geometry,
            EntityGeometry::Line(Line {
                start: Point2::new(3.0, 1.0),
                end: Point2::new(13.0, 1.0),
            })
        );
        assert_eq!(
            project.drawing.entities[1].geometry,
            EntityGeometry::Line(Line {
                start: Point2::new(3.0, 6.0),
                end: Point2::new(13.0, 6.0),
            })
        );

        // One undo restores both entities, not just one.
        assert!(history.undo(&mut project).unwrap());
        assert_eq!(
            project.drawing.entities[0].geometry,
            EntityGeometry::Line(Line {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(10.0, 0.0),
            })
        );
        assert_eq!(
            project.drawing.entities[1].geometry,
            EntityGeometry::Line(Line {
                start: Point2::new(0.0, 5.0),
                end: Point2::new(10.0, 5.0),
            })
        );
    }

    #[test]
    fn group_and_ungroup_entities_can_be_undone_and_redone() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        history
            .execute(
                &mut project,
                DrawingCommand::AddEntity(line_entity(1, (0.0, 0.0), (1.0, 0.0))),
            )
            .unwrap();
        history
            .execute(
                &mut project,
                DrawingCommand::AddEntity(line_entity(2, (0.0, 1.0), (1.0, 1.0))),
            )
            .unwrap();

        let group = Group {
            id: GroupId::new(1),
            name: "part-a".to_owned(),
            member_ids: vec![EntityId::new(1), EntityId::new(2)],
        };
        history
            .execute(&mut project, DrawingCommand::GroupEntities(group.clone()))
            .unwrap();
        assert_eq!(project.drawing.group(GroupId::new(1)), Some(&group));

        history
            .execute(
                &mut project,
                DrawingCommand::UngroupEntities {
                    group_id: GroupId::new(1),
                },
            )
            .unwrap();
        assert!(project.drawing.group(GroupId::new(1)).is_none());

        // Undoing the ungroup recreates the exact same group.
        assert!(history.undo(&mut project).unwrap());
        assert_eq!(project.drawing.group(GroupId::new(1)), Some(&group));

        // Undoing the group creation removes it again.
        assert!(history.undo(&mut project).unwrap());
        assert!(project.drawing.group(GroupId::new(1)).is_none());
    }

    #[test]
    fn deleting_a_grouped_entity_removes_it_from_the_group() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        history
            .execute(
                &mut project,
                DrawingCommand::AddEntity(line_entity(1, (0.0, 0.0), (1.0, 0.0))),
            )
            .unwrap();
        history
            .execute(
                &mut project,
                DrawingCommand::AddEntity(line_entity(2, (0.0, 1.0), (1.0, 1.0))),
            )
            .unwrap();
        history
            .execute(
                &mut project,
                DrawingCommand::GroupEntities(Group {
                    id: GroupId::new(1),
                    name: "part-a".to_owned(),
                    member_ids: vec![EntityId::new(1), EntityId::new(2)],
                }),
            )
            .unwrap();

        history
            .execute(
                &mut project,
                DrawingCommand::DeleteEntity {
                    id: EntityId::new(1),
                },
            )
            .unwrap();

        assert_eq!(
            project.drawing.group(GroupId::new(1)).unwrap().member_ids,
            vec![EntityId::new(2)]
        );
    }

    #[test]
    fn add_and_remove_layer_can_be_undone_and_redone() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        let id = LayerId::new(1);

        history
            .execute(
                &mut project,
                DrawingCommand::AddLayer(Layer::new(id, "dim")),
            )
            .unwrap();
        assert!(project.drawing.layer(id).is_some());

        history
            .execute(&mut project, DrawingCommand::RemoveLayer { id })
            .unwrap();
        assert!(project.drawing.layer(id).is_none());

        assert!(history.undo(&mut project).unwrap());
        assert!(project.drawing.layer(id).is_some());

        assert!(history.redo(&mut project).unwrap());
        assert!(project.drawing.layer(id).is_none());
    }

    #[test]
    fn add_and_remove_dimension_can_be_undone() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        let dim = Dimension {
            id: DimensionId::new(1),
            kind: cad_core::DimensionKind::Linear,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(100.0, 0.0),
            offset: LengthMm(10.0),
            layer_id: DEFAULT_LAYER_ID,
        };

        history
            .execute(&mut project, DrawingCommand::AddDimension(dim.clone()))
            .unwrap();
        assert_eq!(project.drawing.dimensions.len(), 1);

        history
            .execute(&mut project, DrawingCommand::RemoveDimension { id: dim.id })
            .unwrap();
        assert!(project.drawing.dimensions.is_empty());

        assert!(history.undo(&mut project).unwrap());
        assert_eq!(project.drawing.dimensions.len(), 1);
    }

    #[test]
    fn execute_rejects_entity_with_missing_layer_and_rolls_back() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        let entity = Entity {
            id: EntityId::new(1),
            layer_id: LayerId::new(99),
            geometry: EntityGeometry::Line(Line {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(1.0, 0.0),
            }),
        };

        let result = history.execute(&mut project, DrawingCommand::AddEntity(entity));
        assert!(matches!(result, Err(CommandError::Validation(_))));
        assert!(project.drawing.entities.is_empty());
    }

    #[test]
    fn execute_rejects_zero_length_line_and_rolls_back() {
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        let entity = line_entity(1, (5.0, 5.0), (5.0, 5.0));

        let result = history.execute(&mut project, DrawingCommand::AddEntity(entity));
        assert!(matches!(result, Err(CommandError::Validation(_))));
        assert!(project.drawing.entities.is_empty());
    }

    #[test]
    fn replay_rebuilds_project_deterministically() {
        let commands = vec![
            DrawingCommand::AddEntity(line_entity(1, (0.0, 0.0), (10.0, 0.0))),
            DrawingCommand::MoveEntity {
                id: EntityId::new(1),
                dx: LengthMm(1.0),
                dy: LengthMm(1.0),
            },
            DrawingCommand::AddEntity(line_entity(2, (0.0, 0.0), (5.0, 5.0))),
        ];

        let project = replay(&commands).expect("replay succeeds");
        assert_eq!(project.drawing.entities.len(), 2);
        assert_eq!(
            project.drawing.entities[0].geometry,
            EntityGeometry::Line(Line {
                start: Point2::new(1.0, 1.0),
                end: Point2::new(11.0, 1.0),
            })
        );
    }
}
