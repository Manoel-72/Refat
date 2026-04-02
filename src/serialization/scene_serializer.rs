use std::io;
use std::path::Path;

use crate::core::scene::Scene;

pub fn scene_to_json(scene: &Scene) -> String {
    serde_json::to_string_pretty(scene).unwrap_or_else(|error| {
        eprintln!("Falha ao serializar cena '{}': {}", scene.name, error);
        String::new()
    })
}

pub fn try_scene_from_json(json: &str) -> Result<Scene, String> {
    serde_json::from_str(json).map_err(|error| format!("Falha ao desserializar cena: {}", error))
}

pub fn scene_from_json(json: &str) -> Option<Scene> {
    try_scene_from_json(json).map_err(|error| {
        eprintln!("{}", error);
        error
    }).ok()
}

pub fn save_scene_to_path(scene: &Scene, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, scene_to_json(scene))
}

pub fn try_load_scene_from_path(path: &Path) -> Result<Scene, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("Falha ao ler cena '{}': {}", path.display(), error))?;
    try_scene_from_json(&content)
}

pub fn load_scene_from_path(path: &Path) -> Option<Scene> {
    try_load_scene_from_path(path).ok()
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::scene::Scene;

    #[test]
    fn scene_roundtrip_works() {
        let scene = Scene::new("Teste");
        let json = scene_to_json(&scene);
        let loaded = try_scene_from_json(&json).expect("scene should deserialize");
        assert_eq!(loaded.name, scene.name);
    }
}
