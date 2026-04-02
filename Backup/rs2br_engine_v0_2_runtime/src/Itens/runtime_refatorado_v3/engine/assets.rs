// ============================================================
//  engine/assets.rs
//  Gerenciador de assets — lê o disco e mantém a árvore de
//  arquivos/pastas do projeto.
// ============================================================

use std::path::{Path, PathBuf};
use std::fs;

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

            // Pastas primeiro, depois arquivos, ambos em ordem alfabética
            kids.sort_by(|a, b| {
                b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
            });
            kids
        } else {
            Vec::new()
        };

        Some(AssetNode { name, path: path.to_path_buf(), is_dir, children })
    }

    /// Ícone emoji simples baseado no tipo/extensão
    pub fn icon(&self) -> &str {
        if self.is_dir {
            return "📁";
        }

        let lower_name = self.name.to_lowercase();
        if lower_name.ends_with(".matr.json") || lower_name.ends_with(".prefab.json") {
            return "🧱";
        }

        match self.path.extension().and_then(|e| e.to_str()) {
            Some("png") | Some("jpg") | Some("jpeg") | Some("webp") => "🖼",
            Some("rs")  => "⚙",
            Some("json") => "📄",
            Some("wav") | Some("ogg") | Some("mp3") => "🔊",
            Some("ttf") | Some("otf") => "🔤",
            _ => "📃",
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
        let mut mgr = Self { root: root.clone(), tree: None };
        mgr.refresh();
        mgr
    }

    /// Relê o disco e atualiza a árvore
    pub fn refresh(&mut self) {
        let assets_path = self.root.join("assets");
        // Garante que a pasta assets existe
        let _ = fs::create_dir_all(&assets_path);
        self.tree = AssetNode::from_path(&assets_path);
    }

    /// Cria uma nova pasta dentro de um diretório existente
    pub fn create_folder(&self, parent: &Path, name: &str) -> std::io::Result<()> {
        let new_dir = parent.join(name);
        fs::create_dir_all(new_dir)
    }

    /// Importa um arquivo externo para dentro de uma pasta de assets
    pub fn import_file(&self, source: &Path, target_dir: &Path) -> std::io::Result<PathBuf> {
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
    pub fn rename_path(&self, path: &Path, new_name: &str) -> std::io::Result<PathBuf> {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Nome não pode ficar vazio.",
            ));
        }

        let parent = path.parent().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Pasta pai não encontrada.")
        })?;

        let final_name = if path.is_file()
            && Path::new(trimmed).extension().is_none()
            && path.extension().is_some()
        {
            format!(
                "{}.{}",
                trimmed,
                path.extension().and_then(|e| e.to_str()).unwrap_or_default()
            )
        } else {
            trimmed.to_string()
        };

        let target = parent.join(final_name);
        fs::rename(path, &target)?;
        Ok(target)
    }

    /// Deleta um arquivo ou pasta do disco
    pub fn delete_path(&self, path: &Path) -> std::io::Result<()> {
        if !path.exists() {
            return Ok(());
        }

        if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }

    /// Cria um arquivo de script Rust template
    pub fn create_script(&self, parent: &Path, name: &str) -> std::io::Result<PathBuf> {
        let script_name = if name.ends_with(".rs") {
            name.to_string()
        } else {
            format!("{}.rs", name)
        };
        let path = parent.join(&script_name);
        let template = format!(
r#"// Script: {}
// Criado pelo Rust2D Engine
//
// Este script é anexado a uma entidade via componente Script.
// O runtime atual interpreta as diretivas abaixo durante o Play:
//
// @start_message Olá, mundo!
// @move_x 80
// @move_y 0
// @rotate_speed 45
// @player_controller 220
// @camera_follow
//
// Também são aceitos formatos estilo Rust dentro do arquivo:
// move_x = 80;
// rotate_speed = 45;
// player_controller = 220;

pub struct {} {{
    // Campos do script aqui
}}

impl {} {{
    /// Chamado uma vez quando o jogo começa
    pub fn start(&mut self) {{
        println!("Script '{}' iniciado!");
    }}

    /// Chamado todo frame
    pub fn update(&mut self, delta_time: f32) {{
        // delta_time = tempo em segundos desde o último frame
        // Exemplo de script compatível com o runtime atual:
        // move_x = 80;
        // rotate_speed = 45;
        let _ = delta_time;
    }}
}}
"#,
            script_name,
            to_pascal_case(name.trim_end_matches(".rs")),
            to_pascal_case(name.trim_end_matches(".rs")),
            script_name,
        );
        fs::write(&path, template)?;
        Ok(path)
    }

    /// Retorna o caminho relativo à pasta assets
    pub fn relative_path<'a>(&self, path: &'a Path) -> Option<&'a Path> {
        let assets = self.root.join("assets");
        path.strip_prefix(&assets).ok()
    }
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect()
}

fn make_unique_path(target_dir: &Path, source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("asset");
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");

    for index in 1..1000 {
        let candidate = if ext.is_empty() {
            target_dir.join(format!("{}_{}", stem, index))
        } else {
            target_dir.join(format!("{}_{}.{}", stem, index, ext))
        };

        if !candidate.exists() {
            return candidate;
        }
    }

    target_dir.join(format!("{}_copy", stem))
}
