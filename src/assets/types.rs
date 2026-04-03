use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetType {
    Folder,
    Scene,
    Prefab,
    Texture,
    Audio,
    Font,
    ScriptRs2,
    Json,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetLoadStatus {
    NotLoaded,
    Ready,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetValidation {
    pub is_valid: bool,
    pub message: String,
}

impl AssetValidation {
    pub fn ok() -> Self {
        Self { is_valid: true, message: "OK".to_string() }
    }

    pub fn issue(message: impl Into<String>) -> Self {
        Self { is_valid: false, message: message.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub asset_type: AssetType,
    pub load_status: AssetLoadStatus,
    pub validation: AssetValidation,
}

impl AssetRecord {
    pub fn new(path: PathBuf, asset_type: AssetType) -> Self {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("asset").to_string();
        let validation = validate_asset_path(&path, asset_type);
        let load_status = detect_load_status_with_validation(&path, asset_type, &validation);
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            path,
            asset_type,
            load_status,
            validation,
        }
    }
}

pub fn detect_asset_type(path: &Path) -> AssetType {
    if path.is_dir() {
        return AssetType::Folder;
    }

    let lower_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_ascii_lowercase();
    if lower_name.ends_with(".scene.json") {
        return AssetType::Scene;
    }
    if lower_name.ends_with(".prefab.json") || lower_name.ends_with(".matr.json") {
        return AssetType::Prefab;
    }

    match path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()).as_deref() {
        Some("rs2") => AssetType::ScriptRs2,
        Some("png") | Some("jpg") | Some("jpeg") | Some("webp") => AssetType::Texture,
        Some("wav") | Some("ogg") | Some("mp3") => AssetType::Audio,
        Some("ttf") | Some("otf") => AssetType::Font,
        Some("json") => AssetType::Json,
        _ => AssetType::Unknown,
    }
}

pub fn detect_load_status(path: &Path) -> AssetLoadStatus {
    let asset_type = detect_asset_type(path);
    let validation = validate_asset_path(path, asset_type);
    detect_load_status_with_validation(path, asset_type, &validation)
}

pub fn detect_load_status_with_validation(
    path: &Path,
    asset_type: AssetType,
    validation: &AssetValidation,
) -> AssetLoadStatus {
    if !path.exists() {
        return AssetLoadStatus::Missing;
    }
    if path.is_dir() {
        return AssetLoadStatus::Ready;
    }
    if !validation.is_valid {
        return AssetLoadStatus::Invalid;
    }

    match asset_type {
        AssetType::Unknown => AssetLoadStatus::Invalid,
        _ => AssetLoadStatus::Ready,
    }
}

pub fn validate_asset_path(path: &Path, asset_type: AssetType) -> AssetValidation {
    if !path.exists() {
        return AssetValidation::issue("Arquivo ou pasta não encontrado no disco.");
    }

    match asset_type {
        AssetType::Folder => AssetValidation::ok(),
        AssetType::Scene => validate_scene_asset(path),
        AssetType::Prefab => validate_prefab_asset(path),
        AssetType::Texture => validate_texture_asset(path),
        AssetType::ScriptRs2 => validate_script_asset(path),
        AssetType::Audio => validate_regular_file(path, "Áudio inválido ou inacessível."),
        AssetType::Font => validate_regular_file(path, "Fonte inválida ou inacessível."),
        AssetType::Json => validate_json_asset(path),
        AssetType::Unknown => AssetValidation::issue("Tipo de asset desconhecido ou não suportado."),
    }
}

pub fn validate_reference_path(project_root: &Path, raw_path: &str, expected: AssetType) -> AssetValidation {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return AssetValidation::issue("Referência vazia.");
    }

    let normalized = trimmed.replace('\\', "/");
    let candidates = [
        project_root.join(&normalized),
        project_root.join("assets").join(&normalized),
        project_root.join("assets/scripts").join(&normalized),
        project_root.join("assets/prefabs").join(&normalized),
        project_root.join("assets/sprites").join(&normalized),
    ];

    for candidate in candidates {
        if candidate.exists() {
            let actual_type = detect_asset_type(&candidate);
            if expected != AssetType::Unknown && actual_type != expected {
                return AssetValidation::issue(format!(
                    "Referência aponta para {:?}, mas o esperado era {:?}.",
                    actual_type, expected
                ));
            }
            return validate_asset_path(&candidate, actual_type);
        }
    }

    AssetValidation::issue(format!("Arquivo referenciado não encontrado: {}", trimmed))
}

fn validate_regular_file(path: &Path, message: &str) -> AssetValidation {
    if path.is_file() {
        AssetValidation::ok()
    } else {
        AssetValidation::issue(message)
    }
}

fn validate_scene_asset(path: &Path) -> AssetValidation {
    if !path.is_file() {
        return AssetValidation::issue("Cena inválida: o caminho não aponta para um arquivo.");
    }

    let Ok(content) = std::fs::read_to_string(path) else {
        return AssetValidation::issue("Cena não pôde ser lida.");
    };

    match crate::serialization::scene_serializer::try_scene_from_json(&content) {
        Ok(_) => AssetValidation::ok(),
        Err(error) => AssetValidation::issue(error),
    }
}

fn validate_prefab_asset(path: &Path) -> AssetValidation {
    if !path.is_file() {
        return AssetValidation::issue("Prefab inválido: o caminho não aponta para um arquivo.");
    }

    let Ok(content) = std::fs::read_to_string(path) else {
        return AssetValidation::issue("Prefab não pôde ser lido.");
    };

    let prefab_result = crate::serialization::prefab_serializer::try_prefab_from_json(&content);
    if prefab_result.is_ok() || crate::core::entity::Entity::from_json(&content).is_some() {
        AssetValidation::ok()
    } else {
        AssetValidation::issue(prefab_result.err().unwrap_or_else(|| {
            "O JSON do prefab não pôde ser interpretado como Prefab nem como Entity legacy.".to_string()
        }))
    }
}

fn validate_texture_asset(path: &Path) -> AssetValidation {
    if !path.is_file() {
        return AssetValidation::issue("Textura inválida: o caminho não aponta para um arquivo.");
    }

    match image::ImageReader::open(path) {
        Ok(reader) => match reader.with_guessed_format() {
            Ok(reader) => match reader.decode() {
                Ok(_) => AssetValidation::ok(),
                Err(error) => AssetValidation::issue(format!("Imagem quebrada ou formato inválido: {}", error)),
            },
            Err(error) => AssetValidation::issue(format!("Não foi possível detectar o formato da imagem: {}", error)),
        },
        Err(error) => AssetValidation::issue(format!("Textura não pôde ser aberta: {}", error)),
    }
}

fn validate_script_asset(path: &Path) -> AssetValidation {
    if !path.is_file() {
        return AssetValidation::issue("Script inválido: o caminho não aponta para um arquivo.");
    }

    let Ok(source) = std::fs::read_to_string(path) else {
        return AssetValidation::issue("Script não pôde ser lido.");
    };

    let errors = crate::runtime::script::validate_script(&source);
    if errors.is_empty() {
        AssetValidation::ok()
    } else {
        AssetValidation::issue(crate::runtime::script::format_script_errors(&errors).join(" | "))
    }
}

fn validate_json_asset(path: &Path) -> AssetValidation {
    if !path.is_file() {
        return AssetValidation::issue("JSON inválido: o caminho não aponta para um arquivo.");
    }

    let Ok(content) = std::fs::read_to_string(path) else {
        return AssetValidation::issue("JSON não pôde ser lido.");
    };

    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(_) => AssetValidation::ok(),
        Err(error) => AssetValidation::issue(format!("JSON inválido: {}", error)),
    }
}
