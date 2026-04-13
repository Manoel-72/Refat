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
mod animator_editor;

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    assets::AssetManager,
    core::{
        component::{Component, Sprite},
        entity::Entity,
        project::{ProjectConfig, create_basic_project_template_at},
        scene::Scene,
    },
    runtime::{self, RuntimeState, context::{RuntimeContext, RuntimePlayState}},
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


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomDockTab {
    Assets,
    Animator,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorScreen {
    ProjectHub,
    Editor,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectHubSession {
    pub last_project: Option<String>,
    #[serde(default)]
    pub recent_projects: Vec<String>,
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
    /// Clipboard para copiar/colar entidades (Ctrl+C / Ctrl+V)
    pub entity_clipboard: Option<crate::core::entity::Entity>,
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
    /// Diálogo de novo script Lua (caminho da pasta, nome do arquivo)
    pub new_lua_dialog: Option<(PathBuf, String)>,
    /// Diálogo de nova pasta (caminho pai, nome)
    pub new_folder_dialog: Option<(PathBuf, String)>,
    /// Diálogo de nova entidade (nome)
    pub new_entity_dialog: Option<String>,
    /// Popup para escolher nome ao criar Template Básico
    pub new_template_dialog: Option<String>,
    /// Pop-up inicial com a versão atual da engine
    pub show_version_popup: bool,
    /// Tela atual do aplicativo
    pub app_screen: EditorScreen,
    /// Sessão do hub de projetos
    pub project_hub_session: ProjectHubSession,
    /// Diálogo: novo projeto (nome, pasta base)
    pub new_project_dialog: Option<(String, String)>,
    /// Diálogo: abrir projeto (caminho)
    pub open_project_dialog: Option<String>,
    /// Aba ativa do dock inferior
    pub bottom_tab: BottomDockTab,
    /// Estado selecionado no workspace do Animator
    pub animator_selected_state: String,
    /// Posições normalizadas dos nós do Animator (0..1 dentro do graph)
    pub animator_node_positions: HashMap<String, [f32; 2]>,
    /// Estado sendo arrastado no graph do Animator
    pub animator_dragging_state: Option<String>,
    /// Offset local entre mouse e canto do nó durante arraste
    pub animator_drag_offset: [f32; 2],
    /// Zoom visual do graph do Animator
    pub animator_graph_zoom: f32,
    /// Abre o Animator em uma janela destacada para edição ampla
    pub animator_detached: bool,
    /// Estado origem de uma ligação em arraste no graph
    pub animator_link_drag_source: Option<String>,
}


impl EditorApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let engine_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let project_hub_session = load_project_hub_session(&engine_root).unwrap_or_default();
        let initial_project_root = project_hub_session
            .last_project
            .as_ref()
            .map(PathBuf::from)
            .filter(|p| p.join("project.json").exists())
            .unwrap_or_else(|| engine_root.clone());

        let _ = std::fs::create_dir_all(initial_project_root.join("assets/sprites"));
        let _ = std::fs::create_dir_all(initial_project_root.join("assets/scripts"));
        let _ = std::fs::create_dir_all(initial_project_root.join("assets/sounds"));
        let _ = std::fs::create_dir_all(initial_project_root.join("assets/matrs"));
        let _ = std::fs::create_dir_all(initial_project_root.join("assets/prefabs"));
        let _ = std::fs::create_dir_all(initial_project_root.join("assets/scenes"));

        let assets = AssetManager::new(initial_project_root.clone());
        let layout = load_editor_layout(&initial_project_root).unwrap_or_default();
        let project_config = ProjectConfig::load_or_create(&initial_project_root);

        let initial_scene = crate::serialization::scene_serializer::try_load_scene_from_path(
            &initial_project_root.join(&project_config.initial_scene),
        )
        .unwrap_or_else(|_| Scene::new("Cena Principal"));

        Self {
            scene: initial_scene.clone(),
            selected_entity_id: None,
            selected_entity_ids: Vec::new(),
            entity_clipboard: None,
            selected_asset: None,
            assets,
            project_root: initial_project_root,
            status_msg: "Selecione um projeto para começar.".to_string(),
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
            new_lua_dialog: None,
            new_folder_dialog: None,
            new_entity_dialog: None,
            new_template_dialog: None,
            show_version_popup: false,
            app_screen: EditorScreen::ProjectHub,
            project_hub_session,
            new_project_dialog: None,
            open_project_dialog: None,
            bottom_tab: BottomDockTab::Assets,
            animator_selected_state: "idle".to_string(),
            animator_node_positions: HashMap::new(),
            animator_dragging_state: None,
            animator_drag_offset: [0.0, 0.0],
            animator_graph_zoom: 1.0,
            animator_detached: false,
            animator_link_drag_source: None,
        }
    }


    fn current_project_name(&self) -> String {
        ProjectConfig::load_or_create(&self.project_root).name
    }

    fn update_window_title(&self, ctx: &egui::Context) {
        let title = format!("{} {} - {}", version::ENGINE_TITLE, version::ENGINE_VERSION, self.current_project_name());
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    fn enter_editor_for_current_project(&mut self, ctx: &egui::Context) {
        self.app_screen = EditorScreen::Editor;
        self.show_version_popup = true;
        self.update_window_title(ctx);
    }

    fn register_recent_project(&mut self, root: &Path) {
        let root_str = root.to_string_lossy().to_string();
        self.project_hub_session.recent_projects.retain(|p| p != &root_str);
        self.project_hub_session.recent_projects.insert(0, root_str.clone());
        self.project_hub_session.recent_projects.truncate(8);
        self.project_hub_session.last_project = Some(root_str);
        let _ = save_project_hub_session(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")), &self.project_hub_session);
    }

    fn load_project_root(&mut self, ctx: &egui::Context, root: PathBuf) -> Result<(), String> {
        if !root.join("project.json").exists() {
            return Err(format!("project.json não encontrado em {}", root.display()));
        }

        let project_config = ProjectConfig::load_or_create(&root);
        let initial_scene = crate::serialization::scene_serializer::try_load_scene_from_path(
            &root.join(&project_config.initial_scene),
        )
        .unwrap_or_else(|_| Scene::new("Cena Principal"));

        self.project_root = root.clone();
        self.assets = AssetManager::new(root.clone());
        self.assets.refresh();
        self.scene = initial_scene.clone();
        self.open_scenes = vec![OpenSceneDocument { scene: initial_scene, file_path: Some(root.join(&project_config.initial_scene)) }];
        self.active_scene_index = 0;
        self.selected_entity_id = None;
        self.selected_entity_ids.clear();
        self.selected_asset = None;
        self.sprite_textures.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.play_state = EditorPlayState::Edit;
        self.runtime.stop();
        self.runtime.window_open = false;
        self.asset_search.clear();
        self.status_msg = format!("✅ Projeto '{}' carregado.", project_config.name);
        self.register_recent_project(&root);
        self.enter_editor_for_current_project(ctx);
        Ok(())
    }

    fn create_new_project_at(&mut self, ctx: &egui::Context, project_name: &str, base_dir: &str) -> Result<(), String> {
        let base = PathBuf::from(base_dir.trim());
        if base_dir.trim().is_empty() {
            return Err("Escolha uma pasta base para o projeto.".to_string());
        }
        let folder_name = sanitize_filename(project_name.trim());
        let project_root = base.join(if folder_name.is_empty() { "MeuProjeto" } else { &folder_name });
        fs::create_dir_all(&project_root).map_err(|e| format!("Falha ao criar pasta do projeto: {}", e))?;
        create_basic_project_template_at(&project_root, project_name)
            .map_err(|e| format!("Falha ao criar template do projeto: {}", e))?;
        self.load_project_root(ctx, project_root)
    }

    fn close_current_project_to_hub(&mut self, ctx: &egui::Context) {
        self.runtime.stop();
        self.runtime.window_open = false;
        self.play_state = EditorPlayState::Edit;
        self.app_screen = EditorScreen::ProjectHub;
        self.show_version_popup = false;
        self.status_msg = "Projeto fechado. Escolha outro projeto ou crie um novo.".to_string();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!("{} {}", version::ENGINE_TITLE, version::ENGINE_VERSION)));
    }

    fn show_project_hub(&mut self, ctx: &egui::Context) {
        // ── Paleta — dark professional (mesmo tema do editor) ──
        let bg          = egui::Color32::from_rgb(15, 18, 24);
        let panel_bg    = egui::Color32::from_rgb(22, 27, 36);
        let card_bg     = egui::Color32::from_rgb(28, 34, 46);
        let accent      = egui::Color32::from_rgb(88, 166, 255);
        let accent_green= egui::Color32::from_rgb(63, 185, 80);
        let accent_org  = egui::Color32::from_rgb(210, 105, 30);
        let accent_red  = egui::Color32::from_rgb(200, 50, 50);
        let text_dim    = egui::Color32::from_rgb(120, 135, 155);
        let text_muted  = egui::Color32::from_rgb(75, 88, 108);
        let border      = egui::Color32::from_rgb(38, 46, 62);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(bg))
            .show(ctx, |ui| {

            let total_w = ui.available_width();
            let total_h = ui.available_height();

            // ── Header strip ──
            let header_h = 52.0_f32;
            let (header_rect, _) = ui.allocate_exact_size(
                egui::vec2(total_w, header_h), egui::Sense::hover(),
            );
            ui.painter().rect_filled(header_rect, 0.0, panel_bg);
            // linha inferior sutil
            ui.painter().line_segment(
                [header_rect.left_bottom(), header_rect.right_bottom()],
                egui::Stroke::new(1.0, border),
            );
            // Título + versão no header
            let title_text = format!("{} {}", version::ENGINE_TITLE, version::ENGINE_VERSION);
            ui.painter().text(
                egui::pos2(header_rect.left() + 22.0, header_rect.center().y - 7.0),
                egui::Align2::LEFT_CENTER,
                &title_text,
                egui::FontId::proportional(16.0),
                egui::Color32::WHITE,
            );
            ui.painter().text(
                egui::pos2(header_rect.left() + 22.0, header_rect.center().y + 10.0),
                egui::Align2::LEFT_CENTER,
                "Game Engine 2D — Hub de Projetos",
                egui::FontId::proportional(11.0),
                text_dim,
            );
            // Pequeno badge de versão
            let badge_text = version::ENGINE_VERSION;
            let badge_rect = egui::Rect::from_min_size(
                egui::pos2(header_rect.right() - 100.0, header_rect.center().y - 10.0),
                egui::vec2(88.0, 20.0),
            );
            ui.painter().rect_filled(badge_rect, 4.0, egui::Color32::from_rgb(30, 50, 80));
            ui.painter().rect_stroke(badge_rect, 4.0, egui::Stroke::new(1.0, accent));
            ui.painter().text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                badge_text,
                egui::FontId::proportional(11.0),
                accent,
            );

            // ── Corpo: coluna esquerda (projetos) + coluna direita (ações) ──
            let body_y      = header_h + 1.0;
            let body_h      = total_h - body_y - 28.0; // 28 = status bar
            let sidebar_w   = 260.0_f32;
            let content_w   = total_w - sidebar_w - 1.0;

            // ── Sidebar direita de ações ──
            let sidebar_x   = total_w - sidebar_w;
            let sidebar_rect = egui::Rect::from_min_size(
                egui::pos2(sidebar_x, body_y),
                egui::vec2(sidebar_w, body_h),
            );
            ui.painter().rect_filled(sidebar_rect, 0.0, panel_bg);
            ui.painter().line_segment(
                [sidebar_rect.left_top(), sidebar_rect.left_bottom()],
                egui::Stroke::new(1.0, border),
            );

            // Ações sidebar
            let action_defs: &[(&str, &str, egui::Color32)] = &[
                ("＋", "Novo Projeto",       accent_green),
                ("▣",  "Abrir Projeto",      accent),
                ("→",  "Entrar no Editor",   egui::Color32::from_rgb(88, 166, 255)),
                ("⏻",  "Sair",              accent_red),
            ];

            let mut sb_ui_rect = egui::Rect::from_min_size(
                egui::pos2(sidebar_x + 16.0, body_y + 20.0),
                egui::vec2(sidebar_w - 32.0, body_h - 40.0),
            );

            for (icon, label, color) in action_defs {
                let btn_h = 46.0_f32;
                let btn_rect = egui::Rect::from_min_size(
                    sb_ui_rect.min,
                    egui::vec2(sb_ui_rect.width(), btn_h),
                );

                let btn_id = egui::Id::new(format!("hub_action_{}", label));
                let response = ui.interact(btn_rect, btn_id, egui::Sense::click());

                let bg_color = if response.hovered() {
                    egui::Color32::from_rgb(38, 50, 70)
                } else {
                    card_bg
                };

                ui.painter().rect_filled(btn_rect, 6.0, bg_color);
                ui.painter().rect_stroke(btn_rect, 6.0, egui::Stroke::new(1.0, border));

                // ícone círculo colorido
                let icon_center = egui::pos2(btn_rect.left() + 24.0, btn_rect.center().y);
                ui.painter().circle_filled(icon_center, 14.0, *color);
                ui.painter().text(
                    icon_center,
                    egui::Align2::CENTER_CENTER,
                    icon,
                    egui::FontId::proportional(14.0),
                    egui::Color32::WHITE,
                );
                // label
                ui.painter().text(
                    egui::pos2(btn_rect.left() + 46.0, btn_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(13.0),
                    egui::Color32::WHITE,
                );
                // seta
                ui.painter().text(
                    egui::pos2(btn_rect.right() - 12.0, btn_rect.center().y),
                    egui::Align2::CENTER_CENTER,
                    "›",
                    egui::FontId::proportional(18.0),
                    text_muted,
                );

                if response.clicked() {
                    match *label {
                        "Novo Projeto" => {
                            let base = std::env::current_dir()
                                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                .to_string_lossy()
                                .to_string();
                            self.new_project_dialog = Some(("MeuProjeto".to_string(), base));
                        }
                        "Abrir Projeto" => {
                            self.open_project_dialog = Some(String::new());
                        }
                        "Entrar no Editor" => {
                            self.enter_editor_for_current_project(ctx);
                        }
                        "Sair" => {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        _ => {}
                    }
                }

                sb_ui_rect.min.y += btn_h + 8.0;
            }

            // ── Coluna esquerda: projetos ──
            let left_rect = egui::Rect::from_min_size(
                egui::pos2(0.0, body_y),
                egui::vec2(content_w, body_h),
            );

            // Último projeto — card em destaque
            let last_card_h = 90.0_f32;
            let last_card_rect = egui::Rect::from_min_size(
                egui::pos2(left_rect.left() + 16.0, left_rect.top() + 16.0),
                egui::vec2(left_rect.width() - 32.0, last_card_h),
            );

            ui.painter().rect_filled(last_card_rect, 8.0, card_bg);
            ui.painter().rect_stroke(last_card_rect, 8.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 100, 170)));

            // ícone pasta grande
            let folder_cx = last_card_rect.left() + 52.0;
            let folder_cy = last_card_rect.center().y;
            // sombra do folder
            ui.painter().circle_filled(egui::pos2(folder_cx, folder_cy), 26.0, egui::Color32::from_rgb(20, 30, 50));
            ui.painter().text(
                egui::pos2(folder_cx, folder_cy),
                egui::Align2::CENTER_CENTER,
                "▶",
                egui::FontId::proportional(22.0),
                accent_green,
            );

            match self.project_hub_session.last_project.clone() {
                Some(ref path) => {
                    let short = std::path::Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(path.as_str());
                    ui.painter().text(
                        egui::pos2(last_card_rect.left() + 96.0, folder_cy - 12.0),
                        egui::Align2::LEFT_CENTER,
                        short,
                        egui::FontId::proportional(14.0),
                        egui::Color32::WHITE,
                    );
                    ui.painter().text(
                        egui::pos2(last_card_rect.left() + 96.0, folder_cy + 6.0),
                        egui::Align2::LEFT_CENTER,
                        path.as_str(),
                        egui::FontId::proportional(10.0),
                        text_dim,
                    );

                    // botão play
                    let play_btn = egui::Rect::from_min_size(
                        egui::pos2(last_card_rect.left() + 96.0, folder_cy + 22.0),
                        egui::vec2(130.0, 22.0),
                    );
                    let play_id = egui::Id::new("hub_continue_btn");
                    let play_resp = ui.interact(play_btn, play_id, egui::Sense::click());
                    let play_col = if play_resp.hovered() {
                        egui::Color32::from_rgb(46, 120, 220)
                    } else {
                        egui::Color32::from_rgb(36, 100, 200)
                    };
                    ui.painter().rect_filled(play_btn, 4.0, play_col);
                    ui.painter().text(
                        play_btn.center(),
                        egui::Align2::CENTER_CENTER,
                        "▶  Continuar",
                        egui::FontId::proportional(11.0),
                        egui::Color32::WHITE,
                    );
                    if play_resp.clicked() {
                        let root = std::path::PathBuf::from(path.clone());
                        if let Err(e) = self.load_project_root(ctx, root) {
                            self.status_msg = format!("❌ {}", e);
                        }
                    }
                }
                None => {
                    ui.painter().text(
                        egui::pos2(last_card_rect.left() + 96.0, folder_cy),
                        egui::Align2::LEFT_CENTER,
                        "Nenhum projeto recente",
                        egui::FontId::proportional(13.0),
                        text_dim,
                    );
                }
            }

            // ── Seção "Recentes" ──
            let section_y = last_card_rect.bottom() + 20.0;
            ui.painter().text(
                egui::pos2(left_rect.left() + 16.0, section_y),
                egui::Align2::LEFT_TOP,
                "PROJETOS RECENTES",
                egui::FontId::proportional(10.0),
                text_muted,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(left_rect.left() + 16.0, section_y + 14.0),
                    egui::pos2(left_rect.right() - 16.0, section_y + 14.0),
                ],
                egui::Stroke::new(1.0, border),
            );

            let recent_start_y = section_y + 22.0;
            let recent = self.project_hub_session.recent_projects.clone();
            if recent.is_empty() {
                ui.painter().text(
                    egui::pos2(left_rect.left() + 16.0, recent_start_y + 10.0),
                    egui::Align2::LEFT_TOP,
                    "Nenhum projeto recente.",
                    egui::FontId::proportional(12.0),
                    text_dim,
                );
            } else {
                for (i, path) in recent.iter().enumerate() {
                    let row_h = 44.0_f32;
                    let row_rect = egui::Rect::from_min_size(
                        egui::pos2(left_rect.left() + 16.0, recent_start_y + i as f32 * (row_h + 6.0)),
                        egui::vec2(left_rect.width() - 32.0, row_h),
                    );

                    if row_rect.bottom() > left_rect.bottom() - 8.0 { break; }

                    let row_id = egui::Id::new(format!("hub_recent_{}", i));
                    let row_resp = ui.interact(row_rect, row_id, egui::Sense::click());

                    let row_bg = if row_resp.hovered() {
                        egui::Color32::from_rgb(34, 42, 58)
                    } else {
                        egui::Color32::from_rgb(24, 30, 42)
                    };
                    ui.painter().rect_filled(row_rect, 5.0, row_bg);
                    ui.painter().rect_stroke(row_rect, 5.0, egui::Stroke::new(1.0, border));

                    // folder icon
                    let folder_colors = [
                        egui::Color32::from_rgb(210, 140, 30),
                        egui::Color32::from_rgb(30, 140, 100),
                        egui::Color32::from_rgb(100, 80, 200),
                        egui::Color32::from_rgb(200, 80, 80),
                    ];
                    let fc = folder_colors[i % folder_colors.len()];
                    ui.painter().text(
                        egui::pos2(row_rect.left() + 22.0, row_rect.center().y),
                        egui::Align2::CENTER_CENTER,
                        "▣",
                        egui::FontId::proportional(16.0),
                        fc,
                    );

                    let short = std::path::Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(path.as_str());
                    ui.painter().text(
                        egui::pos2(row_rect.left() + 40.0, row_rect.center().y - 6.0),
                        egui::Align2::LEFT_CENTER,
                        short,
                        egui::FontId::proportional(13.0),
                        egui::Color32::WHITE,
                    );
                    ui.painter().text(
                        egui::pos2(row_rect.left() + 40.0, row_rect.center().y + 8.0),
                        egui::Align2::LEFT_CENTER,
                        path.as_str(),
                        egui::FontId::proportional(10.0),
                        text_dim,
                    );
                    // seta abrir
                    ui.painter().text(
                        egui::pos2(row_rect.right() - 14.0, row_rect.center().y),
                        egui::Align2::CENTER_CENTER,
                        "›",
                        egui::FontId::proportional(18.0),
                        text_muted,
                    );

                    if row_resp.clicked() || row_resp.double_clicked() {
                        if let Err(e) = self.load_project_root(ctx, std::path::PathBuf::from(path.clone())) {
                            self.status_msg = format!("❌ {}", e);
                        }
                    }
                }
            }

            // ── Status bar inferior ──
            let status_rect = egui::Rect::from_min_size(
                egui::pos2(0.0, total_h - 26.0),
                egui::vec2(total_w, 26.0),
            );
            ui.painter().rect_filled(status_rect, 0.0, panel_bg);
            ui.painter().line_segment(
                [status_rect.left_top(), status_rect.right_top()],
                egui::Stroke::new(1.0, border),
            );
            let status_text = if self.status_msg.is_empty() {
                "Selecione um projeto para começar.".to_string()
            } else {
                self.status_msg.clone()
            };
            ui.painter().text(
                egui::pos2(status_rect.right() - 12.0, status_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                &status_text,
                egui::FontId::proportional(11.0),
                text_dim,
            );
        });

        self.show_new_project_dialog(ctx);
        self.show_open_project_dialog(ctx);
    }

    fn show_new_project_dialog(&mut self, ctx: &egui::Context) {
        if let Some((name, base_dir)) = self.new_project_dialog.clone() {
            let mut open = true;
            egui::Window::new("Novo Projeto")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    let mut name_buf = name.clone();
                    let mut dir_buf = base_dir.clone();
                    ui.label("Nome do projeto:");
                    ui.text_edit_singleline(&mut name_buf);
                    ui.label("Pasta base:");
                    ui.text_edit_singleline(&mut dir_buf);
                    if ui.button("Escolher pasta...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            dir_buf = folder.to_string_lossy().to_string();
                        }
                    }
                    if let Some((n, d)) = self.new_project_dialog.as_mut() {
                        *n = name_buf.clone();
                        *d = dir_buf.clone();
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Criar").clicked() {
                            if let Err(e) = self.create_new_project_at(ctx, &name_buf, &dir_buf) {
                                self.status_msg = format!("❌ {}", e);
                            }
                            self.new_project_dialog = None;
                        }
                        if ui.button("Cancelar").clicked() {
                            self.new_project_dialog = None;
                        }
                    });
                });
            if !open { self.new_project_dialog = None; }
        }
    }

    fn show_open_project_dialog(&mut self, ctx: &egui::Context) {
        if let Some(path) = self.open_project_dialog.clone() {
            let mut open = true;
            egui::Window::new("Abrir Projeto")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    let mut buf = path.clone();
                    ui.label("Pasta do projeto ou arquivo project.json:");
                    ui.text_edit_singleline(&mut buf);
                    if ui.button("Escolher...").clicked() {
                        if let Some(file) = rfd::FileDialog::new().add_filter("Projeto RS2BR", &["json"]).pick_file() {
                            buf = file.to_string_lossy().to_string();
                        } else if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            buf = folder.to_string_lossy().to_string();
                        }
                    }
                    if let Some(v) = self.open_project_dialog.as_mut() { *v = buf.clone(); }
                    ui.horizontal(|ui| {
                        if ui.button("Abrir").clicked() {
                            let mut root = PathBuf::from(buf.trim());
                            if root.is_file() { root = root.parent().unwrap_or(Path::new(".")).to_path_buf(); }
                            if let Err(e) = self.load_project_root(ctx, root) {
                                self.status_msg = format!("❌ {}", e);
                            }
                            self.open_project_dialog = None;
                        }
                        if ui.button("Cancelar").clicked() {
                            self.open_project_dialog = None;
                        }
                    });
                });
            if !open { self.open_project_dialog = None; }
        }
    }


    fn show_version_popup(&mut self, ctx: &egui::Context) {
        if !self.show_version_popup {
            return;
        }

        let panel_bg  = egui::Color32::from_rgb(18, 23, 35);
        let accent     = egui::Color32::from_rgb(88, 166, 255);
        let text_dim   = egui::Color32::from_rgb(120, 136, 158);
        let border     = egui::Color32::from_rgb(38, 52, 78);

        egui::Window::new("##version_popup")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(egui::Frame::none()
                .fill(panel_bg)
                .rounding(10.0)
                .stroke(egui::Stroke::new(1.0, border))
                .inner_margin(egui::Margin::same(24.0)))
            .show(ctx, |ui| {
                ui.set_min_width(320.0);

                // cabeçalho
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot.center(), 5.0, accent);
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(version::ENGINE_TITLE)
                            .size(16.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                });
                ui.add_space(4.0);

                // versão badge
                let ver_text = format!("Versão  {}", version::ENGINE_VERSION);
                ui.label(egui::RichText::new(&ver_text).size(13.0).color(accent));
                ui.add_space(8.0);

                ui.label(egui::RichText::new("Engine 2D — Game Layer ativo").size(11.0).color(text_dim));
                ui.label(egui::RichText::new("Use Lua para scripts · JSON para cenas · RS2 para shaders").size(10.0).color(text_dim));

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                ui.vertical_centered(|ui| {
                    let ok_btn = egui::Button::new(
                        egui::RichText::new("  Entrar no Editor  ").color(egui::Color32::WHITE).size(12.0)
                    )
                    .fill(egui::Color32::from_rgb(36, 100, 200))
                    .rounding(5.0)
                    .min_size(egui::vec2(160.0, 30.0));
                    if ui.add(ok_btn).clicked() {
                        self.show_version_popup = false;
                        self.status_msg = version::startup_message();
                    }
                });
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

    fn show_new_lua_dialog(&mut self, ctx: &egui::Context) {
        if let Some((folder, name)) = self.new_lua_dialog.clone() {
            let mut open = true;
            egui::Window::new("Novo Script Lua")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Nome do script (.lua):");
                    let mut buf = name.clone();
                    ui.text_edit_singleline(&mut buf);

                    if let Some((_, ref mut n)) = self.new_lua_dialog {
                        *n = buf.clone();
                    }

                    ui.horizontal(|ui| {
                        if ui.button("✅ Criar").clicked() {
                            match self.assets.create_lua_script_file(&folder, &buf) {
                                Ok(p) => {
                                    self.assets.refresh();
                                    self.selected_asset = Some(p.clone());
                                    self.status_msg = format!(
                                        "🌙 Script Lua criado: {}",
                                        p.file_name().and_then(|n| n.to_str()).unwrap_or("script.lua")
                                    );
                                    #[cfg(target_os = "windows")]
                                    {
                                        let _ = std::process::Command::new("cmd")
                                            .args(["/C", "code", &p.to_string_lossy()])
                                            .spawn();
                                    }
                                }
                                Err(e) => self.status_msg = format!("Erro: {}", e),
                            }
                            self.new_lua_dialog = None;
                        }

                        if ui.button("❌ Cancelar").clicked() {
                            self.new_lua_dialog = None;
                        }
                    });
                });

            if !open {
                self.new_lua_dialog = None;
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

    fn show_new_template_dialog(&mut self, ctx: &egui::Context) {
        if let Some(name) = self.new_template_dialog.clone() {
            let mut open = true;
            egui::Window::new("Criar Template Básico")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Nome do projeto / template:");
                    let mut buf = name.clone();
                    ui.text_edit_singleline(&mut buf);
                    if let Some(ref mut n) = self.new_template_dialog {
                        *n = buf.clone();
                    }
                    ui.horizontal(|ui| {
                        if ui.button("✅ Criar").clicked() {
                            match self.assets.create_basic_project_template(&buf) {
                                Ok(path) => {
                                    self.assets.refresh();
                                    self.selected_asset = Some(path.clone());
                                    self.status_msg = format!("📦 Template '{}' criado.", buf.trim());
                                }
                                Err(e) => self.status_msg = format!("❌ {}", e),
                            }
                            self.new_template_dialog = None;
                        }
                        if ui.button("❌ Cancelar").clicked() {
                            self.new_template_dialog = None;
                        }
                    });
                });
            if !open {
                self.new_template_dialog = None;
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

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Tema — RS2BR-Engine V0.9.7.5 ──
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::from_rgb(210, 220, 235));
        // Fundos dos painéis
        visuals.panel_fill              = egui::Color32::from_rgb(18, 22, 32);   // painel lateral
        visuals.faint_bg_color          = egui::Color32::from_rgb(24, 30, 42);
        visuals.extreme_bg_color        = egui::Color32::from_rgb(10, 13, 20);   // viewport grid
        visuals.code_bg_color           = egui::Color32::from_rgb(16, 22, 34);
        visuals.window_fill             = egui::Color32::from_rgb(22, 28, 40);
        // Widgets
        visuals.widgets.noninteractive.bg_fill      = egui::Color32::from_rgb(24, 30, 44);
        visuals.widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(28, 36, 50);
        visuals.widgets.inactive.bg_fill            = egui::Color32::from_rgb(32, 40, 58);
        visuals.widgets.inactive.weak_bg_fill       = egui::Color32::from_rgb(36, 45, 64);
        visuals.widgets.hovered.bg_fill             = egui::Color32::from_rgb(44, 58, 84);
        visuals.widgets.hovered.weak_bg_fill        = egui::Color32::from_rgb(48, 64, 90);
        visuals.widgets.active.bg_fill              = egui::Color32::from_rgb(40, 100, 200);
        visuals.widgets.active.weak_bg_fill         = egui::Color32::from_rgb(32, 84, 175);
        visuals.widgets.open.bg_fill                = egui::Color32::from_rgb(30, 38, 56);
        // Seleção
        visuals.selection.bg_fill                   = egui::Color32::from_rgb(40, 100, 200);
        visuals.selection.stroke.color              = egui::Color32::from_rgb(88, 166, 255);
        visuals.hyperlink_color                     = egui::Color32::from_rgb(88, 166, 255);
        // Bordas e strokes
        visuals.window_stroke.color                         = egui::Color32::from_rgb(40, 52, 72);
        visuals.widgets.noninteractive.bg_stroke.color      = egui::Color32::from_rgb(36, 46, 66);
        visuals.widgets.inactive.bg_stroke.color            = egui::Color32::from_rgb(42, 54, 76);
        visuals.widgets.hovered.bg_stroke.color             = egui::Color32::from_rgb(88, 166, 255);
        visuals.widgets.active.bg_stroke.color              = egui::Color32::from_rgb(120, 190, 255);
        visuals.widgets.open.bg_stroke.color                = egui::Color32::from_rgb(50, 64, 88);
        visuals.widgets.noninteractive.fg_stroke.color      = egui::Color32::from_rgb(40, 52, 72);
        // Rounding — minimalista mas com cantos levemente arredondados
        visuals.window_rounding                             = 6.0.into();
        visuals.menu_rounding                               = 5.0.into();
        visuals.widgets.noninteractive.rounding             = 3.0.into();
        visuals.widgets.inactive.rounding                   = 3.0.into();
        visuals.widgets.hovered.rounding                    = 3.0.into();
        visuals.widgets.active.rounding                     = 3.0.into();
        visuals.widgets.open.rounding                       = 3.0.into();
        ctx.set_visuals(visuals);

        if self.app_screen == EditorScreen::ProjectHub {
            self.show_project_hub(ctx);
            return;
        }

        self.sync_active_scene_document();
        self.update_window_title(ctx);

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

        // Ctrl+C — copia entidade selecionada
        if !ctx.wants_keyboard_input()
            && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C))
        {
            if let Some(id) = &self.selected_entity_id.clone() {
                if let Some(entity) = self.scene.find_entity(id).cloned() {
                    self.entity_clipboard = Some(entity);
                    self.status_msg = "📋 Entidade copiada.".to_string();
                }
            }
        }

        // Ctrl+V — cola entidade copiada (com novo ID e nome sufixado)
        if !ctx.wants_keyboard_input()
            && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::V))
        {
            if let Some(template) = self.entity_clipboard.clone() {
                self.push_undo_state();
                let mut cloned = template.clone();
                cloned.id = uuid::Uuid::new_v4().to_string();
                cloned.name = format!("{} (cópia)", template.name);
                // Desloca levemente para não sobrepor exatamente
                if let Some(t) = cloned.transform_mut() {
                    t.x += 32.0;
                    t.y -= 32.0;
                }
                let new_id = cloned.id.clone();
                self.scene.add_entity(cloned);
                self.select_single_entity(Some(new_id));
                self.status_msg = format!("📋 '{}' colada.", template.name);
            }
        }

        if self.play_state == EditorPlayState::Edit && self.runtime.active_scene.is_some() {
            self.runtime.stop();
        }

        menubar::show(self, ctx);

        let hierarchy_response = egui::SidePanel::left("hierarchy_panel")
            .default_width(self.layout.hierarchy_width)
            .resizable(true)
            .min_width(180.0)
            .max_width(520.0)
            .show(ctx, |ui| {
                hierarchy::show(self, ui);
            });
        self.layout.hierarchy_width = hierarchy_response.response.rect.width();

        let inspector_response = egui::SidePanel::right("inspector_panel")
            .default_width(self.layout.inspector_width)
            .resizable(true)
            .min_width(220.0)
            .max_width(520.0)
            .show(ctx, |ui| {
                inspector::show(self, ui);
            });
        self.layout.inspector_width = inspector_response.response.rect.width();

        let asset_response = egui::TopBottomPanel::bottom("asset_panel")
            .default_height(self.layout.asset_height.max(260.0))
            .resizable(true)
            .min_height(170.0)
            .max_height(900.0)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    // Tab Assets
                    let assets_selected = self.bottom_tab == BottomDockTab::Assets;
                    let assets_color = if assets_selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(120, 136, 158) };
                    let assets_bg = if assets_selected { egui::Color32::from_rgb(32, 48, 72) } else { egui::Color32::TRANSPARENT };
                    let assets_btn = egui::Button::new(
                        egui::RichText::new("Assets").size(12.0).color(assets_color)
                    ).fill(assets_bg).rounding(4.0).min_size(egui::vec2(64.0, 22.0));
                    if ui.add(assets_btn).clicked() {
                        self.bottom_tab = BottomDockTab::Assets;
                    }
                    // Tab Animator
                    let animator_selected = self.bottom_tab == BottomDockTab::Animator;
                    let anim_color = if animator_selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(120, 136, 158) };
                    let anim_bg = if animator_selected { egui::Color32::from_rgb(32, 48, 72) } else { egui::Color32::TRANSPARENT };
                    let anim_btn = egui::Button::new(
                        egui::RichText::new("Animator").size(12.0).color(anim_color)
                    ).fill(anim_bg).rounding(4.0).min_size(egui::vec2(72.0, 22.0));
                    let animator_tab = ui.add(anim_btn);
                    if animator_tab.clicked() {
                        self.bottom_tab = BottomDockTab::Animator;
                    }
                    if animator_tab.double_clicked() {
                        self.bottom_tab = BottomDockTab::Animator;
                        self.animator_detached = true;
                    }
                });
                ui.separator();

                match self.bottom_tab {
                    BottomDockTab::Assets => asset_browser::show(self, ui),
                    BottomDockTab::Animator => animator_editor::show(self, ui),
                }
            });
        self.layout.asset_height = asset_response.response.rect.height();



        if self.animator_detached {
            let mut open = self.animator_detached;
            egui::Window::new("🎞 Animator — Workspace")
                .open(&mut open)
                .default_size(egui::vec2(1180.0, 760.0))
                .min_size(egui::vec2(760.0, 520.0))
                .resizable(true)
                .vscroll(true)
                .hscroll(true)
                .show(ctx, |ui| {
                    animator_editor::show(self, ui);
                });
            self.animator_detached = open;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            scene_view::show(self, ui);
        });

        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(12, 16, 26))
                .inner_margin(egui::Margin { left: 10.0, right: 10.0, top: 3.0, bottom: 3.0 }))
            .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // ── Mensagem de status com ícone colorido ──
                let level = infer_status_level(&self.status_msg);
                let (icon, color) = status_visuals(level);
                let trimmed_message = self
                    .status_msg
                    .trim_start_matches(|c: char| matches!(c, '✅' | '⚠' | '❌' | 'ℹ' | '✔' | ' '));
                ui.label(egui::RichText::new(format!("{} {}", icon, trimmed_message)).size(11.0).color(color));

                // ── Direita: modo, warnings, asset, seleção ──
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(4.0);

                    // Versão compacta
                    ui.label(
                        egui::RichText::new(version::ENGINE_VERSION)
                            .size(10.0)
                            .color(egui::Color32::from_rgb(60, 80, 110)),
                    );
                    ui.separator();

                    // Seleção múltipla
                    if !self.selected_entity_ids.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("{} sel.", self.selected_entity_ids.len()))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(88, 166, 255)),
                        );
                        ui.separator();
                    }

                    // Asset selecionado
                    if let Some(asset) = self.selected_asset.as_ref() {
                        let record = self.assets.asset_record_for(asset);
                        let asset_color = if record.validation.is_valid {
                            egui::Color32::from_rgb(80, 190, 100)
                        } else {
                            egui::Color32::from_rgb(220, 80, 80)
                        };
                        ui.label(egui::RichText::new(&record.name).size(11.0).color(asset_color));
                        ui.separator();
                    }

                    // Warnings
                    let warning_count = self.warning_count();
                    if warning_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("⚠ {}", warning_count))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(230, 160, 30)),
                        );
                        ui.separator();
                    }

                    // Modo play
                    let (mode_label, mode_color) = match self.play_state {
                        EditorPlayState::Edit    => ("● Edição",  egui::Color32::from_rgb(80, 110, 160)),
                        EditorPlayState::Playing => ("▶ Play",    egui::Color32::from_rgb(63, 185, 80)),
                        EditorPlayState::Paused  => ("⏸ Pausado", egui::Color32::from_rgb(200, 160, 30)),
                    };
                    ui.label(egui::RichText::new(mode_label).size(11.0).color(mode_color));
                });
            });
        });

        self.show_new_project_dialog(ctx);
        self.show_open_project_dialog(ctx);
        self.show_new_script_dialog(ctx);
        self.show_new_lua_dialog(ctx);
        self.show_new_folder_dialog(ctx);
        self.show_new_entity_dialog(ctx);
        self.show_new_template_dialog(ctx);
        self.show_rename_asset_dialog(ctx);
        self.show_rename_entity_dialog(ctx);
        self.show_delete_confirmation_dialog(ctx);
        self.show_version_popup(ctx);

        {
            // Desacopla runtime do editor via RuntimeContext — V0.9
            // Swap runtime out para evitar double-borrow de self
            let mut rt = std::mem::replace(&mut self.runtime, RuntimeState::new());
            runtime::show_viewport(self, &mut rt, ctx);
            self.runtime = rt;
        }

        if ctx.input(|i| i.pointer.any_released()) {
            self.dragging_asset_path = None;
            self.active_drag_entity_id = None;
        }
    }
}

fn project_hub_session_path(engine_root: &Path) -> PathBuf {
    engine_root.join("editor_project_hub.json")
}

fn load_project_hub_session(engine_root: &Path) -> Option<ProjectHubSession> {
    let path = project_hub_session_path(engine_root);
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_project_hub_session(engine_root: &Path, session: &ProjectHubSession) -> std::io::Result<()> {
    let path = project_hub_session_path(engine_root);
    let json = serde_json::to_string_pretty(session)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    fs::write(path, json)
}

// ── RuntimeContext impl ──────────────────────────────────────

impl RuntimeContext for EditorApp {
    fn play_state(&self) -> RuntimePlayState {
        match self.play_state {
            EditorPlayState::Edit    => RuntimePlayState::Edit,
            EditorPlayState::Playing => RuntimePlayState::Playing,
            EditorPlayState::Paused  => RuntimePlayState::Paused,
        }
    }

    fn set_play_state(&mut self, state: RuntimePlayState) {
        self.play_state = match state {
            RuntimePlayState::Edit    => EditorPlayState::Edit,
            RuntimePlayState::Playing => EditorPlayState::Playing,
            RuntimePlayState::Paused  => EditorPlayState::Paused,
        };
    }

    fn project_root(&self) -> &std::path::Path {
        &self.project_root
    }

    fn active_scene_snapshot(&self) -> &crate::core::scene::Scene {
        &self.scene
    }

    fn scene_file_candidates(&self) -> Vec<std::path::PathBuf> {
        // delegates to the existing pub method
        EditorApp::scene_file_candidates(self)
    }

    fn scene_snapshot_by_path(&self, path: &std::path::Path) -> Option<(crate::core::scene::Scene, Option<std::path::PathBuf>)> {
        let normalized = path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
        self.open_scenes.iter().find(|doc| {
            doc.file_path
                .as_ref()
                .map(|p| p.to_string_lossy().replace('\\', "/").to_ascii_lowercase() == normalized)
                .unwrap_or(false)
        }).map(|doc| (doc.scene.clone(), doc.file_path.clone()))
    }

    fn set_status(&mut self, msg: String) {
        self.status_msg = msg;
    }

    fn sprite_textures(&mut self) -> &mut std::collections::HashMap<String, eframe::egui::TextureHandle> {
        &mut self.sprite_textures
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