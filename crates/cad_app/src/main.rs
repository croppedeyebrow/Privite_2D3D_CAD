#![forbid(unsafe_code)]

mod app;
mod camera;
mod demo;
mod tool;

use app::CadApp;

/// `egui`'s built-in fonts (Hack / Ubuntu-Light) have no Korean glyphs, so
/// every Korean label in this UI would render as tofu boxes without this.
/// Rather than bundle a font file (a new binary asset with its own license
/// to track), this loads a Korean-capable font already present on Windows.
/// If none of the candidates exist (e.g. a non-Windows build), Korean text
/// falls back to tofu and the rest of the UI still works.
fn install_korean_font(ctx: &egui::Context) {
    const CANDIDATES: [&str; 2] = ["C:/Windows/Fonts/malgun.ttf", "C:/Windows/Fonts/NGULIM.TTF"];

    let Some(bytes) = CANDIDATES.iter().find_map(|path| std::fs::read(path).ok()) else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("korean".to_owned(), egui::FontData::from_owned(bytes));
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "korean".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("korean".to_owned());
    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "CAD Studio",
        eframe::NativeOptions::default(),
        Box::new(|creation_context| {
            install_korean_font(&creation_context.egui_ctx);
            Ok(Box::new(CadApp::default()))
        }),
    )
}
