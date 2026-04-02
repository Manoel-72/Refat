use std::path::Path;

use crate::core::prefab::Prefab;

pub fn prefab_to_json(prefab: &Prefab) -> String {
    serde_json::to_string_pretty(prefab).unwrap_or_default()
}

pub fn prefab_from_json(json: &str) -> Option<Prefab> {
    serde_json::from_str(json).ok()
}

pub fn save_prefab_to_path(prefab: &Prefab, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, prefab_to_json(prefab))
}

pub fn load_prefab_from_path(path: &Path) -> Option<Prefab> {
    let content = std::fs::read_to_string(path).ok()?;
    prefab_from_json(&content)
}
