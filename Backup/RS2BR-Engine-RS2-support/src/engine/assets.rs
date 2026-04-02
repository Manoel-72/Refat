// ============================================================
//  engine/assets.rs
//  Gerenciador de assets — lê o disco e mantém a árvore de
//  arquivos/pastas do projeto.
// ============================================================

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Folder,
    Scene,
    Sprite,
    ScriptRs2,
    Audio,
    Font,
    Prefab,
    Json,
    Unknown,
}

pub fn detect_asset_kind(path: &Path) -> AssetKind {
    if path.is_dir() {
        return AssetKind::Folder;
    }

    let lower_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if lower_name.ends_with(".scene.json") {
        return AssetKind::Scene;
    }
    if lower_name.ends_with(".matr.json") || lower_name.ends_with(".prefab.json") {
        return AssetKind::Prefab;
    }

    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("rs2") => AssetKind::ScriptRs2,
        Some("png") | Some("jpg") | Some("jpeg") | Some("webp") => AssetKind::Sprite,
        Some("wav") | Some("ogg") | Some("mp3") => AssetKind::Audio,
        Some("ttf") | Some("otf") => AssetKind::Font,
        Some("json") => AssetKind::Json,
        _ => AssetKind::Unknown,
    }
}

pub fn sanitize_asset_name(name: &str) -> String {
    let sanitized: String = name
        .trim()
        .replace(' ', "_")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        .collect();

    if sanitized.is_empty() {
        "novo_asset".to_string()
    } else {
        sanitized
    }
}

pub fn is_rs2_script_file(path: &Path) -> bool {
    matches!(detect_asset_kind(path), AssetKind::ScriptRs2)
}

/// Um nó na árvore de assets (arquivo ou pasta)
#[derive(Debug, Clone)]
pub struct AssetNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<AssetNode>,
}

impl AssetNode {
    /// Lê recursivamente uma pasta e constrói a árvore
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();

        let is_dir = path.is_dir();

        let children = if is_dir {
            let mut kids: Vec<AssetNode> = fs::read_dir(path)
                .ok()?
                .filter_map(|e| e.ok())
                .filter_map(|e| AssetNode::from_path(&e.path()))
                .collect();

            kids.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
            kids
        } else {
            Vec::new()
        };

        Some(AssetNode {
            name,
            path: path.to_path_buf(),
            is_dir,
            children,
        })
    }

    /// Ícone emoji simples baseado no tipo/extensão
    pub fn icon(&self) -> &str {
        match detect_asset_kind(&self.path) {
            AssetKind::Folder => "📁",
            AssetKind::Scene => "🎬",
            AssetKind::Sprite => "🖼",
            AssetKind::ScriptRs2 => "📜",
            AssetKind::Audio => "🔊",
            AssetKind::Font => "🔤",
            AssetKind::Prefab => "🧱",
            AssetKind::Json => "📄",
            AssetKind::Unknown => "📃",
        }
    }
}

/// Raiz do gerenciador de assets do projeto
pub struct AssetManager {
    /// Pasta raiz do projeto
    pub root: PathBuf,
    /// Árvore de arquivos lida do disco
    pub tree: Option<AssetNode>,
}

impl AssetManager {
    pub fn new(root: PathBuf) -> Self {
        let mut mgr = Self {
            root: root.clone(),
            tree: None,
        };
        mgr.refresh();
        mgr
    }

    /// Relê o disco e atualiza a árvore
    pub fn refresh(&mut self) {
        let assets_path = self.root.join("assets");
        let _ = fs::create_dir_all(assets_path.join("scripts"));
        let _ = fs::create_dir_all(assets_path.join("scenes"));
        let _ = fs::create_dir_all(assets_path.join("sprites"));
        self.tree = AssetNode::from_path(&assets_path);
    }

    /// Cria uma nova pasta dentro de um diretório existente
    pub fn create_folder(&self, parent: &Path, name: &str) -> io::Result<()> {
        let new_dir = parent.join(sanitize_asset_name(name));
        fs::create_dir_all(new_dir)
    }

    /// Importa um arquivo externo para dentro de uma pasta de assets
    pub fn import_file(&self, source: &Path, target_dir: &Path) -> io::Result<PathBuf> {
        fs::create_dir_all(target_dir)?;

        let file_name = source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("asset.bin");

        let mut destination = target_dir.join(file_name);
        if destination.exists() {
            destination = make_unique_path(target_dir, source);
        }

        fs::copy(source, &destination)?;
        Ok(destination)
    }

    /// Renomeia um arquivo ou pasta existente dentro de /assets
    pub fn rename_path(&self, path: &Path, new_name: &str) -> io::Result<PathBuf> {
        let trimmed = sanitize_asset_name(new_name);
        if trimmed.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Nome não pode ficar vazio.",
            ));
        }

        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Pasta pai não encontrada.")
        })?;

        let final_name = if path.is_file()
            && Path::new(&trimmed).extension().is_none()
            && path.extension().is_some()
        {
            format!(
                "{}.{}",
                trimmed,
                path.extension().and_then(|e| e.to_str()).unwrap_or_default()
            )
        } else {
            trimmed
        };

        let target = parent.join(final_name);
        fs::rename(path, &target)?;
        Ok(target)
    }

    /// Deleta um arquivo ou pasta do disco
    pub fn delete_path(&self, path: &Path) -> io::Result<()> {
        if !path.exists() {
            return Ok(());
        }

        if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }

    /// Cria um arquivo de script RS2 template.
    pub fn create_rs2_script_file(&self, parent: &Path, name: &str) -> io::Result<PathBuf> {
        fs::create_dir_all(parent)?;

        let base_name = name.trim_end_matches(".rs2").trim_end_matches(".rs");
        let sanitized = sanitize_asset_name(base_name);
        let final_name = format!("{sanitized}.rs2");

        let path = parent.join(&final_name);
        if path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Já existe um script RS2 com esse nome.",
            ));
        }

        let template = "// Script RS2BR-Engine\n@start_message Novo script\n";
        fs::write(&path, template)?;
        Ok(path)
    }

    /// Compatibilidade com código antigo.
    pub fn create_script(&self, parent: &Path, name: &str) -> io::Result<PathBuf> {
        self.create_rs2_script_file(parent, name)
    }

    /// Retorna o caminho relativo à pasta assets
    pub fn relative_path<'a>(&self, path: &'a Path) -> Option<&'a Path> {
        let assets = self.root.join("assets");
        path.strip_prefix(&assets).ok()
    }
}

fn make_unique_path(target_dir: &Path, source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("asset");
    let extension = source.extension().and_then(|e| e.to_str()).unwrap_or_default();

    for index in 1..10_000 {
        let candidate_name = if extension.is_empty() {
            format!("{}_{}", stem, index)
        } else {
            format!("{}_{}.{}", stem, index, extension)
        };

        let candidate = target_dir.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    target_dir.join(source.file_name().unwrap_or_default())
}
