// Estado e tipos principais do editor.
// Este arquivo passa a ser a fonte única de verdade para o estado do editor.

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::{Path, PathBuf}};

use crate::{
    assets::AssetManager,
    component::{Component, Sprite},
    entity::Entity,
    scene::Scene,
};
use crate::runtime::RuntimeState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorPlayState {
    Edit,
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub enum DeleteTarget {
    Entities { ids: Vec<String>, label: String },
    Asset { path: PathBuf, label: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorLayout {
    pub hierarchy_width: f32,
    pub inspector_width: f32,
    pub asset_height: f32,
}

impl Default for EditorLayout {
    fn default() -> Self {
        Self {
            hierarchy_width: 240.0,
            inspector_width: 300.0,
            asset_height: 220.0,
        }
    }
}

pub struct EditorApp {
    pub scene: Scene,
    pub selected_entity_id: Option<String>,
    pub selected_entity_ids: Vec<String>,
    pub selected_asset: Option<PathBuf>,
    pub assets: AssetManager,
    pub project_root: PathBuf,
    pub status_msg: String,
    pub scene_zoom: f32,
    pub scene_pan: eframe::egui::Vec2,
    pub show_entity_names: bool,
    pub show_colliders: bool,
    pub snap_to_grid: bool,
    pub asset_search: String,
    pub sprite_textures: HashMap<String, eframe::egui::TextureHandle>,
    pub dragging_asset_path: Option<PathBuf>,
    pub active_drag_entity_id: Option<String>,
    pub delete_confirmation: Option<DeleteTarget>,
    pub rename_asset_dialog: Option<(PathBuf, String)>,
    pub rename_entity_dialog: Option<(String, String)>,
    pub layout: EditorLayout,
    pub selection_box_start: Option<eframe::egui::Pos2>,
    pub selection_box_current: Option<eframe::egui::Pos2>,
    pub undo_stack: Vec<Scene>,
    pub redo_stack: Vec<Scene>,
    pub play_state: EditorPlayState,
    pub runtime: RuntimeState,
    pub new_script_dialog: Option<(PathBuf, String)>,
    pub new_lua_dialog: Option<(PathBuf, String)>,
    pub new_folder_dialog: Option<(PathBuf, String)>,
    pub new_entity_dialog: Option<String>,
}

impl EditorApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let _ = std::fs::create_dir_all(project_root.join("assets/sprites"));
        let _ = std::fs::create_dir_all(project_root.join("assets/scripts"));
        let _ = std::fs::create_dir_all(project_root.join("assets/sounds"));
        let _ = std::fs::create_dir_all(project_root.join("assets/matrs"));
        let _ = std::fs::create_dir_all(project_root.join("assets/prefabs"));
        let _ = std::fs::create_dir_all(project_root.join("scenes"));

        let assets = AssetManager::new(project_root.clone());
        let layout = load_editor_layout(&project_root).unwrap_or_default();

        Self {
            scene: Scene::new("Cena Principal"),
            selected_entity_id: None,
            selected_entity_ids: Vec::new(),
            selected_asset: None,
            assets,
            project_root,
            status_msg: "Bem-vindo ao Rust2D Engine!".to_string(),
            scene_zoom: 1.0,
            scene_pan: eframe::egui::Vec2::ZERO,
            show_entity_names: true,
            show_colliders: true,
            snap_to_grid: false,
            asset_search: String::new(),
            sprite_textures: HashMap::new(),
            dragging_asset_path: None,
            active_drag_entity_id: None,
            delete_confirmation: None,
            rename_asset_dialog: None,
            rename_entity_dialog: None,
            layout,
            selection_box_start: None,
            selection_box_current: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            play_state: EditorPlayState::Edit,
            runtime: RuntimeState::new(),
            new_script_dialog: None,
            new_lua_dialog: None,
            new_folder_dialog: None,
            new_entity_dialog: None,
        }
    }

    pub fn load_texture_from_relative_path(
        &mut self,
        ctx: &eframe::egui::Context,
        relative_path: &str,
    ) -> Option<eframe::egui::TextureHandle> {
        let key = relative_path.replace('\\', "/");

        if let Some(texture) = self.sprite_textures.get(&key) {
            return Some(texture.clone());
        }

        let candidates = [
            self.project_root.join("assets").join(&key),
            self.project_root.join(&key),
        ];

        let full_path = candidates.into_iter().find(|path| path.exists())?;

        let image = image::ImageReader::open(&full_path).ok()?.decode().ok()?.to_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let color_image = eframe::egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());

        let texture = ctx.load_texture(
            format!("asset://{}", key),
            color_image,
            eframe::egui::TextureOptions::LINEAR,
        );

        self.sprite_textures.insert(key, texture.clone());
        Some(texture)
    }

    pub fn save_layout_to_disk(&mut self) {
        match save_editor_layout(&self.project_root, &self.layout) {
            Ok(_) => self.status_msg = "💾 Layout do editor salvo.".to_string(),
            Err(error) => self.status_msg = format!("❌ Erro ao salvar layout: {}", error),
        }
    }

    pub fn create_sprite_entity_from_asset(
        &mut self,
        path: &Path,
        world_pos: Option<(f32, f32)>,
    ) -> Result<String, String> {
        let Some(relative) = self.assets.relative_path(path) else {
            return Err("Não foi possível resolver o caminho do sprite.".to_string());
        };

        self.push_undo_state();

        let relative_str = relative.to_string_lossy().replace('\\', "/");
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Sprite");

        let mut entity = Entity::new(name);
        entity.add_component(Component::Sprite(Sprite {
            texture_path: relative_str,
            ..Sprite::default()
        }));

        if let Some((x, y)) = world_pos {
            if let Some(transform) = entity.transform_mut() {
                transform.x = x;
                transform.y = y;
            }
        }

        let id = entity.id.clone();
        self.scene.add_entity(entity);
        self.select_single_entity(Some(id));
        Ok(name.to_string())
    }

    pub fn export_entity_as_matr(&mut self, entity: &Entity) -> std::io::Result<PathBuf> {
        let prefabs_dir = self.project_root.join("assets/prefabs");
        std::fs::create_dir_all(&prefabs_dir)?;

        let filename = format!("{}.prefab.json", sanitize_filename(&entity.name));
        let path = prefabs_dir.join(filename);
        let prefab = crate::core::prefab::Prefab::new(entity.name.clone(), entity.clone());
        crate::serialization::prefab_serializer::save_prefab_to_path(&prefab, &path)?;
        self.assets.refresh();
        Ok(path)
    }

    pub fn instantiate_matr_from_path(
        &mut self,
        path: &Path,
        world_pos: Option<(f32, f32)>,
    ) -> Result<String, String> {
        if !is_matr_file_path(path) {
            return Err("O arquivo selecionado não é um MATR válido.".to_string());
        }

        self.push_undo_state();

        let content = std::fs::read_to_string(path)
            .map_err(|error| format!("Falha ao ler MATR: {}", error))?;

        let mut entity = if let Some(prefab) = crate::serialization::prefab_serializer::prefab_from_json(&content) {
            prefab.root_entity
        } else {
            Entity::from_json(&content).ok_or_else(|| "Prefab/MATR inválido ou corrompido.".to_string())?
        };

        entity.regenerate_ids_recursive();
        entity.matr_source = path.file_name().and_then(|n| n.to_str()).map(|n| n.to_string());

        if let Some((x, y)) = world_pos {
            if let Some(transform) = entity.transform_mut() {
                transform.x = x;
                transform.y = y;
            }
        }

        let id = entity.id.clone();
        let name = entity.name.clone();
        self.scene.add_entity(entity);
        self.select_single_entity(Some(id));
        Ok(name)
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

fn layout_file_path(project_root: &Path) -> PathBuf {
    project_root.join("editor_layout.json")
}

fn load_editor_layout(project_root: &Path) -> Option<EditorLayout> {
    let path = layout_file_path(project_root);
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_editor_layout(project_root: &Path, layout: &EditorLayout) -> std::io::Result<()> {
    let path = layout_file_path(project_root);
    let json = serde_json::to_string_pretty(layout).unwrap_or_default();
    std::fs::write(path, json)
}

fn is_matr_file_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with(".matr.json") || name.ends_with(".prefab.json"))
        .unwrap_or(false)
}
