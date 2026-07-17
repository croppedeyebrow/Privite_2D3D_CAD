#![forbid(unsafe_code)]

mod app;
mod camera;
mod demo;

use app::CadApp;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "CAD Studio",
        eframe::NativeOptions::default(),
        Box::new(|_creation_context| Ok(Box::new(CadApp::default()))),
    )
}
