// ============================================================
//  runtime/save.rs  —  V0.9
//  Save/Load de estado de jogo.
//  Dados do jogo (placar, posição, flags, etc.) persistem em
//  <projeto>/save/save.json como pares chave→valor JSON.
// ============================================================

use std::{collections::HashMap, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};

/// Valor que pode ser salvo: número, texto ou booleano.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SaveValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

impl SaveValue {
    pub fn as_bool(&self)  -> Option<bool>   { if let SaveValue::Bool(v)  = self { Some(*v) } else { None } }
    pub fn as_int(&self)   -> Option<i64>    { if let SaveValue::Int(v)   = self { Some(*v) } else { None } }
    pub fn as_float(&self) -> Option<f64>    { if let SaveValue::Float(v) = self { Some(*v) } else { None } }
    pub fn as_text(&self)  -> Option<&str>   { if let SaveValue::Text(v)  = self { Some(v) } else { None } }

    /// Tenta converter para f64 independente de variante numérica.
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            SaveValue::Int(v)   => Some(*v as f64),
            SaveValue::Float(v) => Some(*v),
            _ => None,
        }
    }
}

impl From<bool>  for SaveValue { fn from(v: bool)  -> Self { SaveValue::Bool(v) } }
impl From<i32>   for SaveValue { fn from(v: i32)   -> Self { SaveValue::Int(v as i64) } }
impl From<i64>   for SaveValue { fn from(v: i64)   -> Self { SaveValue::Int(v) } }
impl From<f32>   for SaveValue { fn from(v: f32)   -> Self { SaveValue::Float(v as f64) } }
impl From<f64>   for SaveValue { fn from(v: f64)   -> Self { SaveValue::Float(v) } }
impl From<&str>  for SaveValue { fn from(v: &str)  -> Self { SaveValue::Text(v.to_string()) } }
impl From<String>for SaveValue { fn from(v: String)-> Self { SaveValue::Text(v) } }

// ── SaveData ─────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SaveData {
    /// Campos do jogo: chave → valor.
    #[serde(default)]
    pub entries: HashMap<String, SaveValue>,
    /// Cena onde o jogo foi salvo (para retomar no mesmo ponto).
    #[serde(default)]
    pub current_scene: Option<String>,
    /// Versão do save para migração futura.
    #[serde(default = "default_save_version")]
    pub version: u32,
}

fn default_save_version() -> u32 { 1 }

impl SaveData {
    pub fn new() -> Self {
        Self { version: default_save_version(), ..Default::default() }
    }

    // ── get ──────────────────────────────────────────────────

    pub fn get(&self, key: &str) -> Option<&SaveValue> {
        self.entries.get(key)
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.entries.get(key)?.as_bool()
    }

    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.entries.get(key)?.as_int()
    }

    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.entries.get(key)?.to_f64()
    }

    pub fn get_text<'a>(&'a self, key: &str) -> Option<&'a str> {
        self.entries.get(key)?.as_text()
    }

    // ── set ──────────────────────────────────────────────────

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<SaveValue>) {
        self.entries.insert(key.into(), value.into());
    }

    pub fn set_text(&mut self, key: &str, value: impl Into<String>) {
        self.entries.insert(key.to_string(), SaveValue::Text(value.into()));
    }

    pub fn set_float(&mut self, key: &str, value: f64) {
        self.entries.insert(key.to_string(), SaveValue::Float(value));
    }

    pub fn set_int(&mut self, key: &str, value: i64) {
        self.entries.insert(key.to_string(), SaveValue::Int(value));
    }

    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.entries.insert(key.to_string(), SaveValue::Bool(value));
    }

    pub fn remove(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    pub fn has(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    // ── I/O ──────────────────────────────────────────────────

    /// Salva em `<project_root>/save/save.json`.
    pub fn save_to_project(&self, project_root: &Path) -> Result<(), String> {
        let path = save_path(project_root);
        self.save_to_path(&path)
    }

    /// Carrega de `<project_root>/save/save.json`.
    /// Retorna `None` se o arquivo não existe (primeira vez).
    pub fn load_from_project(project_root: &Path) -> Option<Self> {
        let path = save_path(project_root);
        Self::load_from_path(&path).ok()
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Erro ao criar diretório de save: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Erro ao serializar save: {e}"))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Erro ao gravar save em '{}': {e}", path.display()))
    }

    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("Erro ao ler save em '{}': {e}", path.display()))?;
        serde_json::from_str(&raw)
            .map_err(|e| format!("Erro ao desserializar save: {e}"))
    }

    /// Apaga o arquivo de save.
    pub fn delete_save(project_root: &Path) -> bool {
        let path = save_path(project_root);
        std::fs::remove_file(path).is_ok()
    }
}

fn save_path(project_root: &Path) -> PathBuf {
    project_root.join("save").join("save.json")
}

// ── testes ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_round_trip() {
        let mut data = SaveData::new();
        data.set("score", 42_i32);
        data.set("nome", "Player");
        data.set("ativo", true);
        data.set("x_pos", 3.14_f32);

        assert_eq!(data.get_int("score"), Some(42));
        assert_eq!(data.get_text("nome"), Some("Player"));
        assert_eq!(data.get_bool("ativo"), Some(true));
        assert!((data.get_float("x_pos").unwrap() - 3.14).abs() < 0.01);
    }

    #[test]
    fn save_load_roundtrip_temp() {
        let dir = std::env::temp_dir().join("rs2br_save_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut data = SaveData::new();
        data.set("fase", 3_i32);
        data.set("vidas", 2_i32);
        data.current_scene = Some("fases/fase3.scene.json".to_string());

        data.save_to_project(&dir).expect("save falhou");

        let loaded = SaveData::load_from_project(&dir).expect("load falhou");
        assert_eq!(loaded.get_int("fase"), Some(3));
        assert_eq!(loaded.get_int("vidas"), Some(2));
        assert_eq!(loaded.current_scene.as_deref(), Some("fases/fase3.scene.json"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_entry() {
        let mut data = SaveData::new();
        data.set("key", "valor");
        assert!(data.has("key"));
        assert!(data.remove("key"));
        assert!(!data.has("key"));
    }
}
