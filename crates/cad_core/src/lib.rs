#![forbid(unsafe_code)]

use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EntityId(u64);

impl EntityId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LengthMm(pub f64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point2 {
    pub x: LengthMm,
    pub y: LengthMm,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub start: Point2,
    pub end: Point2,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EntityGeometry {
    Line(Line),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub id: EntityId,
    pub geometry: EntityGeometry,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawing {
    pub entities: Vec<Entity>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Project {
    pub drawing: Drawing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreError {
    DuplicateEntityId(EntityId),
    MissingEntity(EntityId),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateEntityId(id) => write!(f, "duplicate entity id: {}", id.value()),
            Self::MissingEntity(id) => write!(f, "missing entity id: {}", id.value()),
        }
    }
}

impl std::error::Error for CoreError {}

impl Drawing {
    /// Adds an entity while preserving stable-ID uniqueness.
    ///
    /// # Errors
    ///
    /// Returns `DuplicateEntityId` when the drawing already contains the entity ID.
    pub fn add_entity(&mut self, entity: Entity) -> Result<(), CoreError> {
        if self.entities.iter().any(|item| item.id == entity.id) {
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
            .position(|item| item.id == id)
            .ok_or(CoreError::MissingEntity(id))?;
        Ok(self.entities.remove(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_ids_prevent_duplicate_entities() {
        let line = Line {
            start: Point2 {
                x: LengthMm(0.0),
                y: LengthMm(0.0),
            },
            end: Point2 {
                x: LengthMm(10.0),
                y: LengthMm(0.0),
            },
        };
        let entity = Entity {
            id: EntityId::new(1),
            geometry: EntityGeometry::Line(line),
        };
        let mut drawing = Drawing::default();
        drawing
            .add_entity(entity.clone())
            .expect("first entity is valid");
        assert_eq!(
            drawing.add_entity(entity),
            Err(CoreError::DuplicateEntityId(EntityId::new(1)))
        );
    }
}
