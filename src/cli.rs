//! Subcomandos de linha de comando (`rs2br-engine build`, etc.).
use std::{
    path::{Path, PathBuf},
    thread,
};

use clap::{Parser, Subcommand};

use crate::core::{project::ProjectConfig, scene::Scene};
use crate::standalone_export::{
    self, find_engine_root, find_prebuilt_runtime_exe, preflight_standalone_toolchain,
    sanitize_filename, StandaloneBuildNotice, StandaloneExportParams,
};

#[derive(Parser, Debug)]
#[command(
    name = "rs2br-engine",
    version,
    about = "RS2BR Engine — editor gráfico e exportação standalone"
)]
pub struct EngineCli {
    #[command(subcommand)]
    pub command: Option<EngineCommands>,
}

#[derive(Subcommand, Debug)]
pub enum EngineCommands {
    /// Exporta build standalone PC (mesmo fluxo que o menu do editor).
    Build(BuildArgs),
}

#[derive(clap::Args, Debug)]
pub struct BuildArgs {
    /// Pasta do projeto (deve conter `project.json`).
    #[arg(long, value_name = "DIR", default_value = ".")]
    pub project: PathBuf,
    /// Caminho do executável a gravar (como no diálogo do editor: define pasta e nome do pacote).
    /// Por omissão: `<project>/build/<nome_seguro>.exe` (Windows) ou sem `.exe` em outros SO.
    #[arg(long, value_name = "PATH")]
    pub output: Option<PathBuf>,
    /// Não regravar a cena inicial (`project.json` → `initial_scene`) em disco antes da build.
    #[arg(long, default_value_t = false)]
    pub no_save_scene: bool,
}

/// Devolve código de saída (0 = sucesso). Imprime passos em stderr.
pub fn run_build_command(args: BuildArgs) -> i32 {
    let project_root = match args.project.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("rs2br-engine build: pasta do projeto inválida: {}", e);
            return 1;
        }
    };

    if !project_root.join("project.json").is_file() {
        eprintln!(
            "rs2br-engine build: não há project.json em {}.",
            project_root.display()
        );
        return 1;
    }

    if let Err(e) = touch_initial_scene(&project_root, args.no_save_scene) {
        eprintln!("rs2br-engine build: {}", e);
        return 1;
    }

    let project_cfg = ProjectConfig::load_or_create(&project_root);
    let project_name = project_cfg.name.clone();
    let safe = sanitize_filename(project_name.trim());
    let initial_abs = project_root.join(&project_cfg.initial_scene);
    let saved_scene_name = initial_abs
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("main.scene.json")
        .to_string();

    let engine_root = match find_engine_root(&project_root) {
        Some(p) => p,
        None => {
            eprintln!(
                "rs2br-engine build: não foi possível localizar a raiz da engine (Cargo.toml)."
            );
            return 1;
        }
    };

    let cargo_toml = engine_root.join("Cargo.toml");
    let bin_name =
        standalone_export::detect_bin_name(&cargo_toml).unwrap_or_else(|| "rs2br-engine".to_string());
    let engine_exe_name = if cfg!(target_os = "windows") {
        format!("{}.exe", bin_name)
    } else {
        bin_name.clone()
    };

    if let Err(e) = preflight_standalone_toolchain(&engine_root, &engine_exe_name) {
        eprintln!("rs2br-engine build: {}", e);
        return 1;
    }

    let default_output = if cfg!(target_os = "windows") {
        project_root
            .join("build")
            .join(format!("{}.exe", safe))
    } else {
        project_root.join("build").join(safe.clone())
    };

    let selected_output = args.output.unwrap_or(default_output);
    if let Some(parent) = selected_output.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("rs2br-engine build: não foi possível criar {}: {}", parent.display(), e);
            return 1;
        }
    }

    let assets_src = project_root.join("assets");
    let save_src = project_root.join("save");
    let project_json_src = project_root.join("project.json");

    let params = StandaloneExportParams {
        engine_root,
        bin_name,
        engine_exe_name,
        project_name,
        assets_src,
        save_src,
        project_json_src,
        saved_scene_name,
        selected_output,
    };

    let use_prebuilt = find_prebuilt_runtime_exe(&params.engine_root, &params.engine_exe_name).is_some();
    if use_prebuilt {
        eprintln!("rs2br-engine build: runtime pré-compilado — sem Cargo.");
    }

    let (notice_tx, notice_rx) = std::sync::mpsc::channel::<StandaloneBuildNotice>();
    let printer = thread::spawn(move || {
        while let Ok(n) = notice_rx.recv() {
            match n {
                StandaloneBuildNotice::Step(s) => eprintln!("▶ {}", s),
                StandaloneBuildNotice::Log(l) => eprintln!("{}", l),
            }
        }
    });

    let result = thread::spawn(move || {
        let r = standalone_export::export_standalone_pc(&params, &notice_tx);
        drop(notice_tx);
        r
    })
    .join();

    let _ = printer.join();

    match result {
        Ok(Ok(path)) => {
            eprintln!("rs2br-engine build: concluído — {}", path.display());
            0
        }
        Ok(Err(e)) => {
            eprintln!("rs2br-engine build: {}", e);
            1
        }
        Err(_) => {
            eprintln!("rs2br-engine build: thread de build encerrou com pânico.");
            1
        }
    }
}

fn touch_initial_scene(project_root: &Path, skip: bool) -> Result<(), String> {
    if skip {
        return Ok(());
    }
    let cfg = ProjectConfig::load_or_create(project_root);
    let scene_path = project_root.join(&cfg.initial_scene);
    let scene = Scene::load_from_path(&scene_path).ok_or_else(|| {
        format!(
            "cena inicial não encontrada ou inválida: {}",
            scene_path.display()
        )
    })?;
    scene
        .save_to_path(&scene_path)
        .map_err(|e| format!("erro ao gravar cena inicial: {}", e))?;
    Ok(())
}
