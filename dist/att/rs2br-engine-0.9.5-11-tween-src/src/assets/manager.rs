// ============================================================
//  assets/manager.rs
//  Gerenciador de assets — lê o disco, mantém a árvore e
//  oferece operações de produção do projeto.
// ============================================================

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::project::create_basic_project_template_at;

use super::types::{detect_asset_type, AssetRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Folder,
    Scene,
    Sprite,
    ScriptRs2,
    ScriptLua,
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
        Some("lua") => AssetKind::ScriptLua,
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

pub fn is_lua_script_file(path: &Path) -> bool {
    matches!(detect_asset_kind(path), AssetKind::ScriptLua)
}

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

    pub fn icon(&self) -> &str {
        match detect_asset_kind(&self.path) {
            AssetKind::Folder => "📁",
            AssetKind::Scene => "🎬",
            AssetKind::Sprite => "🖼",
            AssetKind::ScriptRs2 => "📜",
            AssetKind::ScriptLua => "🌙",
            AssetKind::Audio => "🔊",
            AssetKind::Font => "🔤",
            AssetKind::Prefab => "🧱",
            AssetKind::Json => "📄",
            AssetKind::Unknown => "📃",
        }
    }
}

pub struct AssetManager {
    pub root: PathBuf,
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

    pub fn refresh(&mut self) {
        let assets_path = self.root.join("assets");
        let _ = fs::create_dir_all(assets_path.join("scripts"));
        let _ = fs::create_dir_all(assets_path.join("scenes"));
        let _ = fs::create_dir_all(assets_path.join("sprites"));
        let _ = fs::create_dir_all(assets_path.join("sounds"));
        let _ = fs::create_dir_all(assets_path.join("fonts"));
        let _ = fs::create_dir_all(assets_path.join("prefabs"));
        self.tree = AssetNode::from_path(&assets_path);
    }

    pub fn list_asset_records(&self) -> Vec<AssetRecord> {
        let mut records = Vec::new();
        let assets_root = self.root.join("assets");
        collect_records(&assets_root, &mut records);
        records.sort_by(|a, b| a.name.cmp(&b.name));
        records
    }

    pub fn asset_record_for(&self, path: &Path) -> AssetRecord {
        AssetRecord::new(path.to_path_buf(), detect_asset_type(path))
    }

    pub fn create_folder(&self, parent: &Path, name: &str) -> io::Result<()> {
        let new_dir = parent.join(sanitize_asset_name(name));
        fs::create_dir_all(new_dir)
    }

    pub fn create_scene_file(&self, parent: &Path, name: &str) -> io::Result<PathBuf> {
        fs::create_dir_all(parent)?;
        let sanitized = sanitize_asset_name(name.trim_end_matches(".scene.json"));
        let path = parent.join(format!("{}.scene.json", sanitized));
        if path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Já existe uma cena com esse nome.",
            ));
        }
        let scene = crate::core::scene::Scene::new(&sanitized);
        crate::serialization::scene_serializer::save_scene_to_path(&scene, &path)?;
        Ok(path)
    }

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

    pub fn rename_path(&self, path: &Path, new_name: &str) -> io::Result<PathBuf> {
        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "O asset selecionado não existe mais no disco.",
            ));
        }

        let trimmed = sanitize_asset_name(new_name);
        if trimmed.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Nome não pode ficar vazio.",
            ));
        }

        let parent = path
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Pasta pai não encontrada."))?;
        let current_file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();

        let final_name = if path.is_file() {
            if current_file_name.ends_with(".scene.json") {
                format!("{}.scene.json", trimmed.trim_end_matches(".scene.json"))
            } else if current_file_name.ends_with(".prefab.json") {
                format!("{}.prefab.json", trimmed.trim_end_matches(".prefab.json"))
            } else if current_file_name.ends_with(".matr.json") {
                format!("{}.matr.json", trimmed.trim_end_matches(".matr.json"))
            } else if Path::new(&trimmed).extension().is_none() && path.extension().is_some() {
                format!(
                    "{}.{}",
                    trimmed,
                    path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or_default()
                )
            } else {
                trimmed.clone()
            }
        } else {
            trimmed.clone()
        };

        let target = parent.join(final_name);
        if target == path {
            return Ok(target);
        }
        if target.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Já existe um asset com esse nome nessa pasta.",
            ));
        }

        fs::rename(path, &target)?;
        Ok(target)
    }

    pub fn duplicate_path(&self, path: &Path) -> io::Result<PathBuf> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Pasta pai não encontrada."))?;
        if path.is_dir() {
            let duplicate_dir = make_unique_named_path(
                parent,
                path.file_name().and_then(|n| n.to_str()).unwrap_or("pasta"),
            );
            copy_dir_recursive(path, &duplicate_dir)?;
            return Ok(duplicate_dir);
        }

        let duplicate_file = make_unique_named_path(
            parent,
            path.file_name().and_then(|n| n.to_str()).unwrap_or("asset"),
        );
        fs::copy(path, &duplicate_file)?;
        Ok(duplicate_file)
    }

    pub fn delete_path(&self, path: &Path) -> io::Result<()> {
        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "O asset selecionado já não existe mais.",
            ));
        }
        if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }

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

        let template = "// Script RS2BR-Engine
@start_message Novo script
";
        fs::write(&path, template)?;
        Ok(path)
    }

    pub fn create_lua_script_file(&self, parent: &Path, name: &str) -> io::Result<PathBuf> {
        fs::create_dir_all(parent)?;

        let base_name = name.trim_end_matches(".lua");
        let sanitized = sanitize_asset_name(base_name);
        let final_name = format!("{sanitized}.lua");

        let path = parent.join(&final_name);
        if path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Já existe um script Lua com esse nome.",
            ));
        }

        let template = concat!(
            "-- Script Lua — RS2BR-Engine V",
            env!("CARGO_PKG_VERSION"),
            "\n",
            "--\n",
            "-- APIs disponíveis:\n",
            "--   entity.x, entity.y, entity.vx, entity.vy, entity.name\n",
            "--   entity.set_position(x, y)  entity.set_velocity(vx, vy)\n",
            "--   entity.set_rotation(r)     entity.set_visible(bool)\n",
            "--   entity.play_anim(\"clip\")\n",
            "--   input.key_held(\"A\")        input.key_pressed(\"Space\")\n",
            "--   input.mouse_pos()          input.mouse_left / mouse_right\n",
            "--   game.delta_time            game.elapsed_time\n",
            "--   game.log(\"msg\")            game.change_scene(\"path\")\n",
            "--   save.get(\"k\") save.set(\"k\", v) save.has(\"k\") save.remove(\"k\")\n",
            "\n",
            "function on_start()\n",
            "end\n",
            "\n",
            "function on_update(dt)\n",
            "end\n"
        );
        fs::write(&path, template)?;
        Ok(path)
    }

    pub fn create_script(&self, parent: &Path, name: &str) -> io::Result<PathBuf> {
        self.create_rs2_script_file(parent, name)
    }

    pub fn create_basic_project_template(&self, project_name: &str) -> io::Result<PathBuf> {
        create_basic_project_template_at(&self.root, project_name)
    }

    pub fn relative_path<'a>(&self, path: &'a Path) -> Option<&'a Path> {
        let assets = self.root.join("assets");
        path.strip_prefix(&assets).ok()
    }
}

fn collect_records(path: &Path, output: &mut Vec<AssetRecord>) {
    if !path.exists() {
        return;
    }
    output.push(AssetRecord::new(
        path.to_path_buf(),
        detect_asset_type(path),
    ));
    if path.is_dir() {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            collect_records(&entry.path(), output);
        }
    }
}

fn make_unique_path(target_dir: &Path, source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("asset");
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");

    for index in 1..1000 {
        let candidate_name = if ext.is_empty() {
            format!("{}_{}", stem, index)
        } else {
            format!("{}_{}.{}", stem, index, ext)
        };
        let candidate = target_dir.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    target_dir.join(format!("{}_copy", stem))
}

fn make_unique_named_path(parent: &Path, original_name: &str) -> PathBuf {
    let original = Path::new(original_name);
    let stem = original
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("asset");
    let ext = original.extension().and_then(|e| e.to_str()).unwrap_or("");

    for index in 1..1000 {
        let candidate_name = if ext.is_empty() {
            format!("{}_copy_{}", stem, index)
        } else {
            format!("{}_copy_{}.{}", stem, index, ext)
        };
        let candidate = parent.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    parent.join(format!("{}_copy", stem))
}

fn copy_dir_recursive(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = to.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("rs2br_{}_{}", prefix, unique));
        fs::create_dir_all(root.join("assets")).unwrap();
        root
    }

    #[test]
    fn sanitize_asset_name_removes_invalid_chars() {
        assert_eq!(sanitize_asset_name(" Meu asset?.png "), "Meu_asset.png");
    }

    #[test]
    fn create_scene_file_and_script_work() {
        let root = temp_root("asset_manager_create");
        let manager = AssetManager::new(root.clone());
        let scene_path = manager
            .create_scene_file(&root.join("assets/scenes"), "fase_01")
            .unwrap();
        let script_path = manager
            .create_rs2_script_file(&root.join("assets/scripts"), "jogador")
            .unwrap();

        assert!(scene_path.exists());
        assert!(script_path.exists());
        assert!(scene_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with(".scene.json"));
        assert!(script_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with(".rs2"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rename_scene_keeps_compound_extension() {
        let root = temp_root("asset_manager_scene_rename");
        let manager = AssetManager::new(root.clone());
        let scene = manager
            .create_scene_file(&root.join("assets/scenes"), "fase_teste")
            .unwrap();
        let renamed = manager.rename_path(&scene, "fase_final").unwrap();

        assert!(renamed
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with(".scene.json"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn duplicate_and_rename_asset_keep_file_alive() {
        let root = temp_root("asset_manager_ops");
        let manager = AssetManager::new(root.clone());
        let sprites = root.join("assets/sprites");
        fs::create_dir_all(&sprites).unwrap();
        let original = sprites.join("hero.png");
        fs::write(&original, b"fake").unwrap();

        let duplicate = manager.duplicate_path(&original).unwrap();
        let renamed = manager.rename_path(&duplicate, "hero_copia").unwrap();

        assert!(original.exists());
        assert!(renamed.exists());
        assert!(renamed
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("hero_copia"));

        let _ = fs::remove_dir_all(root);
    }
}
