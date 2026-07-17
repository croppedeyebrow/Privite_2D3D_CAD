#![forbid(unsafe_code)]

use cad_core::{CoreError, Drawing, Entity, EntityId, Project};

#[derive(Clone, Debug, PartialEq)]
pub enum DrawingCommand {
    AddEntity(Entity),
    DeleteEntity { id: EntityId },
}

#[derive(Debug, PartialEq)]
pub enum CommandError {
    Core(CoreError),
}

impl From<CoreError> for CommandError {
    fn from(value: CoreError) -> Self {
        Self::Core(value)
    }
}

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
    /// Executes a command and records its inverse for undo.
    ///
    /// # Errors
    ///
    /// Returns the core-model error when validation or mutation fails.
    pub fn execute(
        &mut self,
        project: &mut Project,
        command: DrawingCommand,
    ) -> Result<(), CommandError> {
        let inverse = inverse(project, &command)?;
        apply(project, &command)?;
        self.undo.push(HistoryEntry { command, inverse });
        self.redo.clear();
        Ok(())
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

fn drawing(project: &mut Project) -> &mut Drawing {
    &mut project.drawing
}

fn apply(project: &mut Project, command: &DrawingCommand) -> Result<(), CommandError> {
    match command {
        DrawingCommand::AddEntity(entity) => drawing(project).add_entity(entity.clone())?,
        DrawingCommand::DeleteEntity { id } => {
            drawing(project).remove_entity(*id)?;
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_core::{EntityGeometry, LengthMm, Line, Point2, DEFAULT_LAYER_ID};

    #[test]
    fn add_entity_can_be_undone_and_redone() {
        let entity = Entity {
            id: EntityId::new(1),
            layer_id: DEFAULT_LAYER_ID,
            geometry: EntityGeometry::Line(Line {
                start: Point2 {
                    x: LengthMm(0.0),
                    y: LengthMm(0.0),
                },
                end: Point2 {
                    x: LengthMm(1.0),
                    y: LengthMm(1.0),
                },
            }),
        };
        let mut project = Project::default();
        let mut history = CommandHistory::default();
        history
            .execute(&mut project, DrawingCommand::AddEntity(entity))
            .expect("command succeeds");
        assert_eq!(project.drawing.entities.len(), 1);
        assert!(history.undo(&mut project).expect("undo succeeds"));
        assert!(project.drawing.entities.is_empty());
        assert!(history.redo(&mut project).expect("redo succeeds"));
        assert_eq!(project.drawing.entities.len(), 1);
    }
}
