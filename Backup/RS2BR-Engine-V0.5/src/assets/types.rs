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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub asset_type: AssetType,
}

impl AssetRecord {
    pub fn new(path: PathBuf, asset_type: AssetType) -> Self {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("asset").to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            path,
            asset_type,
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
