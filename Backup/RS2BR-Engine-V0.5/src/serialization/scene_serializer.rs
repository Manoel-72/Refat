use std::path::Path;

use crate::core::scene::Scene;

pub fn scene_to_json(scene: &Scene) -> String {
    serde_json::to_string_pretty(scene).unwrap_or_default()
}

pub fn scene_from_json(json: &str) -> Option<Scene> {
    serde_json::from_str(json).ok()
}

pub fn save_scene_to_path(scene: &Scene, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, scene_to_json(scene))
}

pub fn load_scene_from_path(path: &Path) -> Option<Scene> {
    let content = std::fs::read_to_string(path).ok()?;
    scene_from_json(&content)
}
