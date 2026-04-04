use std::{fs, io, path::Path};

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
