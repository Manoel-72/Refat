use std::{fs, io, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub initial_scene: String,
    #[serde(default)]
    pub engine_version: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "RS2BR Project".to_string(),
            initial_scene: "assets/scenes/main.scene.json".to_string(),
            engine_version: crate::core::version::ENGINE_VERSION.to_string(),
        }
    }
}

impl ProjectConfig {
    pub fn load_or_create(project_root: &Path) -> Self {
        let path = project_root.join("project.json");
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<ProjectConfig>(&content) {
                return config;
            }
        }

        let config = ProjectConfig::default();
        let _ = save_project_config(project_root, &config);
        config
    }
}

pub fn save_project_config(project_root: &Path, config: &ProjectConfig) -> io::Result<()> {
    let path = project_root.join("project.json");
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    fs::write(path, json)
}


pub fn create_basic_project_template_at(project_root: &Path, project_name: &str) -> io::Result<PathBuf> {
    let name = if project_name.trim().is_empty() {
        "MeuProjeto"
    } else {
        project_name.trim()
    };

    let assets_root = project_root.join("assets");
    let scenes_dir = assets_root.join("scenes");
    let scripts_dir = assets_root.join("scripts");
    fs::create_dir_all(&scenes_dir)?;
    fs::create_dir_all(&scripts_dir)?;
    fs::create_dir_all(assets_root.join("sprites"))?;
    fs::create_dir_all(assets_root.join("sounds"))?;
    fs::create_dir_all(assets_root.join("fonts"))?;
    fs::create_dir_all(assets_root.join("prefabs"))?;
    fs::create_dir_all(project_root.join("save"))?;

    let scene_file = format!("{}.scene.json", sanitize_project_name(name));
    let scene_path = scenes_dir.join(&scene_file);
    if !scene_path.exists() {
        let scene = crate::core::scene::Scene::new(name);
        crate::serialization::scene_serializer::save_scene_to_path(&scene, &scene_path)?;
    }

    let config = ProjectConfig {
        name: name.to_string(),
        initial_scene: format!("assets/scenes/{}", scene_file),
        engine_version: crate::core::version::ENGINE_VERSION.to_string(),
    };
    save_project_config(project_root, &config)?;

    let lua_path = scripts_dir.join("player_base.lua");
    if !lua_path.exists() {
        let template = concat!(
            "-- Script Lua — RS2BR-Engine V", env!("CARGO_PKG_VERSION"), "
",
            "--
",
            "-- Estrutura mínima recomendada.
",
            "-- Use on_start para inicialização e on_update(dt) para lógica por frame.
",
            "--

",
            "function on_start()
",
            "end

",
            "function on_update(dt)
",
            "end
"
        );
        fs::write(lua_path, template)?;
    }

    Ok(scene_path)
}

fn sanitize_project_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    if cleaned.trim_matches('_').is_empty() {
        "MeuProjeto".to_string()
    } else {
        cleaned
    }
}
