#![forbid(unsafe_code)]

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use cad_core::{Project, ValidationReport};
use cad_render::RenderPrimitive;
use serde::{Deserialize, Serialize};

/// The only schema version this build knows how to read and write.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// The only unit this build's schema supports. Stored in the file so a
/// future unit change has an explicit migration signal to key off.
const SUPPORTED_UNITS: &str = "millimeter";

/// Number of most-recent backups kept per project; older ones are pruned.
const MAX_BACKUPS: usize = 10;

#[derive(Serialize)]
struct ProjectFileRef<'a> {
    schema_version: u32,
    units: &'a str,
    project: &'a Project,
}

#[derive(Deserialize)]
struct ProjectFileOwned {
    schema_version: u32,
    units: String,
    project: Project,
}

#[derive(Debug)]
pub enum IoError {
    Io(io::Error),
    Serde(serde_json::Error),
    UnsupportedSchemaVersion(u32),
    UnsupportedUnits(String),
    Validation(ValidationReport),
    /// The temp file that was just written did not deserialize back to an
    /// identical project. Save was aborted before touching the real file.
    RoundTripMismatch,
}

impl From<io::Error> for IoError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for IoError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "file I/O error: {e}"),
            Self::Serde(e) => write!(f, "JSON error: {e}"),
            Self::UnsupportedSchemaVersion(v) => write!(f, "unsupported schema_version: {v}"),
            Self::UnsupportedUnits(u) => write!(f, "unsupported units: {u}"),
            Self::Validation(report) => {
                write!(f, "validation failed with {} issue(s)", report.issues.len())
            }
            Self::RoundTripMismatch => {
                write!(f, "saved file did not round-trip to an identical project")
            }
        }
    }
}

impl std::error::Error for IoError {}

fn validate_for_save(project: &Project) -> Result<(), IoError> {
    let report = project.drawing.validate();
    if report.has_errors() {
        Err(IoError::Validation(report))
    } else {
        Ok(())
    }
}

fn temp_path_for(path: &Path) -> PathBuf {
    let mut os_string = path.as_os_str().to_owned();
    os_string.push(".tmp");
    PathBuf::from(os_string)
}

/// Writes `project` to `path` following the save pipeline: temp file write,
/// flush, re-read-and-compare, then atomic rename over the real path.
/// Does not touch backups; callers that want a backup call [`save_project`]
/// instead.
///
/// # Errors
///
/// Returns `Validation` if `project` fails structural validation, or an I/O
/// / serialization error if any step of the write fails. On any error the
/// file at `path` is left untouched.
fn write_project_atomically(project: &Project, path: &Path) -> Result<(), IoError> {
    validate_for_save(project)?;

    let file_contents = ProjectFileRef {
        schema_version: CURRENT_SCHEMA_VERSION,
        units: SUPPORTED_UNITS,
        project,
    };
    let json = serde_json::to_string_pretty(&file_contents)?;

    let tmp_path = temp_path_for(path);
    {
        let mut handle = fs::File::create(&tmp_path)?;
        handle.write_all(json.as_bytes())?;
        handle.sync_all()?;
    }

    let round_trip_contents = fs::read_to_string(&tmp_path)?;
    let round_trip: ProjectFileOwned = serde_json::from_str(&round_trip_contents)?;
    if round_trip.project != *project {
        let _ = fs::remove_file(&tmp_path);
        return Err(IoError::RoundTripMismatch);
    }

    fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Saves a project: validates, writes it atomically, then rotates a backup
/// copy into the `backups/` folder next to `path` (keeping the most recent
/// [`MAX_BACKUPS`]).
///
/// # Errors
///
/// See [`write_project_atomically`]. Backup I/O errors are also surfaced.
pub fn save_project(project: &Project, path: &Path) -> Result<(), IoError> {
    write_project_atomically(project, path)?;
    update_backup(path)?;
    Ok(())
}

/// Loads and validates a project from `path`.
///
/// # Errors
///
/// Returns `UnsupportedSchemaVersion`/`UnsupportedUnits` if the file was
/// written by an incompatible version, `Validation` if the loaded project
/// fails structural validation, or an I/O / parse error.
pub fn load_project(path: &Path) -> Result<Project, IoError> {
    let contents = fs::read_to_string(path)?;
    let file: ProjectFileOwned = serde_json::from_str(&contents)?;

    if file.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(IoError::UnsupportedSchemaVersion(file.schema_version));
    }
    if file.units != SUPPORTED_UNITS {
        return Err(IoError::UnsupportedUnits(file.units));
    }

    let report = file.project.drawing.validate();
    if report.has_errors() {
        return Err(IoError::Validation(report));
    }

    Ok(file.project)
}

/// The autosave path for a project file: `name.cadproj` ->
/// `name.autosave.cadproj`, in the same folder.
#[must_use]
pub fn autosave_path(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let file_name = format!("{stem}.autosave.cadproj");
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(file_name),
        _ => PathBuf::from(file_name),
    }
}

/// Writes `project` to its autosave path. Unlike [`save_project`], this does
/// not rotate a backup — autosaves happen too frequently for that to be
/// useful, and the user's explicit saves are what backups protect.
///
/// # Errors
///
/// See [`write_project_atomically`].
pub fn autosave_project(project: &Project, original_path: &Path) -> Result<(), IoError> {
    write_project_atomically(project, &autosave_path(original_path))
}

fn backups_dir_for(path: &Path) -> PathBuf {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join("backups"),
        _ => PathBuf::from("backups"),
    }
}

static BACKUP_SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// Builds a lexicographically-sortable, collision-resistant backup file
/// name: fixed-width millisecond timestamp plus a per-process sequence
/// number, so two backups written within the same millisecond still sort
/// and prune correctly.
fn backup_file_name(stem: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = BACKUP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{stem}_{millis:016}_{seq:06}.cadproj")
}

fn update_backup(path: &Path) -> Result<(), IoError> {
    let backups_dir = backups_dir_for(path);
    fs::create_dir_all(&backups_dir)?;

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let backup_name = backup_file_name(stem);
    fs::copy(path, backups_dir.join(&backup_name))?;

    prune_backups(&backups_dir, stem)
}

fn prune_backups(backups_dir: &Path, stem: &str) -> Result<(), IoError> {
    let prefix = format!("{stem}_");
    let mut names: Vec<String> = fs::read_dir(backups_dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(&prefix) && name.ends_with(".cadproj"))
        .collect();
    names.sort();

    if names.len() > MAX_BACKUPS {
        for name in &names[..names.len() - MAX_BACKUPS] {
            fs::remove_file(backups_dir.join(name))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SVG export
// ---------------------------------------------------------------------------

type Bounds = (f64, f64, f64, f64);

fn extend_bounds(bounds: &mut Option<Bounds>, x: f64, y: f64) {
    *bounds = Some(match *bounds {
        None => (x, y, x, y),
        Some((min_x, min_y, max_x, max_y)) => {
            (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
        }
    });
}

fn bounds_of(primitives: &[RenderPrimitive]) -> Option<Bounds> {
    let mut bounds = None;
    for primitive in primitives {
        match primitive {
            RenderPrimitive::Line { start, end }
            | RenderPrimitive::Dimension { start, end, .. } => {
                extend_bounds(&mut bounds, start.0, start.1);
                extend_bounds(&mut bounds, end.0, end.1);
            }
            RenderPrimitive::Polyline { points, .. } => {
                for p in points {
                    extend_bounds(&mut bounds, p.0, p.1);
                }
            }
            RenderPrimitive::Rectangle {
                origin,
                width,
                height,
            } => {
                extend_bounds(&mut bounds, origin.0, origin.1);
                extend_bounds(&mut bounds, origin.0 + width, origin.1 + height);
            }
            RenderPrimitive::Circle { center, radius }
            | RenderPrimitive::Arc { center, radius, .. } => {
                extend_bounds(&mut bounds, center.0 - radius, center.1 - radius);
                extend_bounds(&mut bounds, center.0 + radius, center.1 + radius);
            }
            RenderPrimitive::Text { origin, .. } => {
                extend_bounds(&mut bounds, origin.0, origin.1);
            }
        }
    }
    bounds
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// SVG's sweep-flag for a CAD sweep angle. CAD angles increase
/// counter-clockwise in a Y-up space; this renderer flips Y (`svg_y =
/// -cad_y`) so drawings display right-side-up, which reverses the raw
/// angle direction fed to SVG's `A` path command.
fn svg_sweep_flag(sweep_angle: f64) -> u8 {
    u8::from(sweep_angle <= 0.0)
}

fn svg_large_arc_flag(sweep_angle: f64) -> u8 {
    u8::from(sweep_angle.abs() > std::f64::consts::PI)
}

fn arc_path_svg(center: (f64, f64), radius: f64, start_angle: f64, sweep_angle: f64) -> String {
    let end_angle = start_angle + sweep_angle;
    let start = (
        center.0 + radius * start_angle.cos(),
        center.1 + radius * start_angle.sin(),
    );
    let end = (
        center.0 + radius * end_angle.cos(),
        center.1 + radius * end_angle.sin(),
    );
    format!(
        "  <path d=\"M {} {} A {radius} {radius} 0 {} {} {} {}\" fill=\"none\" stroke=\"black\" />\n",
        start.0,
        -start.1,
        svg_large_arc_flag(sweep_angle),
        svg_sweep_flag(sweep_angle),
        end.0,
        -end.1
    )
}

fn render_primitive_svg(primitive: &RenderPrimitive) -> String {
    match primitive {
        RenderPrimitive::Line { start, end } => format!(
            "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" />\n",
            start.0, -start.1, end.0, -end.1
        ),
        RenderPrimitive::Polyline { points, closed } => {
            let coords: Vec<String> = points.iter().map(|p| format!("{},{}", p.0, -p.1)).collect();
            let tag = if *closed { "polygon" } else { "polyline" };
            format!(
                "  <{tag} points=\"{}\" fill=\"none\" stroke=\"black\" />\n",
                coords.join(" ")
            )
        }
        RenderPrimitive::Rectangle {
            origin,
            width,
            height,
        } => format!(
            "  <rect x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{height}\" fill=\"none\" stroke=\"black\" />\n",
            origin.0,
            -(origin.1 + height),
        ),
        RenderPrimitive::Circle { center, radius } => format!(
            "  <circle cx=\"{}\" cy=\"{}\" r=\"{radius}\" fill=\"none\" stroke=\"black\" />\n",
            center.0, -center.1,
        ),
        RenderPrimitive::Arc {
            center,
            radius,
            start_angle,
            sweep_angle,
        } => arc_path_svg(*center, *radius, *start_angle, *sweep_angle),
        RenderPrimitive::Text {
            origin,
            content,
            height,
        } => format!(
            "  <text x=\"{}\" y=\"{}\" font-size=\"{height}\">{}</text>\n",
            origin.0,
            -origin.1,
            escape_xml(content)
        ),
        RenderPrimitive::Dimension { start, end, .. } => format!(
            "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"gray\" stroke-dasharray=\"4 2\" />\n",
            start.0, -start.1, end.0, -end.1
        ),
    }
}

/// Renders a project to an SVG document. Coordinates are flipped on Y so the
/// drawing displays right-side-up (SVG's Y axis points down; CAD's points
/// up). Dimension primitives are drawn as a plain dashed line between their
/// `start`/`end` points — the full dimension-line/extension-line/arrowhead
/// layout is a UI-layer decision (see Phase 5's write-up).
#[must_use]
pub fn export_svg(project: &Project) -> String {
    let primitives = cad_render::build_render_model(project);
    let (min_x, min_y, max_x, max_y) = bounds_of(&primitives).unwrap_or((0.0, 0.0, 100.0, 100.0));
    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);
    let view_box_y = -max_y;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{min_x} {view_box_y} {width} {height}\">\n"
    );
    for primitive in &primitives {
        svg.push_str(&render_primitive_svg(primitive));
    }
    svg.push_str("</svg>\n");
    svg
}

/// Renders `project` to SVG and writes it to `path`. This is a derived
/// export, not the project's source of truth, so it is written directly
/// without the temp-file/backup pipeline used by [`save_project`].
///
/// # Errors
///
/// Returns an I/O error if the file cannot be written.
pub fn save_svg(project: &Project, path: &Path) -> Result<(), IoError> {
    fs::write(path, export_svg(project))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_core::{
        Entity, EntityGeometry, EntityId, Layer, LayerId, Line, Point2, DEFAULT_LAYER_ID,
    };
    use std::sync::atomic::AtomicU64;

    static TEST_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = TEST_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cad_studio_io_test_{nanos}_{seq}"));
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
    fn save_then_load_round_trips() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");
        let project = sample_project();

        save_project(&project, &path).unwrap();
        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded, project);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_rejects_invalid_project_and_writes_nothing() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");
        let mut project = Project::default();
        project.drawing.entities.push(Entity {
            id: EntityId::new(1),
            layer_id: LayerId::new(99),
            geometry: EntityGeometry::Line(Line {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(1.0, 0.0),
            }),
        });

        let result = save_project(&project, &path);
        assert!(matches!(result, Err(IoError::Validation(_))));
        assert!(!path.exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_rejects_unsupported_schema_version() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");
        fs::write(
            &path,
            r#"{"schema_version":999,"units":"millimeter","project":{"id":0,"drawing":{"id":0,"entities":[],"layers":[{"id":0,"name":"0","visible":true,"locked":false}],"dimensions":[]}}}"#,
        )
        .unwrap();

        let result = load_project(&path);
        assert!(matches!(
            result,
            Err(IoError::UnsupportedSchemaVersion(999))
        ));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn autosave_path_uses_autosave_suffix() {
        let path = Path::new("house.cadproj");
        assert_eq!(autosave_path(path), Path::new("house.autosave.cadproj"));
    }

    #[test]
    fn save_creates_a_backup_file() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");
        save_project(&sample_project(), &path).unwrap();

        let backups_dir = dir.join("backups");
        let backups: Vec<_> = fs::read_dir(&backups_dir).unwrap().collect();
        assert_eq!(backups.len(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn backups_are_pruned_to_max_count() {
        let dir = temp_dir();
        let path = dir.join("test.cadproj");
        let project = sample_project();
        for _ in 0..(MAX_BACKUPS + 3) {
            save_project(&project, &path).unwrap();
        }

        let backups_dir = dir.join("backups");
        let backups: Vec<_> = fs::read_dir(&backups_dir).unwrap().collect();
        assert_eq!(backups.len(), MAX_BACKUPS);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_svg_contains_expected_elements() {
        let svg = export_svg(&sample_project());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<line"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn export_svg_skips_entities_on_hidden_layers() {
        let mut project = Project::default();
        let hidden = LayerId::new(1);
        let mut layer = Layer::new(hidden, "hidden");
        layer.visible = false;
        project.drawing.add_layer(layer).unwrap();
        project
            .drawing
            .add_entity(Entity {
                id: EntityId::new(1),
                layer_id: hidden,
                geometry: EntityGeometry::Line(Line {
                    start: Point2::new(0.0, 0.0),
                    end: Point2::new(1.0, 1.0),
                }),
            })
            .unwrap();

        assert!(!export_svg(&project).contains("<line"));
    }

    #[test]
    fn svg_sweep_flag_matches_cad_ccw_convention() {
        assert_eq!(svg_sweep_flag(1.0), 0);
        assert_eq!(svg_sweep_flag(-1.0), 1);
    }

    #[test]
    fn svg_large_arc_flag_reflects_sweep_magnitude() {
        assert_eq!(svg_large_arc_flag(std::f64::consts::PI * 1.5), 1);
        assert_eq!(svg_large_arc_flag(std::f64::consts::FRAC_PI_2), 0);
    }
}
