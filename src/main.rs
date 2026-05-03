#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// ============================================================
//  RS2BR-Engine - Entry point do editor
// ============================================================
//
// CLI: `rs2br-engine build --project <dir> [--output <exe>]`
//       (stderr: passos e log do Cargo; código 0 = pacote criado)

pub mod assets;
pub mod cli;
pub mod core;
pub mod editor;
pub mod effects;
/// Reexportações legadas; modelo de dados canônico está em [`core`].
pub mod engine;
pub mod renderer;
pub mod runtime;
pub mod serialization;
pub mod standalone;
pub mod standalone_export;
pub mod world;

use editor::EditorApp;

pub use crate::core::{component, entity, prefab, project, scene, version};

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn AttachConsole(dw_process_id: u32) -> i32;
}

#[cfg(windows)]
fn attach_parent_console_for_cli() {
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_parent_console_for_cli() {}

fn load_app_icon() -> Option<egui::IconData> {
    const EMBEDDED_PNG: &[u8] = include_bytes!("../assets/icon/rs2br_engine_icon.png");
    if let Ok(image) = image::load_from_memory(EMBEDDED_PNG) {
        let image = image.into_rgba8();
        let (width, height) = image.dimensions();
        return Some(egui::IconData {
            rgba: image.into_raw(),
            width,
            height,
        });
    }

    let cwd = std::env::current_dir().ok();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let candidates = [
        cwd.as_ref()
            .map(|p| p.join("assets/icon/rs2br_engine_icon.png")),
        exe_dir
            .as_ref()
            .map(|p| p.join("assets/icon/rs2br_engine_icon.png")),
        exe_dir
            .as_ref()
            .map(|p| p.join("../assets/icon/rs2br_engine_icon.png")),
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

    use clap::Parser;
    let parsed = cli::EngineCli::parse();
    if let Some(cli::EngineCommands::Build(args)) = parsed.command {
        attach_parent_console_for_cli();
        std::process::exit(cli::run_build_command(args));
    }

    let standalone_root = standalone::detect_standalone_project_root();

    let (window_title, standalone_fullscreen) = match &standalone_root {
        Some(root) => {
            let cfg = crate::core::project::ProjectConfig::load_or_create(root);
            (cfg.name.clone(), cfg.start_fullscreen)
        }
        None => (
            format!("{} {}", version::ENGINE_TITLE, version::ENGINE_VERSION),
            false,
        ),
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(&window_title)
        .with_inner_size([1366.0, 768.0])
        .with_min_inner_size([800.0, 600.0])
        .with_icon(load_app_icon().unwrap_or_else(|| egui::IconData {
            rgba: vec![255, 255, 255, 255],
            width: 1,
            height: 1,
        }));

    if standalone_root.is_some() {
        // Standalone sempre abre maximizado para cobrir o monitor inteiro,
        // independente da resolução (1440x900, 1920x1080, 2560x1440, etc.).
        // with_maximized garante cobertura total sem bordas do WM.
        viewport = viewport.with_maximized(true);
    }

    if standalone_root.is_some() && standalone_fullscreen {
        viewport = viewport.with_fullscreen(true);
    }

    let options = eframe::NativeOptions {
        viewport,
        // Limita FPS ao refresh do monitor; alinha com request_repaint no loop de jogo.
        vsync: true,
        ..Default::default()
    };

    if let Some(project_root) = standalone_root {
        eframe::run_native(
            &window_title,
            options,
            Box::new(move |cc| {
                Ok(Box::new(standalone::StandaloneApp::new(
                    cc,
                    project_root.clone(),
                )))
            }),
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
