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
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Err("Falha ao desserializar prefab: JSON vazio.".to_string());
    }

    serde_json::from_str(trimmed).map_err(|error| format!("Falha ao desserializar prefab: {}", error))
}

pub fn prefab_from_json(json: &str) -> Option<Prefab> {
    try_prefab_from_json(json).ok()
}

pub fn save_prefab_to_path(prefab: &Prefab, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = prefab_to_json(prefab);
    if json.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Falha ao serializar prefab '{}'.", prefab.name),
        ));
    }

    std::fs::write(path, json)
}

pub fn try_load_prefab_from_path(path: &Path) -> Result<Prefab, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("Falha ao ler prefab '{}': {}", path.display(), error))?;
    try_prefab_from_json(&content)
        .map_err(|error| format!("{} (arquivo: {})", error, path.display()))
}

pub fn load_prefab_from_path(path: &Path) -> Option<Prefab> {
    try_load_prefab_from_path(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{entity::Entity, prefab::Prefab};

    #[test]
    fn prefab_roundtrip_works() {
        let prefab = Prefab::new("Teste", Entity::new("Entidade"));
        let json = prefab_to_json(&prefab);
        let loaded = try_prefab_from_json(&json).expect("prefab should deserialize");
        assert_eq!(loaded.name, prefab.name);
    }

    #[test]
    fn invalid_prefab_json_reports_error() {
        let result = try_prefab_from_json("{ invalid json }");
        assert!(result.is_err());
    }
}
