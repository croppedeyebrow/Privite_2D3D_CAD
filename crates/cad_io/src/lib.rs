#![forbid(unsafe_code)]

use cad_core::Project;
use std::path::Path;

#[derive(Debug)]
pub enum IoError {
    UnsupportedFormat,
    NotImplemented,
}

/// Saves a project using the future transactional project-file backend.
///
/// # Errors
///
/// Returns `NotImplemented` until the approved persistence backend is added.
pub fn save_project(_project: &Project, _path: &Path) -> Result<(), IoError> {
    Err(IoError::NotImplemented)
}

/// Loads a project using the future versioned project-file backend.
///
/// # Errors
///
/// Returns `NotImplemented` until the approved persistence backend is added.
pub fn load_project(_path: &Path) -> Result<Project, IoError> {
    Err(IoError::NotImplemented)
}
