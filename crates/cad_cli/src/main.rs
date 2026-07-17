#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cad_core::Project;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "--help".to_owned());

    match run(&command, args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "cad_cli commands:\n\
         \x20 validate <path.cadproj>\n\
         \x20 export <path.cadproj> [output.svg]\n\
         \x20 batch-export <directory>\n\
         \x20 inspect <path.cadproj>\n\
         \x20 recover <path.cadproj>"
    );
}

fn run(command: &str, args: impl Iterator<Item = String>) -> Result<(), String> {
    match command {
        "--help" | "help" => {
            print_help();
            Ok(())
        }
        "validate" => run_validate(args),
        "export" => run_export(args),
        "batch-export" => run_batch_export(args),
        "inspect" => run_inspect(args),
        "recover" => run_recover(args),
        other => Err(format!("unknown command '{other}' (see --help)")),
    }
}

fn next_arg(args: &mut impl Iterator<Item = String>, usage: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("usage: cad_cli {usage}"))
}

fn run_validate(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let path = next_arg(&mut args, "validate <path.cadproj>")?;
    let project =
        cad_io::load_project(Path::new(&path)).map_err(|e| format!("INVALID: {path}: {e}"))?;
    println!(
        "OK: {path} is valid ({} entities, {} layers, {} dimensions)",
        project.drawing.entities.len(),
        project.drawing.layers.len(),
        project.drawing.dimensions.len()
    );
    Ok(())
}

fn run_export(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let path = next_arg(&mut args, "export <path.cadproj> [output.svg]")?;
    let out_path = args
        .next()
        .map_or_else(|| Path::new(&path).with_extension("svg"), PathBuf::from);

    let project = cad_io::load_project(Path::new(&path))
        .map_err(|e| format!("failed to load {path}: {e}"))?;
    cad_io::save_svg(&project, &out_path)
        .map_err(|e| format!("failed to write {}: {e}", out_path.display()))?;
    println!("exported {path} -> {}", out_path.display());
    Ok(())
}

fn run_batch_export(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let dir = next_arg(&mut args, "batch-export <directory>")?;
    let report = cad_batch::export_svg_dir(Path::new(&dir));
    println!(
        "batch-export: {} exported, {} failed",
        report.processed, report.failed
    );
    for failed_path in &report.failed_paths {
        eprintln!("  failed: {}", failed_path.display());
    }
    if report.failed > 0 {
        return Err(format!("{} file(s) failed to export", report.failed));
    }
    Ok(())
}

fn run_inspect(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let path = next_arg(&mut args, "inspect <path.cadproj>")?;
    let project = cad_io::load_project(Path::new(&path))
        .map_err(|e| format!("failed to load {path}: {e}"))?;

    println!("project id: {}", project.id);
    println!("drawing id: {}", project.drawing.id);
    println!("layers: {}", project.drawing.layers.len());
    for layer in &project.drawing.layers {
        println!(
            "  - {} (visible={}, locked={})",
            layer.name, layer.visible, layer.locked
        );
    }
    println!("entities: {}", project.drawing.entities.len());
    println!("dimensions: {}", project.drawing.dimensions.len());
    Ok(())
}

fn run_recover(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let path_str = next_arg(&mut args, "recover <path.cadproj>")?;
    let path = Path::new(&path_str);

    if cad_io::load_project(path).is_ok() {
        println!("{path_str} already loads without errors; no recovery needed.");
        return Ok(());
    }

    let autosave = cad_io::autosave_path(path);
    if let Ok(project) = cad_io::load_project(&autosave) {
        return finish_recovery(&project, path, &autosave);
    }

    match cad_io::latest_backup(path) {
        Some(backup_path) => match cad_io::load_project(&backup_path) {
            Ok(project) => finish_recovery(&project, path, &backup_path),
            Err(e) => Err(format!(
                "backup {} also failed to load: {e}",
                backup_path.display()
            )),
        },
        None => Err(format!(
            "no autosave or backup found for {path_str}; nothing to recover from"
        )),
    }
}

/// Writes the recovered project to `<name>.recovered.cadproj`. The original
/// file at `original` is never overwritten — recovery always produces a new
/// file so the user can compare and decide (`file_save_docs.md` 7.3:
/// "원본 파일을 직접 덮어쓰지 않는다").
fn finish_recovery(project: &Project, original: &Path, source: &Path) -> Result<(), String> {
    let recovered_path = original.with_extension("recovered.cadproj");
    cad_io::save_project(project, &recovered_path).map_err(|e| {
        format!(
            "recovered project from {} but failed to write {}: {e}",
            source.display(),
            recovered_path.display()
        )
    })?;
    println!(
        "recovered from {} -> wrote {} (original left untouched)",
        source.display(),
        recovered_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_core::{Entity, EntityGeometry, EntityId, Line, Point2, DEFAULT_LAYER_ID};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = TEST_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cad_studio_cli_test_{nanos}_{seq}"));
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

    fn args(values: &[&str]) -> impl Iterator<Item = String> {
        values
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn validate_succeeds_for_a_valid_project() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");
        cad_io::save_project(&sample_project(), &path).unwrap();

        let result = run_validate(args(&[path.to_str().unwrap()]));
        assert!(result.is_ok());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn validate_fails_for_a_missing_file() {
        let result = run_validate(args(&["this/does/not/exist.cadproj"]));
        assert!(result.is_err());
    }

    #[test]
    fn export_writes_an_svg_file() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");
        cad_io::save_project(&sample_project(), &path).unwrap();

        let result = run_export(args(&[path.to_str().unwrap()]));
        assert!(result.is_ok());
        assert!(dir.join("test.svg").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn inspect_succeeds_for_a_valid_project() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");
        cad_io::save_project(&sample_project(), &path).unwrap();

        let result = run_inspect(args(&[path.to_str().unwrap()]));
        assert!(result.is_ok());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn recover_reports_no_action_when_original_is_valid() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");
        cad_io::save_project(&sample_project(), &path).unwrap();

        let result = run_recover(args(&[path.to_str().unwrap()]));
        assert!(result.is_ok());
        assert!(!dir.join("test.recovered.cadproj").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn recover_falls_back_to_autosave_without_touching_the_original() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");
        cad_io::autosave_project(&sample_project(), &path).unwrap();

        let result = run_recover(args(&[path.to_str().unwrap()]));
        assert!(result.is_ok());
        assert!(!path.exists());
        assert!(dir.join("test.recovered.cadproj").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn recover_falls_back_to_latest_backup_when_autosave_is_missing() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");
        let project = sample_project();
        cad_io::save_project(&project, &path).unwrap();
        fs::remove_file(&path).unwrap();

        let result = run_recover(args(&[path.to_str().unwrap()]));
        assert!(result.is_ok());
        assert!(dir.join("test.recovered.cadproj").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn recover_fails_when_nothing_can_be_found() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");

        let result = run_recover(args(&[path.to_str().unwrap()]));
        assert!(result.is_err());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn batch_export_reports_success_for_a_directory_of_valid_projects() {
        let dir = temp_dir();
        cad_io::save_project(&sample_project(), &dir.join("a.cadproj")).unwrap();

        let result = run_batch_export(args(&[dir.to_str().unwrap()]));
        assert!(result.is_ok());
        assert!(dir.join("a.svg").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unknown_command_is_an_error() {
        let result = run("does-not-exist", args(&[]));
        assert!(result.is_err());
    }
}
