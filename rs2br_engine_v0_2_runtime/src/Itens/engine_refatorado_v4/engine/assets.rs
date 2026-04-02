use std::fs;
use std::path::{Component as PathComponent, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AssetNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<AssetNode>,
}

impl AssetNode {
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("assets")
            .to_string();

        let is_dir = path.is_dir();
        let children = if is_dir {
            let mut kids: Vec<AssetNode> = fs::read_dir(path)
                .ok()?
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| AssetNode::from_path(&entry.path()))
                .collect();

            kids.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
            kids
        } else {
            Vec::new()
        };

        Some(Self {
            name,
            path: path.to_path_buf(),
            is_dir,
            children,
        })
    }

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
            Some("rs") => "⚙",
            Some("json") => "📄",
            Some("wav") | Some("ogg") | Some("mp3") => "🔊",
            Some("ttf") | Some("otf") => "🔤",
            _ => "📃",
        }
    }
}

pub struct AssetManager {
    pub root: PathBuf,
    pub tree: Option<AssetNode>,
}

impl AssetManager {
    pub fn new(root: PathBuf) -> Self {
        let mut manager = Self { root, tree: None };
        manager.refresh();
        manager
    }

    pub fn assets_root(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn refresh(&mut self) {
        let assets_path = self.assets_root();
        let _ = fs::create_dir_all(&assets_path);
        self.tree = AssetNode::from_path(&assets_path);
    }

    pub fn create_folder(&self, parent: &Path, name: &str) -> std::io::Result<()> {
        let safe_name = sanitize_file_name(name)?;
        let parent = self.ensure_inside_assets(parent)?;
        fs::create_dir_all(parent.join(safe_name))
    }

    pub fn import_file(&self, source: &Path, target_dir: &Path) -> std::io::Result<PathBuf> {
        let target_dir = self.ensure_inside_assets(target_dir)?;
        fs::create_dir_all(&target_dir)?;

        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("asset.bin");

        let mut destination = target_dir.join(file_name);
        if destination.exists() {
            destination = make_unique_path(&target_dir, source);
        }

        fs::copy(source, &destination)?;
        Ok(destination)
    }

    pub fn rename_path(&self, path: &Path, new_name: &str) -> std::io::Result<PathBuf> {
        let path = self.ensure_inside_assets(path)?;
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
                sanitize_file_name(trimmed)?,
                path.extension().and_then(|e| e.to_str()).unwrap_or_default()
            )
        } else {
            sanitize_file_name(trimmed)?
        };

        let target = parent.join(final_name);
        fs::rename(path, &target)?;
        Ok(target)
    }

    pub fn delete_path(&self, path: &Path) -> std::io::Result<()> {
        let path = self.ensure_inside_assets(path)?;

        if !path.exists() {
            return Ok(());
        }

        if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }

    pub fn create_script(&self, parent: &Path, name: &str) -> std::io::Result<PathBuf> {
        let parent = self.ensure_inside_assets(parent)?;
        let base_name = sanitize_file_name(name)?;
        let script_name = if base_name.ends_with(".rs") {
            base_name
        } else {
            format!("{}.rs", base_name)
        };

        let path = parent.join(&script_name);
        let struct_name = to_pascal_case(script_name.trim_end_matches(".rs"));
        let template = format!(
            r#"// Script: {script_name}
// Criado pelo Rust2D Engine
//
// Diretivas reconhecidas pelo runtime atual:
// @start_message Olá, mundo!
// @move_x 80
// @move_y 0
// @rotate_speed 45
// @player_controller 220
// @camera_follow

pub struct {struct_name} {{}}

impl {struct_name} {{
    pub fn start(&mut self) {{
        println!(\"Script '{script_name}' iniciado!\");
    }}

    pub fn update(&mut self, delta_time: f32) {{
        let _ = delta_time;
    }}
}}
"#
        );
        fs::write(&path, template)?;
        Ok(path)
    }

    pub fn relative_path<'a>(&self, path: &'a Path) -> Option<&'a Path> {
        let assets = self.assets_root();
        path.strip_prefix(&assets).ok()
    }

    fn ensure_inside_assets(&self, path: &Path) -> std::io::Result<PathBuf> {
        let assets = self.assets_root();
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            assets.join(path)
        };

        if contains_parent_dir(&candidate) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Caminho inválido.",
            ));
        }

        if candidate.starts_with(&assets) {
            Ok(candidate)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Operação permitida apenas dentro da pasta assets.",
            ))
        }
    }
}

fn sanitize_file_name(name: &str) -> std::io::Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Nome não pode ficar vazio.",
        ));
    }

    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Nome de arquivo/pasta inválido.",
        ));
    }

    Ok(trimmed.to_string())
}

fn contains_parent_dir(path: &Path) -> bool {
    path.components().any(|component| matches!(component, PathComponent::ParentDir))
}

fn to_pascal_case(value: &str) -> String {
    value
        .split('_')
        .flat_map(|part| part.split('-'))
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
