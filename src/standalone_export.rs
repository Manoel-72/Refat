//! Exportação standalone PC (editor + CLI). Lógica única para não haver deriva.
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::Sender,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

/// Mensagens da thread de build para a UI ou para stdout no modo CLI.
#[derive(Debug)]
pub enum StandaloneBuildNotice {
    Step(String),
    Log(String),
}

/// Parâmetros já validados (equivalente ao fluxo pós-diálogo do editor).
#[derive(Debug, Clone)]
pub struct StandaloneExportParams {
    pub engine_root: PathBuf,
    pub bin_name: String,
    pub engine_exe_name: String,
    pub project_name: String,
    pub assets_src: PathBuf,
    pub save_src: PathBuf,
    pub project_json_src: PathBuf,
    pub saved_scene_name: String,
    pub selected_output: PathBuf,
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn find_engine_root(project_root: &Path) -> Option<PathBuf> {
    for base in [project_root.to_path_buf(), std::env::current_dir().ok()?] {
        for ancestor in base.ancestors() {
            let candidate = ancestor.join("Cargo.toml");
            if candidate.exists() {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

pub fn detect_bin_name(cargo_toml_path: &Path) -> Option<String> {
    let content = fs::read_to_string(cargo_toml_path).ok()?;
    let mut in_bin = false;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line == "[[bin]]" {
            in_bin = true;
            continue;
        }
        if line.starts_with('[') && line != "[[bin]]" {
            if in_bin {
                break;
            }
        }
        if in_bin && line.starts_with("name") {
            let value = line.split('=').nth(1)?.trim().trim_matches('"').to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.starts_with("name") {
            let value = line.split('=').nth(1)?.trim().trim_matches('"').to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

pub fn detect_command_available(command: &str) -> bool {
    let result = Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    matches!(result, Ok(status) if status.success())
}

pub fn find_prebuilt_runtime_exe(engine_root: &Path, engine_exe_name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(
        engine_root
            .join("standalone_runtime")
            .join("windows")
            .join(engine_exe_name),
    );
    candidates.push(
        engine_root
            .join("standalone_runtime")
            .join(engine_exe_name),
    );
    candidates.push(
        engine_root
            .join("build")
            .join("standalone_runtime")
            .join("windows")
            .join(engine_exe_name),
    );
    candidates.push(
        engine_root
            .join("build")
            .join("standalone_runtime")
            .join(engine_exe_name),
    );
    candidates.push(
        engine_root
            .join("target")
            .join("release")
            .join(engine_exe_name),
    );
    if let Ok(custom_dir) = std::env::var("RS2BR_STANDALONE_RUNTIME_DIR") {
        let trimmed = custom_dir.trim();
        if !trimmed.is_empty() {
            candidates.push(PathBuf::from(trimmed).join(engine_exe_name));
        }
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn unique_temp_build_dir_in_parent(parent: &Path, project_name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    parent.join(format!(
        ".rs2br_build_cache_{}_{}",
        sanitize_filename(project_name),
        timestamp
    ))
}

fn copy_minimal_engine_workspace(src_root: &Path, dst_root: &Path) -> std::io::Result<()> {
    fn copy_file_if_exists(src_root: &Path, dst_root: &Path, rel: &str) -> std::io::Result<()> {
        let src = src_root.join(rel);
        if src.is_file() {
            let dst = dst_root.join(rel);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(src, dst)?;
        }
        Ok(())
    }

    fn copy_dir_recursive_filtered(src: &Path, dst: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == "target" || name == ".git" || name == "build" || name == ".cargo-lock" {
                continue;
            }
            let target = dst.join(entry.file_name());
            let ty = entry.file_type()?;
            if ty.is_dir() {
                copy_dir_recursive_filtered(&path, &target)?;
            } else if ty.is_file() {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&path, &target)?;
            }
        }
        Ok(())
    }

    if dst_root.exists() {
        fs::remove_dir_all(dst_root)?;
    }
    fs::create_dir_all(dst_root)?;

    copy_file_if_exists(src_root, dst_root, "Cargo.toml")?;
    copy_file_if_exists(src_root, dst_root, "Cargo.lock")?;
    copy_file_if_exists(src_root, dst_root, "build.rs")?;
    copy_file_if_exists(src_root, dst_root, "assets/icon/rs2br_engine_icon.ico")?;
    copy_file_if_exists(src_root, dst_root, "assets/icon/rs2br_engine_icon.png")?;

    let cargo_dir = src_root.join(".cargo");
    if cargo_dir.is_dir() {
        copy_dir_recursive_filtered(&cargo_dir, &dst_root.join(".cargo"))?;
    }

    let src_dir = src_root.join("src");
    if !src_dir.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Pasta src da engine não encontrada",
        ));
    }
    copy_dir_recursive_filtered(&src_dir, &dst_root.join("src"))?;

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ty.is_file() {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn copy_runtime_binary_set(
    exe_src: &Path,
    dst_root: &Path,
    game_exe_name: &str,
) -> std::io::Result<()> {
    fs::create_dir_all(dst_root)?;
    fs::copy(exe_src, dst_root.join(game_exe_name))?;
    let Some(runtime_dir) = exe_src.parent() else {
        return Ok(());
    };
    for entry in fs::read_dir(runtime_dir)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if !ty.is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if ext == "dll" {
            fs::copy(&path, dst_root.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Pré-checagem (Cargo ou runtime pré-compilado).
pub fn preflight_standalone_toolchain(engine_root: &Path, engine_exe_name: &str) -> Result<(), String> {
    let prebuilt = find_prebuilt_runtime_exe(engine_root, engine_exe_name);
    let cargo_ok = detect_command_available("cargo");
    if prebuilt.is_none() && !cargo_ok {
        return Err(
            "Rust/Cargo não encontrado e não há runtime pré-compilado disponível. \
             Para exportar sem Rust instalado, adicione um executável em standalone_runtime/windows/ na raiz da engine."
                .to_string(),
        );
    }
    Ok(())
}

/// Executa o pacote standalone (apenas Step/Log pelo canal; o chamador envia Finished).
pub fn export_standalone_pc(
    params: &StandaloneExportParams,
    tx: &Sender<StandaloneBuildNotice>,
) -> Result<PathBuf, String> {
    let safe_project_name = sanitize_filename(&params.project_name);
    let chosen_stem = params
        .selected_output
        .file_stem()
        .and_then(|s| s.to_str())
        .map(sanitize_filename)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| safe_project_name.clone());

    let selected_parent = params
        .selected_output
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "Caminho de saída inválido.".to_string())?;

    let export_root = selected_parent.join(&chosen_stem);
    let build_cache_root = unique_temp_build_dir_in_parent(&selected_parent, &chosen_stem);
    let game_exe_name = if cfg!(target_os = "windows") {
        format!("{}.exe", chosen_stem)
    } else {
        chosen_stem.clone()
    };

    let engine_root = &params.engine_root;
    let bin_name = &params.bin_name;
    let engine_exe_name = &params.engine_exe_name;
    let project_name = &params.project_name;
    let assets_src = &params.assets_src;
    let save_src = &params.save_src;
    let project_json_src = &params.project_json_src;
    let saved_scene_name = &params.saved_scene_name;

    let prebuilt_runtime_exe = find_prebuilt_runtime_exe(engine_root, engine_exe_name);
    let use_prebuilt_runtime = prebuilt_runtime_exe.is_some();

    let send_step = |msg: &str| {
        let _ = tx.send(StandaloneBuildNotice::Step(msg.to_string()));
    };

    let run = || -> Result<PathBuf, String> {
        if build_cache_root.exists() {
            let _ = fs::remove_dir_all(&build_cache_root);
        }
        fs::create_dir_all(&build_cache_root)
            .map_err(|e| format!("Falha ao preparar cache temporário da build: {}", e))?;

        send_step("Preparando pasta de saída...");
        let staged_export_root = build_cache_root.join("final_bundle");
        fs::create_dir_all(&staged_export_root)
            .map_err(|e| format!("Falha ao criar pasta temporária de saída: {}", e))?;

        let exe_src = if let Some(prebuilt_path) = prebuilt_runtime_exe.as_ref() {
            send_step("Usando runtime pré-compilado (sem Cargo)...");
            let _ = tx.send(StandaloneBuildNotice::Log(format!(
                "▶ runtime pré-compilado: {}",
                prebuilt_path.display()
            )));
            prebuilt_path.clone()
        } else {
            let temp_engine_root = build_cache_root.join("engine_workspace");
            send_step("Preparando workspace temporário...");
            copy_minimal_engine_workspace(engine_root, &temp_engine_root)
                .map_err(|e| format!("Falha ao preparar workspace temporário: {}", e))?;

            send_step("Compilando jogo standalone em release...");

            let mut child = Command::new("cargo")
                .arg("build")
                .arg("--release")
                .arg("--bin")
                .arg(bin_name)
                .env("CARGO_TARGET_DIR", build_cache_root.join("target"))
                .current_dir(&temp_engine_root)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| {
                    format!(
                        "Falha ao executar cargo build --release: {}. \
                         Dica: instale Rust/Cargo ou configure runtime pré-compilado em standalone_runtime/windows/.",
                        e
                    )
                })?;

            let _ = tx.send(StandaloneBuildNotice::Log(format!(
                "▶ comando: cargo build --release --bin {} ({})",
                bin_name,
                temp_engine_root.display()
            )));

            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            let stdout_handle = stdout.map(|pipe| {
                let tx_out = tx.clone();
                thread::spawn(move || {
                    let reader = BufReader::new(pipe);
                    for line in reader.lines().map_while(Result::ok) {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            let _ = tx_out.send(StandaloneBuildNotice::Log(trimmed.to_string()));
                        }
                    }
                })
            });

            let stderr_handle = stderr.map(|pipe| {
                let tx_err = tx.clone();
                thread::spawn(move || {
                    let reader = BufReader::new(pipe);
                    for line in reader.lines().map_while(Result::ok) {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            let _ = tx_err.send(StandaloneBuildNotice::Log(trimmed.to_string()));
                        }
                    }
                })
            });

            let status = child
                .wait()
                .map_err(|e| format!("Falha ao aguardar término da build: {}", e))?;
            if let Some(handle) = stdout_handle {
                let _ = handle.join();
            }
            if let Some(handle) = stderr_handle {
                let _ = handle.join();
            }

            if !status.success() {
                return Err(format!(
                    "Build falhou (status: {}). Veja os detalhes no console. Cache em {}",
                    status,
                    build_cache_root.display()
                ));
            }

            build_cache_root
                .join("target")
                .join("release")
                .join(engine_exe_name)
        };

        send_step("Montando pasta final do jogo...");

        if !exe_src.exists() {
            return Err(format!(
                "Build concluída, mas o executável não foi encontrado em {}.",
                exe_src.display()
            ));
        }

        copy_runtime_binary_set(&exe_src, &staged_export_root, &game_exe_name).map_err(|e| {
            format!("Falha ao copiar executável/dependências do jogo: {}", e)
        })?;

        if project_json_src.exists() {
            fs::copy(project_json_src, staged_export_root.join("project.json"))
                .map_err(|e| format!("Falha ao copiar project.json: {}", e))?;
        }

        fs::write(
            staged_export_root.join(crate::standalone::STANDALONE_MARKER_FILE),
            b"standalone=true\n",
        )
        .map_err(|e| format!("Falha ao criar marcador standalone: {}", e))?;

        if assets_src.exists() {
            copy_dir_recursive(assets_src, &staged_export_root.join("assets"))
                .map_err(|e| format!("Falha ao copiar assets: {}", e))?;
        }

        if save_src.exists() {
            copy_dir_recursive(save_src, &staged_export_root.join("save"))
                .map_err(|e| format!("Falha ao copiar save: {}", e))?;
        }

        let readme = format!(
            "RS2BR Engine - Build Standalone PC\n\nProjeto: {}\nCena inicial: {}\nExecutável: {}\nModo: {}\n\nPara rodar:\n1. Deixe esta pasta inteira junta.\n2. Execute {}.\n\nRequisitos:\n- Não precisa de Rust ou Cargo.\n- Se o Windows reclamar de runtime C++, instale o Microsoft Visual C++ Redistributable 2015-2022 (x64).\n",
            project_name,
            saved_scene_name,
            game_exe_name,
            if use_prebuilt_runtime {
                "Runtime pré-compilado"
            } else {
                "Compilado via Cargo local"
            },
            game_exe_name,
        );
        fs::write(staged_export_root.join("LEIA-ME.txt"), readme)
            .map_err(|e| format!("Falha ao criar LEIA-ME.txt: {}", e))?;

        if export_root.exists() {
            fs::remove_dir_all(&export_root)
                .map_err(|e| format!("Falha ao limpar build anterior: {}", e))?;
        }
        fs::rename(&staged_export_root, &export_root)
            .map_err(|e| format!("Falha ao finalizar pasta da build: {}", e))?;

        let _ = fs::remove_dir_all(&build_cache_root);
        Ok(export_root)
    };

    run()
}
