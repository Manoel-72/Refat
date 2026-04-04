// ============================================================
//  editor/mod.rs
//  Módulo raiz do editor — junta todos os painéis da UI
// ============================================================

mod hierarchy;
mod inspector;
mod asset_browser;
mod scene_view;
mod menubar;
mod warnings;

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    assets::AssetManager,
    core::{
        component::{Component, Sprite},
        entity::Entity,
        project::ProjectConfig,
        scene::Scene,
    },
    runtime::{self, RuntimeState},
    core::version,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorPlayState {
    Edit,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetBrowserFilter {
    All,
    Images,
    Scripts,
    Scenes,
    Prefabs,
    Audio,
    Fonts,
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

#[derive(Debug, Clone)]
pub struct OpenSceneDocument {
    pub scene: Scene,
    pub file_path: Option<PathBuf>,
}

impl OpenSceneDocument {
    pub fn display_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|path| path.file_stem())
            .and_then(|name| name.to_str())
            .map(|name| name.replace(".scene", ""))
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| self.scene.name.clone())
    }
}

/// Estado global do editor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorStatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct RuntimePreviewWarning {
    pub severity: warnings::EditorWarningSeverity,
    pub label: String,
    pub details: String,
}

pub struct EditorApp {
    /// Cena atualmente aberta
    pub scene: Scene,
    /// Entidade principal selecionada (ID)
    pub selected_entity_id: Option<String>,
    /// Entidades selecionadas (seleção múltipla)
    pub selected_entity_ids: Vec<String>,
    /// Asset selecionado (caminho)
    pub selected_asset: Option<PathBuf>,
    /// Gerenciador de assets
    pub assets: AssetManager,
    /// Pasta raiz do projeto
    pub project_root: PathBuf,
    /// Mensagem de status no rodapé
    pub status_msg: String,
    /// Zoom atual da viewport da cena
    pub scene_zoom: f32,
    /// Deslocamento visual da cena (pan da câmera do editor)
    pub scene_pan: egui::Vec2,
    /// Exibir nomes das entidades na viewport
    pub show_entity_names: bool,
    /// Exibir colliders/gizmos extras na viewport
    pub show_colliders: bool,
    /// Ajuste opcional do arrasto à grade de 32px
    pub snap_to_grid: bool,
    /// Texto de busca do painel de assets
    pub asset_search: String,
    /// Filtro rápido do painel de assets
    pub asset_filter: AssetBrowserFilter,
    /// Cache de texturas carregadas dos sprites
    pub sprite_textures: HashMap<String, egui::TextureHandle>,
    /// Asset atualmente sendo arrastado do painel Assets para a cena
    pub dragging_asset_path: Option<PathBuf>,
    /// Entidade atualmente em arrasto dentro da cena
    pub active_drag_entity_id: Option<String>,
    /// Diálogo/estado de confirmação de exclusão
    pub delete_confirmation: Option<DeleteTarget>,
    /// Diálogo de renomear asset/pasta
    pub rename_asset_dialog: Option<(PathBuf, String)>,
    /// Diálogo de renomear entidade (id, novo nome)
    pub rename_entity_dialog: Option<(String, String)>,
    /// Layout salvo do editor (larguras/alturas dos painéis)
    pub layout: EditorLayout,
    /// Início da caixa de seleção por arrasto na viewport
    pub selection_box_start: Option<egui::Pos2>,
    /// Posição atual da caixa de seleção por arrasto
    pub selection_box_current: Option<egui::Pos2>,
    /// Histórico para desfazer alterações da cena
    pub undo_stack: Vec<Scene>,
    /// Histórico para refazer alterações da cena
    pub redo_stack: Vec<Scene>,
    /// Estado atual do editor (editar, play, pause)
    pub play_state: EditorPlayState,
    /// Estado interno do runtime básico usado pelo preview de jogo
    pub runtime: RuntimeState,
    /// Lista de cenas abertas no editor
    pub open_scenes: Vec<OpenSceneDocument>,
    /// Índice da cena ativa na lista de cenas abertas
    pub active_scene_index: usize,
    /// Diálogo de novo script (caminho da pasta, nome do arquivo)
    pub new_script_dialog: Option<(PathBuf, String)>,
    /// Diálogo de nova pasta (caminho pai, nome)
    pub new_folder_dialog: Option<(PathBuf, String)>,
    /// Diálogo de nova entidade (nome)
    pub new_entity_dialog: Option<String>,
    /// Pop-up inicial com a versão atual da engine
    pub show_version_popup: bool,
}


impl EditorApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let _ = std::fs::create_dir_all(project_root.join("assets/sprites"));
        let _ = std::fs::create_dir_all(project_root.join("assets/scripts"));
        let _ = std::fs::create_dir_all(project_root.join("assets/sounds"));
        let _ = std::fs::create_dir_all(project_root.join("assets/matrs"));
        let _ = std::fs::create_dir_all(project_root.join("assets/prefabs"));
        let _ = std::fs::create_dir_all(project_root.join("assets/scenes"));

        let assets = AssetManager::new(project_root.clone());
        let layout = load_editor_layout(&project_root).unwrap_or_default();
        let project_config = ProjectConfig::load_or_create(&project_root);

        let initial_scene = crate::serialization::scene_serializer::try_load_scene_from_path(
            &project_root.join(&project_config.initial_scene),
        )
        .unwrap_or_else(|_| Scene::new("Cena Principal"));

        Self {
            scene: initial_scene.clone(),
            selected_entity_id: None,
            selected_entity_ids: Vec::new(),
            selected_asset: None,
            assets,
            project_root,
            status_msg: format!("Bem-vindo ao RS2BR-Engine! Projeto: {}", project_config.name),
            scene_zoom: 1.0,
            scene_pan: egui::Vec2::ZERO,
            show_entity_names: true,
            show_colliders: true,
            snap_to_grid: false,
            asset_search: String::new(),
            asset_filter: AssetBrowserFilter::All,
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
            open_scenes: vec![OpenSceneDocument {
                scene: initial_scene,
                file_path: None,
            }],
            active_scene_index: 0,
            new_script_dialog: None,
            new_folder_dialog: None,
            new_entity_dialog: None,
            show_version_popup: true,
        }
    }


    fn show_version_popup(&mut self, ctx: &egui::Context) {
        if !self.show_version_popup {
            return;
        }

        egui::Window::new("RS2BR Engine - Versão atual")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.heading(format!("{} {}", version::ENGINE_TITLE, version::ENGINE_VERSION));
                ui.label(format!("Status: {}", version::ENGINE_STATUS));
                ui.label("Esta versão está em teste.");
                ui.label("Clique em OK para continuar e confirmar qual versão está aberta.");
                ui.separator();
                if ui.button("OK").clicked() {
                    self.show_version_popup = false;
                    self.status_msg = format!("{}", version::startup_message());
                }
            });
    }

    pub fn find_entity_mut(&mut self, id: &str) -> Option<&mut Entity> {
        find_entity_recursive_mut(&mut self.scene.entities, id)
    }

    pub fn load_texture_from_relative_path(
        &mut self,
        ctx: &egui::Context,
        relative_path: &str,
    ) -> Option<egui::TextureHandle> {
        let key = relative_path.replace('\\', "/");

        if let Some(texture) = self.sprite_textures.get(&key) {
            return Some(texture.clone());
        }

        let candidates = [
            self.project_root.join("assets").join(&key),
            self.project_root.join(&key),
        ];

        let full_path = candidates.into_iter().find(|p| p.exists())?;

        let image = image::ImageReader::open(&full_path)
            .ok()?
            .decode()
            .ok()?
            .to_rgba8();

        let size = [image.width() as usize, image.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());

        let texture = ctx.load_texture(
            format!("asset://{}", key),
            color_image,
            egui::TextureOptions::LINEAR,
        );

        self.sprite_textures.insert(key, texture.clone());
        Some(texture)
    }

    pub fn is_entity_selected(&self, id: &str) -> bool {
        self.selected_entity_ids
            .iter()
            .any(|selected| selected == id)
    }

    pub fn select_single_entity(&mut self, id: Option<String>) {
        self.selected_entity_id = id.clone();
        self.selected_entity_ids = id.into_iter().collect();
    }

    pub fn clear_entity_selection(&mut self) {
        self.selected_entity_id = None;
        self.selected_entity_ids.clear();
    }

    pub fn toggle_entity_selection(&mut self, id: String) {
        if let Some(index) = self
            .selected_entity_ids
            .iter()
            .position(|selected| selected == &id)
        {
            self.selected_entity_ids.remove(index);
            if self.selected_entity_id.as_deref() == Some(&id) {
                self.selected_entity_id = self.selected_entity_ids.last().cloned();
            }
        } else {
            self.selected_entity_ids.push(id.clone());
            self.selected_entity_id = Some(id);
        }
    }

    pub fn push_undo_state(&mut self) {
        self.undo_stack.push(self.scene.clone());
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo_scene(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(self.scene.clone());
            self.scene = previous;
            self.clear_entity_selection();
            self.status_msg = "↶ Desfazer aplicado.".to_string();
        } else {
            self.status_msg = "Nada para desfazer.".to_string();
        }
    }

    pub fn redo_scene(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.scene.clone());
            self.scene = next;
            self.clear_entity_selection();
            self.status_msg = "↷ Refazer aplicado.".to_string();
        } else {
            self.status_msg = "Nada para refazer.".to_string();
        }
    }

    pub fn request_delete_selected(&mut self) {
        if !self.selected_entity_ids.is_empty() {
            let ids = self.selected_entity_ids.clone();
            let label = if ids.len() == 1 {
                let name = find_entity_recursive(&self.scene.entities, &ids[0])
                    .map(|e| e.name.clone())
                    .unwrap_or_else(|| "Entidade".to_string());
                format!("a entidade '{}'", name)
            } else {
                format!("{} entidades selecionadas", ids.len())
            };
            self.delete_confirmation = Some(DeleteTarget::Entities { ids, label });
            return;
        }

        if let Some(id) = self.selected_entity_id.clone() {
            let name = find_entity_recursive(&self.scene.entities, &id)
                .map(|e| e.name.clone())
                .unwrap_or_else(|| "Entidade".to_string());
            self.request_delete_entity(id, name);
            return;
        }

        if let Some(path) = self.selected_asset.clone() {
            self.request_delete_asset(path);
            return;
        }

        self.status_msg = "Nada selecionado para deletar.".to_string();
    }

    pub fn request_delete_entity(&mut self, id: String, name: String) {
        self.delete_confirmation = Some(DeleteTarget::Entities {
            ids: vec![id],
            label: format!("a entidade '{}'", name),
        });
    }

    pub fn request_delete_asset(&mut self, path: PathBuf) {
        let label = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("asset")
            .to_string();
        self.delete_confirmation = Some(DeleteTarget::Asset { path, label });
    }

    pub fn request_rename_asset(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("asset")
            .to_string();
        self.rename_asset_dialog = Some((path, name));
    }

    pub fn request_rename_selected_entity(&mut self) {
        if let Some(id) = self.selected_entity_id.clone() {
            let name = find_entity_recursive(&self.scene.entities, &id)
                .map(|e| e.name.clone())
                .unwrap_or_else(|| "Entidade".to_string());
            self.rename_entity_dialog = Some((id, name));
        }
    }

    pub fn save_layout_to_disk(&mut self) {
        match save_editor_layout(&self.project_root, &self.layout) {
            Ok(_) => {
                self.status_msg = "💾 Layout do editor salvo.".to_string();
            }
            Err(e) => {
                self.status_msg = format!("❌ Erro ao salvar layout: {}", e);
            }
        }
    }


    pub fn collect_runtime_warnings(&self) -> Vec<RuntimePreviewWarning> {
        warnings::collect_scene_warnings(&self.project_root, &self.scene)
            .into_iter()
            .map(|warning| RuntimePreviewWarning {
                severity: warning.severity,
                label: warning.label,
                details: warning.details,
            })
            .collect()
    }

    pub fn warning_count(&self) -> usize {
        self.collect_runtime_warnings().len()
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
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Sprite");

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

        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Falha ao ler MATR: {}", e))?;

        let mut entity = if let Some(prefab) = crate::serialization::prefab_serializer::prefab_from_json(&content) {
            prefab.root_entity
        } else {
            Entity::from_json(&content).ok_or_else(|| "Prefab/MATR inválido ou corrompido.".to_string())?
        };

        entity.regenerate_ids_recursive();
        entity.matr_source = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_string());

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

    pub fn scenes_dir(&self) -> PathBuf {
        self.project_root.join("assets/scenes")
    }

    pub fn scene_file_candidates(&self) -> Vec<PathBuf> {
        let scenes_dir = self.scenes_dir();
        let mut files: Vec<PathBuf> = std::fs::read_dir(&scenes_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.is_file())
            .filter(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".scene.json"))
                    .unwrap_or(false)
            })
            .collect();
        files.sort();
        files
    }

    pub fn sync_active_scene_document(&mut self) {
        if let Some(active) = self.open_scenes.get_mut(self.active_scene_index) {
            active.scene = self.scene.clone();
        }
    }

    pub fn activate_scene_tab(&mut self, index: usize) {
        if index >= self.open_scenes.len() {
            return;
        }

        self.sync_active_scene_document();
        self.active_scene_index = index;

        if let Some(active) = self.open_scenes.get(index) {
            self.scene = active.scene.clone();
            self.clear_entity_selection();
            self.status_msg = format!("Cena ativa: {}", self.scene.name);
        }
    }

    pub fn open_scene_from_path(&mut self, path: PathBuf) -> Result<(), String> {
        self.sync_active_scene_document();

        if let Some(existing_index) = self
            .open_scenes
            .iter()
            .position(|doc| doc.file_path.as_ref() == Some(&path))
        {
            self.activate_scene_tab(existing_index);
            return Ok(());
        }

        let scene = Scene::load_from_path(&path)
            .ok_or_else(|| format!("Falha ao carregar a cena: {}", path.display()))?;

        self.open_scenes.push(OpenSceneDocument {
            scene: scene.clone(),
            file_path: Some(path.clone()),
        });

        self.active_scene_index = self.open_scenes.len().saturating_sub(1);
        self.scene = scene;
        self.clear_entity_selection();
        self.assets.refresh();
        self.status_msg = format!(
            "✅ Cena carregada: {}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("cena")
        );
        Ok(())
    }

    pub fn create_new_scene_tab(&mut self, name: impl Into<String>) {
        self.sync_active_scene_document();

        let scene = Scene::new(name.into());
        self.open_scenes.push(OpenSceneDocument {
            scene: scene.clone(),
            file_path: None,
        });

        self.active_scene_index = self.open_scenes.len().saturating_sub(1);
        self.scene = scene;
        self.clear_entity_selection();
        self.status_msg = format!("✅ Nova cena criada: {}", self.scene.name);
    }

    pub fn save_active_scene(&mut self) -> Result<PathBuf, String> {
        self.sync_active_scene_document();

        let mut path = self
            .open_scenes
            .get(self.active_scene_index)
            .and_then(|doc| doc.file_path.clone())
            .unwrap_or_else(|| self.scenes_dir().join(self.scene.default_file_name()));

        if path.file_name().is_none() {
            path = self.scenes_dir().join(self.scene.default_file_name());
        }

        self.scene
            .save_to_path(&path)
            .map_err(|error| format!("Erro ao salvar cena: {}", error))?;

        if let Some(active) = self.open_scenes.get_mut(self.active_scene_index) {
            active.file_path = Some(path.clone());
            active.scene = self.scene.clone();
        }

        self.assets.refresh();
        Ok(path)
    }

    pub fn close_scene_tab(&mut self, index: usize) {
        if self.open_scenes.len() <= 1 || index >= self.open_scenes.len() {
            return;
        }

        self.sync_active_scene_document();
        self.open_scenes.remove(index);

        let next_index = self
            .active_scene_index
            .min(self.open_scenes.len().saturating_sub(1));

        self.active_scene_index = next_index;

        if let Some(active) = self.open_scenes.get(next_index) {
            self.scene = active.scene.clone();
            self.clear_entity_selection();
        }
    }

    fn show_new_script_dialog(&mut self, ctx: &egui::Context) {
        if let Some((folder, name)) = self.new_script_dialog.clone() {
            let mut open = true;
            egui::Window::new("Novo Script RS2")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Nome do script (.rs2):");
                    let mut buf = name.clone();
                    ui.text_edit_singleline(&mut buf);

                    if let Some((_, ref mut n)) = self.new_script_dialog {
                        *n = buf.clone();
                    }

                    ui.horizontal(|ui| {
                        if ui.button("✅ Criar").clicked() {
                            match self.assets.create_rs2_script_file(&folder, &buf) {
                                Ok(p) => {
                                    self.status_msg = format!("✅ Script RS2 criado: {}", p.file_name().and_then(|n| n.to_str()).unwrap_or("script.rs2"));
                                    self.assets.refresh();

                                    #[cfg(target_os = "windows")]
                                    {
                                        let _ = std::process::Command::new("cmd")
                                            .args(["/C", "code", &p.to_string_lossy()])
                                            .spawn();
                                    }
                                }
                                Err(e) => self.status_msg = format!("Erro: {}", e),
                            }
                            self.new_script_dialog = None;
                        }

                        if ui.button("❌ Cancelar").clicked() {
                            self.new_script_dialog = None;
                        }
                    });
                });

            if !open {
                self.new_script_dialog = None;
            }
        }
    }

    fn show_new_folder_dialog(&mut self, ctx: &egui::Context) {
        if let Some((parent, name)) = self.new_folder_dialog.clone() {
            let mut open = true;
            egui::Window::new("Nova Pasta")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Nome da pasta:");
                    let mut buf = name.clone();
                    ui.text_edit_singleline(&mut buf);

                    if let Some((_, ref mut n)) = self.new_folder_dialog {
                        *n = buf.clone();
                    }

                    ui.horizontal(|ui| {
                        if ui.button("✅ Criar").clicked() {
                            match self.assets.create_folder(&parent, &buf) {
                                Ok(_) => {
                                    self.status_msg = format!("✅ Pasta criada: {}", crate::assets::manager::sanitize_asset_name(&buf));
                                    self.assets.refresh();
                                }
                                Err(e) => self.status_msg = format!("Erro: {}", e),
                            }
                            self.new_folder_dialog = None;
                        }

                        if ui.button("❌ Cancelar").clicked() {
                            self.new_folder_dialog = None;
                        }
                    });
                });

            if !open {
                self.new_folder_dialog = None;
            }
        }
    }

    fn show_new_entity_dialog(&mut self, ctx: &egui::Context) {
        if let Some(name) = self.new_entity_dialog.clone() {
            let mut open = true;
            egui::Window::new("Nova Entidade")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Nome da entidade:");
                    let mut buf = name.clone();
                    ui.text_edit_singleline(&mut buf);

                    if let Some(ref mut n) = self.new_entity_dialog {
                        *n = buf.clone();
                    }

                    ui.horizontal(|ui| {
                        if ui.button("✅ Criar").clicked() {
                            self.push_undo_state();
                            let entity = Entity::new(&buf);
                            let id = entity.id.clone();
                            self.scene.add_entity(entity);
                            self.select_single_entity(Some(id));
                            self.status_msg = format!("✅ Entidade '{}' criada.", buf.trim());
                            self.new_entity_dialog = None;
                        }

                        if ui.button("❌ Cancelar").clicked() {
                            self.new_entity_dialog = None;
                        }
                    });
                });

            if !open {
                self.new_entity_dialog = None;
            }
        }
    }

    fn show_rename_asset_dialog(&mut self, ctx: &egui::Context) {
        if let Some((path, current_name)) = self.rename_asset_dialog.clone() {
            let mut open = true;
            egui::Window::new("Renomear Asset")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Novo nome do arquivo/pasta:");
                    let mut buf = current_name.clone();
                    ui.text_edit_singleline(&mut buf);

                    if let Some((_, ref mut name)) = self.rename_asset_dialog {
                        *name = buf.clone();
                    }

                    ui.horizontal(|ui| {
                        if ui.button("✅ Renomear").clicked() {
                            match self.assets.rename_path(&path, &buf) {
                                Ok(new_path) => {
                                    self.assets.refresh();
                                    self.selected_asset = Some(new_path.clone());
                                    self.status_msg = format!(
                                        "✏ Asset renomeado para '{}'",
                                        new_path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("asset")
                                    );
                                }
                                Err(e) => {
                                    self.status_msg =
                                        format!("❌ Erro ao renomear asset: {}", e);
                                }
                            }
                            self.rename_asset_dialog = None;
                        }

                        if ui.button("❌ Cancelar").clicked() {
                            self.rename_asset_dialog = None;
                        }
                    });
                });

            if !open {
                self.rename_asset_dialog = None;
            }
        }
    }

    fn show_rename_entity_dialog(&mut self, ctx: &egui::Context) {
        if let Some((entity_id, current_name)) = self.rename_entity_dialog.clone() {
            let mut open = true;
            egui::Window::new("Renomear Entidade")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Novo nome da entidade:");
                    let mut buf = current_name.clone();
                    ui.text_edit_singleline(&mut buf);

                    if let Some((_, ref mut name)) = self.rename_entity_dialog {
                        *name = buf.clone();
                    }

                    ui.horizontal(|ui| {
                        if ui.button("✅ Renomear").clicked() {
                            if !buf.trim().is_empty() {
                                if let Some(entity) = self.find_entity_mut(&entity_id) {
                                    entity.name = buf.trim().to_string();
                                    self.status_msg = format!("✅ Entidade renomeada para '{}'.", buf.trim());
                                }
                            }
                            self.rename_entity_dialog = None;
                        }

                        if ui.button("❌ Cancelar").clicked() {
                            self.rename_entity_dialog = None;
                        }
                    });
                });

            if !open {
                self.rename_entity_dialog = None;
            }
        }
    }

    fn show_delete_confirmation_dialog(&mut self, ctx: &egui::Context) {
        let Some(target) = self.delete_confirmation.clone() else {
            return;
        };

        let mut open = true;
        let label = match &target {
            DeleteTarget::Entities { label, .. } => label.clone(),
            DeleteTarget::Asset { label, .. } => format!("'{}'", label),
        };

        egui::Window::new("Confirmar exclusão")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!("Deseja realmente deletar {}?", label));
                ui.label("Essa ação não pode ser desfeita.");
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui.button("🗑 Sim, deletar").clicked() {
                        match target.clone() {
                            DeleteTarget::Entities { ids, label } => {
                                self.push_undo_state();
                                for id in &ids {
                                    self.scene.remove_entity_by_id(id);
                                }
                                self.clear_entity_selection();
                                self.status_msg = if ids.len() == 1 {
                                    format!("🗑 {} removida.", label)
                                } else {
                                    format!("🗑 {} removidas.", label)
                                };
                            }
                            DeleteTarget::Asset { path, label } => match self.assets.delete_path(&path) {
                                Ok(_) => {
                                    self.assets.refresh();
                                    if self
                                        .selected_asset
                                        .as_ref()
                                        .map(|p| p == &path || p.starts_with(&path))
                                        .unwrap_or(false)
                                    {
                                        self.selected_asset = None;
                                    }
                                    self.status_msg = format!("🗑 '{}' removido.", label);
                                }
                                Err(e) => {
                                    self.status_msg =
                                        format!("❌ Erro ao deletar '{}': {}", label, e);
                                }
                            },
                        }
                        self.delete_confirmation = None;
                    }

                    if ui.button("❌ Não").clicked() {
                        self.delete_confirmation = None;
                    }
                });
            });

        if !open {
            self.delete_confirmation = None;
        }
    }
}

fn infer_status_level(message: &str) -> EditorStatusLevel {
    let trimmed = message.trim_start();
    if trimmed.starts_with('❌') || trimmed.to_ascii_lowercase().starts_with("erro") {
        EditorStatusLevel::Error
    } else if trimmed.starts_with('⚠') {
        EditorStatusLevel::Warning
    } else if trimmed.starts_with('✅') || trimmed.starts_with('✔') {
        EditorStatusLevel::Success
    } else {
        EditorStatusLevel::Info
    }
}

fn status_visuals(level: EditorStatusLevel) -> (&'static str, egui::Color32) {
    match level {
        EditorStatusLevel::Info => ("ℹ", egui::Color32::from_rgb(120, 180, 255)),
        EditorStatusLevel::Success => ("✅", egui::Color32::from_rgb(120, 220, 140)),
        EditorStatusLevel::Warning => ("⚠", egui::Color32::YELLOW),
        EditorStatusLevel::Error => ("❌", egui::Color32::from_rgb(255, 120, 120)),
    }
}



fn apply_editor_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(229, 231, 235));
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 34, 42);
    visuals.widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(37, 41, 50);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 66, 78));
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(42, 47, 58);
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(42, 47, 58);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 78, 92));
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(229, 231, 235));
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 88, 156);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(50, 88, 156);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(88, 144, 255));
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.2, egui::Color32::WHITE);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(36, 72, 138);
    visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(36, 72, 138);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(110, 166, 255));
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.2, egui::Color32::WHITE);
    visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(58, 134, 255, 110);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 134, 255));
    visuals.panel_fill = egui::Color32::from_rgb(24, 28, 35);
    visuals.window_fill = egui::Color32::from_rgb(31, 36, 45);
    visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(64, 72, 88));
    visuals.extreme_bg_color = egui::Color32::from_rgb(18, 22, 28);
    visuals.faint_bg_color = egui::Color32::from_rgb(35, 39, 48);
    visuals.code_bg_color = egui::Color32::from_rgb(20, 24, 30);
    visuals.hyperlink_color = egui::Color32::from_rgb(88, 144, 255);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.indent = 16.0;
    style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(20.0));
    style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    ctx.set_style(style);
}

fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(31, 36, 45))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 66, 80)))
        .inner_margin(egui::Margin::symmetric(10.0, 10.0))
        .rounding(egui::Rounding::same(10.0))
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_editor_theme(ctx);
        self.sync_active_scene_document();

        if self.delete_confirmation.is_none()
            && !ctx.wants_keyboard_input()
            && ctx.input(|i| i.key_pressed(egui::Key::Delete))
        {
            self.request_delete_selected();
        }

        if self.rename_asset_dialog.is_none()
            && self.rename_entity_dialog.is_none()
            && !ctx.wants_keyboard_input()
            && ctx.input(|i| i.key_pressed(egui::Key::F2))
        {
            if self.selected_entity_id.is_some() {
                self.request_rename_selected_entity();
            } else if let Some(path) = self.selected_asset.clone() {
                self.request_rename_asset(path);
            }
        }

        if !ctx.wants_keyboard_input()
            && ctx.input(|i| {
                i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z)
            })
        {
            self.undo_scene();
        }

        if !ctx.wants_keyboard_input()
            && ctx.input(|i| {
                (i.modifiers.command && i.key_pressed(egui::Key::Y))
                    || (i.modifiers.command
                        && i.modifiers.shift
                        && i.key_pressed(egui::Key::Z))
            })
        {
            self.redo_scene();
        }

        if self.play_state == EditorPlayState::Edit && self.runtime.active_scene.is_some() {
            self.runtime.stop();
        }

        menubar::show(self, ctx);

        let hierarchy_response = egui::SidePanel::left("hierarchy_panel")
            .frame(panel_frame())
            .default_width(self.layout.hierarchy_width)
            .resizable(true)
            .min_width(180.0)
            .max_width(520.0)
            .show(ctx, |ui| {
                hierarchy::show(self, ui);
            });
        self.layout.hierarchy_width = hierarchy_response.response.rect.width();

        let inspector_response = egui::SidePanel::right("inspector_panel")
            .frame(panel_frame())
            .default_width(self.layout.inspector_width)
            .resizable(true)
            .min_width(220.0)
            .max_width(520.0)
            .show(ctx, |ui| {
                inspector::show(self, ui);
            });
        self.layout.inspector_width = inspector_response.response.rect.width();

        let asset_response = egui::TopBottomPanel::bottom("asset_panel")
            .frame(panel_frame())
            .default_height(self.layout.asset_height)
            .resizable(true)
            .min_height(150.0)
            .max_height(520.0)
            .show(ctx, |ui| {
                asset_browser::show(self, ui);
            });
        self.layout.asset_height = asset_response.response.rect.height();

        egui::CentralPanel::default()
            .frame(panel_frame())
            .show(ctx, |ui| {
                scene_view::show(self, ui);
            });

        egui::TopBottomPanel::bottom("status_bar").frame(panel_frame()).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                let level = infer_status_level(&self.status_msg);
                let (icon, color) = status_visuals(level);
                let trimmed_message = self
                    .status_msg
                    .trim_start_matches(|c: char| matches!(c, '✅' | '⚠' | '❌' | 'ℹ' | '✔' | ' '));
                ui.colored_label(color, format!("{} {}", icon, trimmed_message));
                ui.separator();

                let mode_label = match self.play_state {
                    EditorPlayState::Edit => "Modo: Edição",
                    EditorPlayState::Playing => "Modo: Play",
                    EditorPlayState::Paused => "Modo: Pausado",
                };
                ui.label(mode_label);

                let warning_count = self.warning_count();
                ui.separator();
                if warning_count == 0 {
                    ui.label("Warnings: 0");
                } else {
                    ui.colored_label(egui::Color32::YELLOW, format!("Warnings: {}", warning_count));
                }

                if let Some(asset) = self.selected_asset.as_ref() {
                    let record = self.assets.asset_record_for(asset);
                    ui.separator();
                    let asset_color = if record.validation.is_valid {
                        egui::Color32::from_rgb(120, 220, 140)
                    } else {
                        egui::Color32::from_rgb(255, 120, 120)
                    };
                    ui.colored_label(asset_color, format!("Asset: {}", record.name));
                }

                if !self.selected_entity_ids.is_empty() {
                    ui.separator();
                    ui.label(format!(
                        "{} selecionada(s)",
                        self.selected_entity_ids.len()
                    ));
                }
            });
        });

        self.show_new_script_dialog(ctx);
        self.show_new_folder_dialog(ctx);
        self.show_new_entity_dialog(ctx);
        self.show_rename_asset_dialog(ctx);
        self.show_rename_entity_dialog(ctx);
        self.show_delete_confirmation_dialog(ctx);
        self.show_version_popup(ctx);

        runtime::show_viewport(self, ctx);

        if ctx.input(|i| i.pointer.any_released()) {
            self.dragging_asset_path = None;
            self.active_drag_entity_id = None;
        }
    }
}

fn find_entity_recursive_mut<'a>(
    entities: &'a mut Vec<Entity>,
    id: &str,
) -> Option<&'a mut Entity> {
    for entity in entities.iter_mut() {
        if entity.id == id {
            return Some(entity);
        }
        if let Some(found) = find_entity_recursive_mut(&mut entity.children, id) {
            return Some(found);
        }
    }
    None
}

fn find_entity_recursive<'a>(entities: &'a [Entity], id: &str) -> Option<&'a Entity> {
    for entity in entities {
        if entity.id == id {
            return Some(entity);
        }
        if let Some(found) = find_entity_recursive(&entity.children, id) {
            return Some(found);
        }
    }
    None
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
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
        .and_then(|n| n.to_str())
        .map(|n| n.ends_with(".matr.json") || n.ends_with(".prefab.json"))
        .unwrap_or(false)
}