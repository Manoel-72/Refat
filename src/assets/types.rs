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
        let load_status = detect_load_status(&path);
        let validation = validate_asset_path(&path, asset_type);
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
    if !path.exists() {
        return AssetLoadStatus::Missing;
    }
    if path.is_dir() {
        return AssetLoadStatus::Ready;
    }

    match detect_asset_type(path) {
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
        AssetType::Scene => {
            if std::fs::read_to_string(path).is_err() {
                AssetValidation::issue("Cena não pôde ser lida.")
            } else {
                AssetValidation::ok()
            }
        }
        AssetType::Prefab => {
            if std::fs::read_to_string(path).is_err() {
                AssetValidation::issue("Prefab não pôde ser lido.")
            } else {
                AssetValidation::ok()
            }
        }
        AssetType::Texture | AssetType::Audio | AssetType::Font | AssetType::ScriptRs2 | AssetType::Json => {
            if path.is_file() { AssetValidation::ok() } else { AssetValidation::issue("O asset esperado não é um arquivo válido.") }
        }
        AssetType::Unknown => AssetValidation::issue("Tipo de asset desconhecido ou não suportado."),
    }
}
