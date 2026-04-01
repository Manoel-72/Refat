// ============================================================
//  editor/mod.rs
//  Módulo raiz do editor — junta todos os painéis da UI
// ============================================================

mod hierarchy;
mod inspector;
mod asset_browser;
mod scene_view;
mod menubar;

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::{Path, PathBuf}};

use crate::{
    engine::{
        assets::AssetManager,
        component::{Component, Sprite},
        entity::Entity,
        scene::Scene,
    },
    runtime::{self, RuntimeState},
};

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

/// Estado global do editor
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
    /// Diálogo de novo script (caminho da pasta, nome do arquivo)
    pub new_script_dialog: Option<(PathBuf, String)>,
    /// Diálogo de nova pasta (caminho pai, nome)
    pub new_folder_dialog: Option<(PathBuf, String)>,
    /// Diálogo de nova entidade (nome)
    pub new_entity_dialog: Option<String>,
}

impl EditorApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Pasta do projeto = diretório onde o executável está rodando
        let project_root = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."));

        // Garante estrutura de pastas mínima
        let _ = std::fs::create_dir_all(project_root.join("assets/sprites"));
        let _ = std::fs::create_dir_all(project_root.join("assets/scripts"));
        let _ = std::fs::create_dir_all(project_root.join("assets/sounds"));
        let _ = std::fs::create_dir_all(project_root.join("assets/matrs"));
        let _ = std::fs::create_dir_all(project_root.join("assets/prefabs")); // compatibilidade antiga
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
            scene_pan: egui::Vec2::ZERO,
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
            new_folder_dialog: None,
            new_entity_dialog: None,
        }
    }

    /// Encontra entidade por ID (buscando na hierarquia)
    pub fn find_entity_mut(&mut self, id: &str) -> Option<&mut Entity> {
        find_entity_recursive_mut(&mut self.scene.entities, id)
    }

    /// Carrega uma textura da pasta /assets e a mantém em cache
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

    /// Retorna se uma entidade faz parte da seleção atual
    pub fn is_entity_selected(&self, id: &str) -> bool {
        self.selected_entity_ids.iter().any(|selected| selected == id)
    }

    /// Seleciona apenas uma entidade como foco principal
    pub fn select_single_entity(&mut self, id: Option<String>) {
        self.selected_entity_id = id.clone();
        self.selected_entity_ids = id.into_iter().collect();
    }

    /// Limpa a seleção atual de entidades
    pub fn clear_entity_selection(&mut self) {
        self.selected_entity_id = None;
        self.selected_entity_ids.clear();
    }

    /// Alterna uma entidade na seleção múltipla (Ctrl + clique)
    pub fn toggle_entity_selection(&mut self, id: String) {
        if let Some(index) = self.selected_entity_ids.iter().position(|selected| selected == &id) {
            self.selected_entity_ids.remove(index);
            if self.selected_entity_id.as_deref() == Some(&id) {
                self.selected_entity_id = self.selected_entity_ids.last().cloned();
            }
        } else {
            self.selected_entity_ids.push(id.clone());
            self.selected_entity_id = Some(id);
        }
    }

    /// Salva um snapshot da cena para permitir desfazer/refazer
    pub fn push_undo_state(&mut self) {
        self.undo_stack.push(self.scene.clone());
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Desfaz a última alteração conhecida da cena
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

    /// Refaz a última alteração desfeita
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

    /// Solicita confirmação para deletar a seleção atual
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

    /// Solicita confirmação para deletar uma entidade específica
    pub fn request_delete_entity(&mut self, id: String, name: String) {
        self.delete_confirmation = Some(DeleteTarget::Entities {
            ids: vec![id],
            label: format!("a entidade '{}'", name),
        });
    }

    /// Solicita confirmação para deletar um asset/pasta específica
    pub fn request_delete_asset(&mut self, path: PathBuf) {
        let label = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("asset")
            .to_string();
        self.delete_confirmation = Some(DeleteTarget::Asset { path, label });
    }

    /// Solicita abrir o diálogo de renomear um asset ou pasta
    pub fn request_rename_asset(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("asset")
            .to_string();
        self.rename_asset_dialog = Some((path, name));
    }

    /// Solicita abrir o diálogo de renomear a entidade principal selecionada
    pub fn request_rename_selected_entity(&mut self) {
        if let Some(id) = self.selected_entity_id.clone() {
            let name = find_entity_recursive(&self.scene.entities, &id)
                .map(|e| e.name.clone())
                .unwrap_or_else(|| "Entidade".to_string());
            self.rename_entity_dialog = Some((id, name));
        }
    }

    /// Salva o layout atual do editor em um arquivo JSON local.
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

    /// Cria uma entidade Sprite baseada em uma imagem dentro de /assets
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
            if let Some(Component::Transform(t)) = entity.components.get_mut(0) {
                t.x = x;
                t.y = y;
            }
        }

        let id = entity.id.clone();
        self.scene.add_entity(entity);
        self.select_single_entity(Some(id));
        Ok(name.to_string())
    }

    /// Exporta uma entidade selecionada como MATR reutilizável em /assets/matrs
    pub fn export_entity_as_matr(&mut self, entity: &Entity) -> std::io::Result<PathBuf> {
        let matrs_dir = self.project_root.join("assets/matrs");
        std::fs::create_dir_all(&matrs_dir)?;

        let filename = format!("{}.matr.json", sanitize_filename(&entity.name));
        let path = matrs_dir.join(filename);
        std::fs::write(&path, entity.to_json())?;
        self.assets.refresh();
        Ok(path)
    }

    /// Instancia uma entidade a partir de um MATR salvo em JSON
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
            .map_err(|e| format!("Falha ao ler MATR: {}", e))?;

        let mut entity = Entity::from_json(&content)
            .ok_or_else(|| "MATR inválido ou corrompido.".to_string())?;

        entity.regenerate_ids_recursive();
        entity.matr_source = path.file_name().and_then(|n| n.to_str()).map(|n| n.to_string());

        if let Some((x, y)) = world_pos {
            if let Some(Component::Transform(t)) = entity.components.get_mut(0) {
                t.x = x;
                t.y = y;
            }
        }

        let id = entity.id.clone();
        let name = entity.name.clone();
        self.scene.add_entity(entity);
        self.select_single_entity(Some(id));
        Ok(name)
    }

}

/// Busca recursiva de entidade mutável por ID
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

/// Busca recursiva de entidade imutável por ID
fn find_entity_recursive<'a>(
    entities: &'a [Entity],
    id: &str,
) -> Option<&'a Entity> {
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
        .and_then(|n| n.to_str())
        .map(|n| n.ends_with(".matr.json") || n.ends_with(".prefab.json"))
        .unwrap_or(false)
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Estilo visual escuro e limpo ──
        ctx.set_visuals(egui::Visuals::dark());

        // Atalhos globais do editor
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
            && ctx.input(|i| i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z))
        {
            self.undo_scene();
        }

        if !ctx.wants_keyboard_input()
            && ctx.input(|i| {
                (i.modifiers.command && i.key_pressed(egui::Key::Y))
                    || (i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Z))
            })
        {
            self.redo_scene();
        }

        // Se voltou para o modo de edição, descarta o preview do runtime.
        if self.play_state == EditorPlayState::Edit && self.runtime.active_scene.is_some() {
            self.runtime.stop();
        }

        // ── Menu superior ──
        menubar::show(self, ctx);

        // ── Painel esquerdo: Hierarquia ──
        let hierarchy_response = egui::SidePanel::left("hierarchy_panel")
            .default_width(self.layout.hierarchy_width)
            .resizable(true)
            .min_width(180.0)
            .max_width(520.0)
            .show(ctx, |ui| {
                hierarchy::show(self, ui);
            });
        self.layout.hierarchy_width = hierarchy_response.response.rect.width();

        // ── Painel direito: Inspector ──
        let inspector_response = egui::SidePanel::right("inspector_panel")
            .default_width(self.layout.inspector_width)
            .resizable(true)
            .min_width(220.0)
            .max_width(520.0)
            .show(ctx, |ui| {
                inspector::show(self, ui);
            });
        self.layout.inspector_width = inspector_response.response.rect.width();

        // ── Painel inferior: Asset Browser ──
        let asset_response = egui::TopBottomPanel::bottom("asset_panel")
            .default_height(self.layout.asset_height)
            .resizable(true)
            .min_height(150.0)
            .max_height(520.0)
            .show(ctx, |ui| {
                asset_browser::show(self, ui);
            });
        self.layout.asset_height = asset_response.response.rect.height();

        // ── Painel central: Cena 2D ──
        egui::CentralPanel::default().show(ctx, |ui| {
            scene_view::show(self, ui);
        });

        // ── Barra de status ──
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(&self.status_msg);
                ui.separator();
                let mode_label = match self.play_state {
                    EditorPlayState::Edit => "Modo: Edição",
                    EditorPlayState::Playing => "Modo: Play",
                    EditorPlayState::Paused => "Modo: Pausado",
                };
                ui.label(mode_label);

                if !self.selected_entity_ids.is_empty() {
                    ui.separator();
                    ui.label(format!("{} selecionada(s)", self.selected_entity_ids.len()));
                }
            });
        });

        // ── Diálogos modais ──
        self.show_new_script_dialog(ctx);
        self.show_new_folder_dialog(ctx);
        self.show_new_entity_dialog(ctx);
        self.show_rename_asset_dialog(ctx);
        self.show_rename_entity_dialog(ctx);
        self.show_delete_confirmation_dialog(ctx);

        // Runtime em janela separada enquanto estiver em Play/Pause
        runtime::show_viewport(self, ctx);

        // Soltou o mouse? encerra arraste pendente de asset e drag de entidade.
        if ctx.input(|i| i.pointer.any_released()) {
            self.dragging_asset_path = None;
            self.active_drag_entity_id = None;
        }
    }
}

impl EditorApp {
    fn show_new_script_dialog(&mut self, ctx: &egui::Context) {
        if let Some((folder, ref mut name)) = self.new_script_dialog.clone() {
            let mut open = true;
            egui::Window::new("Novo Script")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Nome do script (.rs):");
                    let mut buf = name.clone();
                    ui.text_edit_singleline(&mut buf);
                    if let Some((_, ref mut n)) = self.new_script_dialog {
                        *n = buf.clone();
                    }

                    ui.horizontal(|ui| {
                        if ui.button("✅ Criar").clicked() {
                            match self.assets.create_script(&folder, &buf) {
                                Ok(p) => {
                                    self.status_msg = format!("Script criado: {:?}", p.file_name().unwrap_or_default());
                                    self.assets.refresh();
                                    // Abrir no VS Code (Windows)
                                    #[cfg(target_os = "windows")]
                                    {
                                        let _ = std::process::Command::new("cmd").args(["/C", "code", &p.to_string_lossy()]).spawn();
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
        if let Some((parent, ref mut name)) = self.new_folder_dialog.clone() {
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
                                    self.status_msg = format!("Pasta criada: {}", buf);
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
        if let Some(ref mut name) = self.new_entity_dialog.clone() {
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
                            self.status_msg = format!("Entidade '{}' criada!", buf);
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
        if let Some((path, ref mut current_name)) = self.rename_asset_dialog.clone() {
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
                                        new_path.file_name().and_then(|n| n.to_str()).unwrap_or("asset")
                                    );
                                }
                                Err(e) => {
                                    self.status_msg = format!("❌ Erro ao renomear asset: {}", e);
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
        if let Some((entity_id, ref mut current_name)) = self.rename_entity_dialog.clone() {
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
                                    self.status_msg = "✏ Entidade renomeada.".to_string();
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
                            DeleteTarget::Asset { path, label } => {
                                match self.assets.delete_path(&path) {
                                    Ok(_) => {
                                        self.assets.refresh();
                                        if self.selected_asset.as_ref().map(|p| p == &path || p.starts_with(&path)).unwrap_or(false) {
                                            self.selected_asset = None;
                                        }
                                        self.status_msg = format!("🗑 '{}' removido.", label);
                                    }
                                    Err(e) => {
                                        self.status_msg = format!("❌ Erro ao deletar '{}': {}", label, e);
                                    }
                                }
                            }
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
