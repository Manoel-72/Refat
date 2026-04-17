#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// ============================================================
//  RS2BR-Engine - Entry point do editor
// ============================================================

pub mod core;
pub mod assets;
pub mod serialization;
pub mod runtime;
pub mod editor;
pub mod renderer;
pub mod standalone;

use editor::EditorApp;

pub use crate::core::{component, entity, prefab, project, scene, version};


fn load_app_icon() -> Option<egui::IconData> {
    let cwd = std::env::current_dir().ok();
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let candidates = [
        cwd.as_ref().map(|p| p.join("assets/icon/rs2br_engine_icon.png")),
        exe_dir.as_ref().map(|p| p.join("assets/icon/rs2br_engine_icon.png")),
        exe_dir.as_ref().map(|p| p.join("../assets/icon/rs2br_engine_icon.png")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if let Ok(bytes) = std::fs::read(&candidate) {
            if let Ok(image) = image::load_from_memory(&bytes) {
                let image = image.into_rgba8();
                let (width, height) = image.dimensions();
                return Some(egui::IconData {
                    rgba: image.into_raw(),
                    width,
                    height,
                });
            }
        }
    }

    None
}

fn main() {
    env_logger::init();

    let standalone_root = standalone::detect_standalone_project_root();
    let window_title = if let Some(root) = &standalone_root {
        let cfg = crate::core::project::ProjectConfig::load_or_create(root);
        format!("{}", cfg.name)
    } else {
        format!("{} {}", version::ENGINE_TITLE, version::ENGINE_VERSION)
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&window_title)
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(load_app_icon().unwrap_or_else(|| egui::IconData { rgba: vec![255,255,255,255], width: 1, height: 1 })),
        ..Default::default()
    };

    if let Some(project_root) = standalone_root {
        eframe::run_native(
            &window_title,
            options,
            Box::new(move |cc| Ok(Box::new(standalone::StandaloneApp::new(cc, project_root.clone())))),
        )
        .expect("Falha ao iniciar o jogo standalone");
    } else {
        eframe::run_native(
            &window_title,
            options,
            Box::new(|cc| Ok(Box::new(EditorApp::new(cc)))),
        )
        .expect("Falha ao iniciar o editor");
    }
}
