use std::io;
use std::path::Path;

use crate::core::prefab::Prefab;

pub fn prefab_to_json(prefab: &Prefab) -> String {
    serde_json::to_string_pretty(prefab).unwrap_or_else(|error| {
        eprintln!("Falha ao serializar prefab '{}': {}", prefab.name, error);
        String::new()
    })
}

pub fn try_prefab_from_json(json: &str) -> Result<Prefab, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| format!("Falha ao desserializar prefab: {}", error))?;

    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("prefab")
        .to_string();

    let root_entity = value
        .get("root_entity")
        .cloned()
        .ok_or_else(|| "Falha ao desserializar prefab: campo 'root_entity' ausente.".to_string())?;

    serde_json::from_value(serde_json::json!({
        "name": name,
        "root_entity": root_entity,
    }))
    .map_err(|error| format!("Falha ao desserializar prefab: {}", error))
}

pub fn prefab_from_json(json: &str) -> Option<Prefab> {
    try_prefab_from_json(json).ok()
}

pub fn save_prefab_to_path(prefab: &Prefab, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, prefab_to_json(prefab))
}

pub fn try_load_prefab_from_path(path: &Path) -> Result<Prefab, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("Falha ao ler prefab '{}': {}", path.display(), error))?;
    try_prefab_from_json(&content)
}

pub fn load_prefab_from_path(path: &Path) -> Option<Prefab> {
    try_load_prefab_from_path(path).ok()
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{entity::Entity, prefab::Prefab};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_prefab_path() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("rs2br_prefab_serializer_{}", unique))
            .join("assets/prefabs/teste.prefab.json")
    }

    #[test]
    fn prefab_roundtrip_works() {
        let prefab = Prefab::new("Teste", Entity::new("Entidade"));
        let json = prefab_to_json(&prefab);
        let loaded = try_prefab_from_json(&json).expect("prefab should deserialize");
        assert_eq!(loaded.name, prefab.name);
    }

    #[test]
    fn invalid_prefab_json_fails_cleanly() {
        let error = try_prefab_from_json("{ invalido }").unwrap_err();
        assert!(error.contains("Falha ao desserializar prefab"));
    }

    #[test]
    fn missing_root_entity_fails_with_clear_message() {
        let error = try_prefab_from_json(r#"{"name":"SemRaiz"}"#).unwrap_err();
        assert!(error.contains("root_entity"));
    }

    #[test]
    fn save_and_load_prefab_path_work() {
        let path = temp_prefab_path();
        let prefab = Prefab::new("TesteSalvar", Entity::new("EntidadeSalvar"));
        save_prefab_to_path(&prefab, &path).unwrap();

        let loaded = try_load_prefab_from_path(&path).unwrap();
        assert_eq!(loaded.name, "TesteSalvar");

        let root = path.parent().unwrap().parent().unwrap().parent().unwrap();
        let _ = std::fs::remove_dir_all(root);
    }
}
