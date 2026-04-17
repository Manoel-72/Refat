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
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| format!("Falha ao desserializar cena: {}", error))?;

    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("scene")
        .to_string();

    let entities = value
        .get("entities")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    let background_color = value
        .get("background_color")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([0.15, 0.15, 0.18]));

    serde_json::from_value(serde_json::json!({
        "name": name,
        "entities": entities,
        "background_color": background_color,
    }))
    .map_err(|error| format!("Falha ao desserializar cena: {}", error))
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_scene_path() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("rs2br_scene_serializer_{}", unique))
            .join("assets/scenes/teste.scene.json")
    }

    #[test]
    fn scene_roundtrip_works() {
        let scene = Scene::new("Teste");
        let json = scene_to_json(&scene);
        let loaded = try_scene_from_json(&json).expect("scene should deserialize");
        assert_eq!(loaded.name, scene.name);
    }

    #[test]
    fn invalid_scene_json_fails_cleanly() {
        let error = try_scene_from_json("{ invalido }").unwrap_err();
        assert!(error.contains("Falha ao desserializar cena"));
    }

    #[test]
    fn scene_defaults_missing_fields_safely() {
        let loaded = try_scene_from_json(r#"{"name":"SemCampos"}"#).unwrap();
        assert_eq!(loaded.name, "SemCampos");
        assert!(loaded.entities.is_empty());
        assert_eq!(loaded.background_color, [0.15, 0.15, 0.18]);
    }

    #[test]
    fn save_and_load_scene_path_work() {
        let path = temp_scene_path();
        let scene = Scene::new("SalvarCena");
        save_scene_to_path(&scene, &path).unwrap();

        let loaded = try_load_scene_from_path(&path).unwrap();
        assert_eq!(loaded.name, "SalvarCena");

        let root = path.parent().unwrap().parent().unwrap().parent().unwrap();
        let _ = std::fs::remove_dir_all(root);
    }
}
