# CAD Studio

Local-first Rust workspace for safe, repeatable architectural and mechanical 2D drawing production.

## Initial setup

This first milestone intentionally uses only the Rust standard library. External crates for GUI, persistence, rendering, and standards data require explicit approval under the project policy.

## Workspace crates

- `cad_core`: domain model and stable identifiers
- `cad_geometry`: deterministic geometry primitives
- `cad_tolerance`: tolerance calculations and traces
- `cad_command`: command execution and undo/redo boundaries
- `cad_render`: render model boundary
- `cad_io`: persistence and export boundary
- `cad_batch`: batch workflow boundary
- `cad_cli`: command-line entry point
- `cad_app`: GUI entry point boundary

## Verification

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p cad_cli -- --help
```

