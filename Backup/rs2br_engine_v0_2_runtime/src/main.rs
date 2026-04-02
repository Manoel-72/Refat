// ============================================================
//  Rust2D Engine - Entry point do editor
//  Refatorado para reduzir acoplamento no módulo editor.
// ============================================================

mod engine;
mod runtime;
mod editor;
mod renderer;

use editor::EditorApp;

fn main() {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RS2BR-Engine")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

   eframe::run_native(
    "RS2BR-Engine",
        options,
        Box::new(|cc| Ok(Box::new(EditorApp::new(cc)))),
    )
    .expect("Falha ao iniciar o editor");
}
