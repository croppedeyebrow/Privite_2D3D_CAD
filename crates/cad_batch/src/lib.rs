#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub struct BatchReport {
    pub processed: usize,
    pub failed: usize,
    pub failed_paths: Vec<PathBuf>,
}

#[must_use]
pub fn empty_report() -> BatchReport {
    BatchReport {
        processed: 0,
        failed: 0,
        failed_paths: Vec::new(),
    }
}

fn is_project_file(name: &str) -> bool {
    name.ends_with(".cadproj") && !name.ends_with(".autosave.cadproj")
}

/// Loads every project file directly inside `dir` (autosave companions are
/// skipped) and exports each to an SVG file with the same stem. Returns a
/// report of how many succeeded and which ones failed; a directory that
/// can't be read at all yields an empty report rather than an error, since
/// "nothing to process" and "found nothing" are not failures in themselves.
#[must_use]
pub fn export_svg_dir(dir: &Path) -> BatchReport {
    let mut report = empty_report();

    let Ok(entries) = fs::read_dir(dir) else {
        return report;
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_project_file(name) {
            continue;
        }

        let exported = cad_io::load_project(&path)
            .ok()
            .map(|project| cad_io::save_svg(&project, &path.with_extension("svg")))
            .is_some_and(|result| result.is_ok());

        if exported {
            report.processed += 1;
        } else {
            report.failed += 1;
            report.failed_paths.push(path);
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_core::{Entity, EntityGeometry, EntityId, Line, Point2, Project, DEFAULT_LAYER_ID};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = TEST_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cad_studio_batch_test_{nanos}_{seq}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_project() -> Project {
        let mut project = Project::default();
        project
            .drawing
            .add_entity(Entity {
                id: EntityId::new(1),
                layer_id: DEFAULT_LAYER_ID,
                geometry: EntityGeometry::Line(Line {
                    start: Point2::new(0.0, 0.0),
                    end: Point2::new(10.0, 0.0),
                }),
            })
            .unwrap();
        project
    }

    #[test]
    fn empty_report_starts_at_zero() {
        let report = empty_report();
        assert_eq!(report.processed, 0);
        assert_eq!(report.failed, 0);
        assert!(report.failed_paths.is_empty());
    }

    #[test]
    fn export_svg_dir_processes_valid_projects_and_skips_autosave() {
        let dir = temp_dir();
        let project = sample_project();
        cad_io::save_project(&project, &dir.join("a.cadproj")).unwrap();
        cad_io::save_project(&project, &dir.join("b.cadproj")).unwrap();
        cad_io::autosave_project(&project, &dir.join("a.cadproj")).unwrap();

        let report = export_svg_dir(&dir);
        assert_eq!(report.processed, 2);
        assert_eq!(report.failed, 0);
        assert!(dir.join("a.svg").exists());
        assert!(dir.join("b.svg").exists());
        assert!(!dir.join("a.autosave.svg").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_svg_dir_counts_unreadable_files_as_failures() {
        let dir = temp_dir();
        fs::write(dir.join("broken.cadproj"), "not valid json").unwrap();

        let report = export_svg_dir(&dir);
        assert_eq!(report.processed, 0);
        assert_eq!(report.failed, 1);
        assert_eq!(report.failed_paths, vec![dir.join("broken.cadproj")]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_svg_dir_on_missing_directory_is_an_empty_report() {
        let report = export_svg_dir(Path::new("this/directory/does/not/exist"));
        assert_eq!(report, empty_report());
    }
}
