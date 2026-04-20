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
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
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
    Console,
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

#[derive(Debug)]
enum BuildWorkerMessage {
    Step(String),
    Log(String),
    Finished(Result<PathBuf, String>),
}

/// Resultado do diálogo de escolha de destino da build, recebido via canal.
#[derive(Debug)]
enum BuildDialogMessage {
    Selected(PathBuf),
    Cancelled,
}

#[derive(Debug)]
struct BuildJobState {
    receiver: Receiver<BuildWorkerMessage>,
    current_step: String,
    log_lines: Vec<String>,
    started_at: Instant,
    progress: f32,
    estimated_total: f32,
    min_visible_until: Instant,
    pending_result: Option<Result<PathBuf, String>>,
}

/// Estado enquanto o diálogo de destino ainda não respondeu.
struct BuildDialogPending {
    receiver: Receiver<BuildDialogMessage>,
    /// Dados pré-computados antes de abrir o diálogo (para não bloquear nada)
    engine_root: PathBuf,
    bin_name: String,
    engine_exe_name: String,
    project_name: String,
    safe_project_name: String,
    assets_src: PathBuf,
    save_src: PathBuf,
    project_json_src: PathBuf,
    saved_scene_name: String,
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
    /// Busca textual do Inspector para filtrar componentes
    pub inspector_search: String,
    /// Histórico simples do console do editor
    pub console_history: Vec<String>,
    /// Última mensagem registrada no console
    pub last_status_snapshot: String,
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
    /// Campo temporário para renomear o estado selecionado no Animator
    pub animator_state_rename_buffer: String,
    /// Build standalone em execução no momento
    build_job: Option<BuildJobState>,
    /// Diálogo de escolha de destino em aberto (não bloqueia o editor)
    build_dialog_pending: Option<BuildDialogPending>,
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
        let mut layout = load_editor_layout(&initial_project_root).unwrap_or_default();
        layout.hierarchy_width = layout.hierarchy_width.clamp(200.0, 380.0);
        layout.inspector_width = layout.inspector_width.clamp(260.0, 380.0);
        layout.asset_height = layout.asset_height.clamp(190.0, 420.0);
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
            inspector_search: String::new(),
            console_history: vec!["ℹ Editor iniciado.".to_string()],
            last_status_snapshot: "Selecione um projeto para começar.".to_string(),
            animator_selected_state: "idle".to_string(),
            animator_node_positions: HashMap::new(),
            animator_dragging_state: None,
            animator_drag_offset: [0.0, 0.0],
            animator_graph_zoom: 1.0,
            animator_detached: false,
            animator_link_drag_source: None,
            animator_state_rename_buffer: String::new(),
            build_job: None,
            build_dialog_pending: None,
        }
    }

    fn push_console_message(&mut self, message: impl Into<String>) {
        let msg = message.into();
        if msg.trim().is_empty() {
            return;
        }
        self.console_history.push(msg);
        if self.console_history.len() > 240 {
            let excess = self.console_history.len() - 240;
            self.console_history.drain(0..excess);
        }
    }

    fn sync_console_from_status(&mut self) {
        if self.status_msg != self.last_status_snapshot {
            self.last_status_snapshot = self.status_msg.clone();
            self.push_console_message(self.status_msg.clone());
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
        // Hub mais próximo da paleta principal da engine
        let bg = egui::Color32::from_rgb(10, 18, 30);
        let panel_bg = egui::Color32::from_rgb(18, 28, 44);
        let accent = egui::Color32::from_rgb(88, 166, 255);
        let text_dim = egui::Color32::from_rgb(165, 182, 210);
        let btn_bg = egui::Color32::from_rgb(32, 50, 78);
        let btn_hover = egui::Color32::from_rgb(40, 64, 98);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(bg))
            .show(ctx, |ui| {

            // ── Título centrado ──
            ui.add_space(28.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} {}", version::ENGINE_TITLE, version::ENGINE_VERSION))
                        .size(22.0)
                        .color(egui::Color32::WHITE)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("Hub de Projetos")
                        .size(13.0)
                        .color(text_dim),
                );
            });
            ui.add_space(24.0);

            // ── Layout 2 colunas ──
            let avail = ui.available_width();
            let col_w = (avail - 24.0) * 0.5;

            ui.horizontal(|ui| {
                ui.add_space(12.0);

                // ── Coluna esquerda: Último Projeto + Recentes ──
                ui.vertical(|ui| {
                    ui.set_width(col_w);

                    // Último Projeto
                    egui::Frame::none()
                        .fill(panel_bg)
                        .rounding(8.0)
                        .inner_margin(egui::Margin::same(14.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Último Projeto")
                                    .color(text_dim)
                                    .size(11.0),
                            );
                            ui.add_space(4.0);
                            match self.project_hub_session.last_project.clone() {
                                Some(path) => {
                                    ui.label(
                                        egui::RichText::new(&path)
                                            .color(egui::Color32::WHITE)
                                            .size(13.0),
                                    );
                                    ui.label(
                                        egui::RichText::new("Continuar último projeto")
                                            .color(text_dim)
                                            .size(11.0),
                                    );
                                    ui.add_space(10.0);
                                    let btn = egui::Button::new(
                                        egui::RichText::new("Continuar último projeto")
                                            .color(egui::Color32::WHITE),
                                    )
                                    .fill(egui::Color32::from_rgb(42, 74, 124))
                                    .min_size(egui::vec2(ui.available_width(), 32.0));
                                    if ui.add(btn).clicked() {
                                        let root = PathBuf::from(path);
                                        if let Err(e) = self.load_project_root(ctx, root) {
                                            self.status_msg = format!("❌ {}", e);
                                        }
                                    }
                                }
                                None => {
                                    ui.label(
                                        egui::RichText::new("Nenhum projeto recente salvo.")
                                            .color(text_dim),
                                    );
                                }
                            }
                        });

                    ui.add_space(12.0);

                    // Projetos Recentes
                    egui::Frame::none()
                        .fill(panel_bg)
                        .rounding(8.0)
                        .inner_margin(egui::Margin::same(14.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Projetos Recentes")
                                    .color(text_dim)
                                    .size(11.0),
                            );
                            ui.add_space(6.0);
                            if self.project_hub_session.recent_projects.is_empty() {
                                ui.label(
                                    egui::RichText::new("Nenhum projeto recente.")
                                        .color(text_dim),
                                );
                            } else {
                                let recent = self.project_hub_session.recent_projects.clone();
                                for path in recent {
                                    let resp = ui.add(
                                        egui::Button::new(
                                            egui::RichText::new(&path)
                                                .color(egui::Color32::WHITE)
                                                .size(12.0),
                                        )
                                        .fill(btn_bg)
                                        .min_size(egui::vec2(ui.available_width() - 60.0, 28.0)),
                                    );
                                    // "Abrir" ao lado
                                    // layout manual: usamos horizontal dentro
                                    let _ = resp; // handled below via horizontal
                                    ui.add_space(-28.0); // volta para fazer horizontal
                                    ui.horizontal(|ui| {
                                        let w = ui.available_width();
                                        let path_resp = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(&path)
                                                    .color(egui::Color32::WHITE)
                                                    .size(12.0),
                                            )
                                            .fill(btn_bg)
                                            .min_size(egui::vec2(w - 64.0, 28.0)),
                                        );
                                        let open_resp = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new("Abrir")
                                                    .color(accent),
                                            )
                                            .fill(btn_bg)
                                            .min_size(egui::vec2(56.0, 28.0)),
                                        );
                                        if path_resp.double_clicked() || open_resp.clicked() {
                                            if let Err(e) = self.load_project_root(ctx, PathBuf::from(&path)) {
                                                self.status_msg = format!("❌ {}", e);
                                            }
                                        }
                                    });
                                    ui.add_space(2.0);
                                }
                            }
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Selecione um projeto para começar.")
                                    .color(text_dim)
                                    .size(10.0),
                            );
                        });
                });

                ui.add_space(12.0);

                // ── Coluna direita: Ações ──
                ui.vertical(|ui| {
                    ui.set_width(col_w);
                    egui::Frame::none()
                        .fill(panel_bg)
                        .rounding(8.0)
                        .inner_margin(egui::Margin::same(14.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Ações")
                                    .color(text_dim)
                                    .size(11.0),
                            );
                            ui.add_space(8.0);

                            let actions: &[(&str, &str)] = &[
                                ("Novo Projeto...", "🆕"),
                                ("Abrir Projeto...", "📂"),
                                ("Entrar no editor atual", "✏"),
                                ("Sair", "🚪"),
                            ];

                            for (label, icon) in actions {
                                let full = format!("{}  {}", icon, label);
                                let btn = egui::Button::new(
                                    egui::RichText::new(&full)
                                        .color(egui::Color32::WHITE)
                                        .size(13.0),
                                )
                                .fill(btn_bg)
                                .min_size(egui::vec2(ui.available_width(), 38.0));

                                let _hover_id = egui::Id::new(format!("hub_btn_{}", label));
                                let resp = ui.add(btn);
                                if resp.hovered() {
                                    ui.painter().rect_filled(
                                        resp.rect,
                                        6.0,
                                        btn_hover,
                                    );
                                }

                                // Seta › à direita
                                ui.painter().text(
                                    egui::pos2(resp.rect.right() - 16.0, resp.rect.center().y),
                                    egui::Align2::CENTER_CENTER,
                                    "›",
                                    egui::FontId::proportional(16.0),
                                    text_dim,
                                );

                                if resp.clicked() {
                                    match *label {
                                        "Novo Projeto..." => {
                                            let base = std::env::current_dir()
                                                .unwrap_or_else(|_| PathBuf::from("."))
                                                .to_string_lossy()
                                                .to_string();
                                            self.new_project_dialog = Some(("MeuProjeto".to_string(), base));
                                        }
                                        "Abrir Projeto..." => {
                                            self.open_project_dialog = Some(String::new());
                                        }
                                        "Entrar no editor atual" => {
                                            self.enter_editor_for_current_project(ctx);
                                        }
                                        "Sair" => {
                                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                        }
                                        _ => {}
                                    }
                                }
                                ui.add_space(4.0);
                            }
                        });
                });

                ui.add_space(12.0);
            });

            // ── Status ──
            if !self.status_msg.is_empty() {
                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(&self.status_msg)
                            .color(egui::Color32::from_rgb(255, 180, 80)),
                    );
                });
            }
        });

        self.sync_console_from_status();

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
        let key = relative_path.trim().replace('\\', "/");

        if let Some(texture) = self.sprite_textures.get(&key) {
            return Some(texture.clone());
        }

        #[cfg(windows)]
        {
            for (k, tex) in self.sprite_textures.iter() {
                if k.eq_ignore_ascii_case(&key) {
                    return Some(tex.clone());
                }
            }
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

    pub fn start_build_standalone_pc(&mut self) -> Result<(), String> {
        if self.build_job.is_some() {
            return Err("Já existe uma build em andamento. Aguarde terminar a atual.".to_string());
        }
        if self.build_dialog_pending.is_some() {
            return Err("Diálogo de build já está aberto.".to_string());
        }

        // Feedback visual imediato — o save e o diálogo rfd podem demorar um instante.
        self.status_msg = "🔧 Salvando cena antes da build...".to_string();

        let saved_scene_path = self.save_active_scene()?;

        let engine_root = find_engine_root(&self.project_root)
            .ok_or_else(|| "Não foi possível localizar a raiz da engine (Cargo.toml).".to_string())?;

        let cargo_toml = engine_root.join("Cargo.toml");
        let bin_name = detect_bin_name(&cargo_toml).unwrap_or_else(|| "rs2br-engine".to_string());
        let engine_exe_name = if cfg!(target_os = "windows") {
            format!("{}.exe", bin_name)
        } else {
            bin_name.clone()
        };

        let project_name = self.current_project_name();
        let safe_project_name = sanitize_filename(&project_name);
        let default_output = if cfg!(target_os = "windows") {
            format!("{}.exe", safe_project_name)
        } else {
            safe_project_name.clone()
        };

        let saved_scene_name = saved_scene_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("main.scene.json")
            .to_string();

        let assets_src = self.project_root.join("assets");
        let save_src = self.project_root.join("save");
        let project_json_src = self.project_root.join("project.json");

        let cargo_available = detect_command_available("cargo");
        let prebuilt_runtime_exe = find_prebuilt_runtime_exe(&engine_root, &engine_exe_name);

        if prebuilt_runtime_exe.is_none() && !cargo_available {
            return Err(
                "Rust/Cargo não encontrado e não há runtime pré-compilado disponível. \
                 Para exportar sem Rust instalado, adicione um executável em standalone_runtime/windows/ na raiz da engine."
                    .to_string(),
            );
        }

        // Abre o diálogo de arquivo em thread separado para NÃO bloquear o editor.
        let (dialog_tx, dialog_rx) = mpsc::channel::<BuildDialogMessage>();
        let build_dir = self.project_root.join("build");
        let default_output_clone = default_output.clone();
        thread::spawn(move || {
            let result = rfd::FileDialog::new()
                .set_title("Escolher pasta e nome do executável do jogo")
                .set_directory(&build_dir)
                .set_file_name(&default_output_clone)
                .save_file();
            let msg = match result {
                Some(path) => BuildDialogMessage::Selected(path),
                None => BuildDialogMessage::Cancelled,
            };
            let _ = dialog_tx.send(msg);
        });

        self.build_dialog_pending = Some(BuildDialogPending {
            receiver: dialog_rx,
            engine_root,
            bin_name,
            engine_exe_name,
            project_name: project_name.clone(),
            safe_project_name,
            assets_src,
            save_src,
            project_json_src,
            saved_scene_name,
        });

        self.status_msg = format!("🔧 Aguardando escolha de destino para build '{}'...", project_name);
        Ok(())
    }

    /// Dispara a build real depois que o diálogo respondeu com o caminho.
    fn launch_build_after_dialog(&mut self, selected_output: PathBuf) {
        let pending = match self.build_dialog_pending.take() {
            Some(p) => p,
            None => return,
        };

        let safe_project_name = pending.safe_project_name.clone();
        let chosen_stem = selected_output
            .file_stem()
            .and_then(|s| s.to_str())
            .map(sanitize_filename)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| safe_project_name.clone());

        let selected_parent = match selected_output.parent() {
            Some(p) => p.to_path_buf(),
            None => {
                self.status_msg = "❌ Caminho de saída inválido.".to_string();
                return;
            }
        };

        let export_root = selected_parent.join(&chosen_stem);
        let build_cache_root = unique_temp_build_dir_in_parent(&selected_parent, &chosen_stem);
        let game_exe_name = if cfg!(target_os = "windows") {
            format!("{}.exe", chosen_stem)
        } else {
            chosen_stem.clone()
        };

        let engine_root = pending.engine_root;
        let bin_name = pending.bin_name;
        let engine_exe_name = pending.engine_exe_name;
        let project_name = pending.project_name;
        let assets_src = pending.assets_src;
        let save_src = pending.save_src;
        let project_json_src = pending.project_json_src;
        let saved_scene_name = pending.saved_scene_name;

        let prebuilt_runtime_exe = find_prebuilt_runtime_exe(&engine_root, &engine_exe_name);
        let use_prebuilt_runtime = prebuilt_runtime_exe.is_some();

        let (tx, rx) = mpsc::channel();
        self.build_job = Some(BuildJobState {
            receiver: rx,
            current_step: "Preparando build standalone...".to_string(),
            log_lines: vec![format!("🔧 Preparando build do projeto '{}'...", project_name)],
            started_at: Instant::now(),
            progress: 0.05,
            estimated_total: 90.0,
            min_visible_until: Instant::now() + Duration::from_millis(800),
            pending_result: None,
        });

        self.status_msg = format!("🔧 Build em andamento: {}", project_name);
        self.push_console_message(format!(
            "🔧 Build desktop iniciada para '{}' em {}",
            project_name,
            export_root.display()
        ));
        if use_prebuilt_runtime {
            self.push_console_message(
                "ℹ Runtime pré-compilado detectado. Build seguirá sem Rust/Cargo instalados.".to_string(),
            );
        }

        thread::spawn(move || {
            let send_step = |msg: &str| { let _ = tx.send(BuildWorkerMessage::Step(msg.to_string())); };
            let finish = |result: Result<PathBuf, String>| { let _ = tx.send(BuildWorkerMessage::Finished(result)); };

            let run = || -> Result<PathBuf, String> {
                if build_cache_root.exists() {
                    let _ = fs::remove_dir_all(&build_cache_root);
                }
                fs::create_dir_all(&build_cache_root)
                    .map_err(|e| format!("Falha ao preparar cache temporário da build: {}", e))?;

                send_step("Preparando pasta de saída...");
                let staged_export_root = build_cache_root.join("final_bundle");
                fs::create_dir_all(&staged_export_root)
                    .map_err(|e| format!("Falha ao criar pasta temporária de saída: {}", e))?;

                let exe_src = if let Some(prebuilt_path) = prebuilt_runtime_exe.as_ref() {
                    send_step("Usando runtime pré-compilado (sem Cargo)...");
                    let _ = tx.send(BuildWorkerMessage::Log(format!(
                        "▶ runtime pré-compilado: {}",
                        prebuilt_path.display()
                    )));
                    prebuilt_path.clone()
                } else {
                    let temp_engine_root = build_cache_root.join("engine_workspace");
                    send_step("Preparando workspace temporário...");
                    copy_minimal_engine_workspace(&engine_root, &temp_engine_root)
                        .map_err(|e| format!("Falha ao preparar workspace temporário: {}", e))?;

                    send_step("Compilando jogo standalone em release...");

                    let mut child = Command::new("cargo")
                        .arg("build")
                        .arg("--release")
                        .arg("--bin")
                        .arg(&bin_name)
                        .env("CARGO_TARGET_DIR", build_cache_root.join("target"))
                        .current_dir(&temp_engine_root)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                        .map_err(|e| {
                            format!(
                                "Falha ao executar cargo build --release: {}. \
                                 Dica: instale Rust/Cargo ou configure runtime pré-compilado em standalone_runtime/windows/.",
                                e
                            )
                        })?;

                    let _ = tx.send(BuildWorkerMessage::Log(format!(
                        "▶ comando: cargo build --release --bin {} ({})",
                        bin_name,
                        temp_engine_root.display()
                    )));

                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();

                    let stdout_handle = stdout.map(|pipe| {
                        let tx_out = tx.clone();
                        thread::spawn(move || {
                            let reader = BufReader::new(pipe);
                            for line in reader.lines().map_while(Result::ok) {
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    let _ = tx_out.send(BuildWorkerMessage::Log(trimmed.to_string()));
                                }
                            }
                        })
                    });

                    let stderr_handle = stderr.map(|pipe| {
                        let tx_err = tx.clone();
                        thread::spawn(move || {
                            let reader = BufReader::new(pipe);
                            for line in reader.lines().map_while(Result::ok) {
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    let _ = tx_err.send(BuildWorkerMessage::Log(trimmed.to_string()));
                                }
                            }
                        })
                    });

                    let status = child.wait().map_err(|e| format!("Falha ao aguardar término da build: {}", e))?;
                    if let Some(handle) = stdout_handle { let _ = handle.join(); }
                    if let Some(handle) = stderr_handle { let _ = handle.join(); }

                    if !status.success() {
                        return Err(format!(
                            "Build falhou (status: {}). Veja os detalhes no console. Cache em {}",
                            status,
                            build_cache_root.display()
                        ));
                    }

                    build_cache_root.join("target").join("release").join(&engine_exe_name)
                };

                send_step("Montando pasta final do jogo...");

                if !exe_src.exists() {
                    return Err(format!(
                        "Build concluída, mas o executável não foi encontrado em {}.",
                        exe_src.display()
                    ));
                }

                copy_runtime_binary_set(&exe_src, &staged_export_root, &game_exe_name)
                    .map_err(|e| format!("Falha ao copiar executável/dependências do jogo: {}", e))?;

                if project_json_src.exists() {
                    fs::copy(&project_json_src, staged_export_root.join("project.json"))
                        .map_err(|e| format!("Falha ao copiar project.json: {}", e))?;
                }

                fs::write(
                    staged_export_root.join(crate::standalone::STANDALONE_MARKER_FILE),
                    b"standalone=true\n",
                )
                .map_err(|e| format!("Falha ao criar marcador standalone: {}", e))?;

                if assets_src.exists() {
                    copy_dir_recursive(&assets_src, &staged_export_root.join("assets"))
                        .map_err(|e| format!("Falha ao copiar assets: {}", e))?;
                }

                if save_src.exists() {
                    copy_dir_recursive(&save_src, &staged_export_root.join("save"))
                        .map_err(|e| format!("Falha ao copiar save: {}", e))?;
                }

                let readme = format!(
                    "RS2BR Engine - Build Standalone PC\n\nProjeto: {}\nCena inicial: {}\nExecutável: {}\nModo: {}\n\nPara rodar:\n1. Deixe esta pasta inteira junta.\n2. Execute {}.\n\nRequisitos:\n- Não precisa de Rust ou Cargo.\n- Se o Windows reclamar de runtime C++, instale o Microsoft Visual C++ Redistributable 2015-2022 (x64).\n",
                    project_name,
                    saved_scene_name,
                    game_exe_name,
                    if use_prebuilt_runtime { "Runtime pré-compilado" } else { "Compilado via Cargo local" },
                    game_exe_name,
                );
                fs::write(staged_export_root.join("LEIA-ME.txt"), readme)
                    .map_err(|e| format!("Falha ao criar LEIA-ME.txt: {}", e))?;

                if export_root.exists() {
                    fs::remove_dir_all(&export_root)
                        .map_err(|e| format!("Falha ao limpar build anterior: {}", e))?;
                }
                fs::rename(&staged_export_root, &export_root)
                    .map_err(|e| format!("Falha ao finalizar pasta da build: {}", e))?;

                let _ = fs::remove_dir_all(&build_cache_root);
                Ok(export_root)
            };

            finish(run());
        });
    }

    /// Verifica se o diálogo de destino já respondeu; se sim, dispara a build.
    fn poll_build_dialog(&mut self, ctx: &egui::Context) {
        if self.build_dialog_pending.is_none() {
            return;
        }
        let msg = match self.build_dialog_pending.as_ref() {
            Some(p) => p.receiver.try_recv().ok(),
            None => return,
        };
        match msg {
            Some(BuildDialogMessage::Selected(path)) => {
                self.launch_build_after_dialog(path);
                ctx.request_repaint();
            }
            Some(BuildDialogMessage::Cancelled) => {
                self.build_dialog_pending = None;
                self.status_msg = "Build cancelada.".to_string();
            }
            None => {
                // Ainda aguardando — repaint rápido para checar logo
                ctx.request_repaint_after(Duration::from_millis(50));
            }
        }
    }

    fn poll_build_job(&mut self, ctx: &egui::Context) {
        let mut ready_result = None;
        let mut drained_messages: Vec<BuildWorkerMessage> = Vec::new();

        if let Some(job) = &mut self.build_job {
            while let Ok(msg) = job.receiver.try_recv() {
                drained_messages.push(msg);
            }
        }

        for msg in drained_messages {
            match msg {
                BuildWorkerMessage::Step(step) => {
                    if let Some(job) = &mut self.build_job {
                        job.current_step = step.clone();
                        let elapsed = job.started_at.elapsed().as_secs_f32();
                        let (progress, estimated_total) = estimate_build_progress(&step, elapsed, &job.log_lines);
                        job.progress = progress;
                        job.estimated_total = estimated_total;
                        job.log_lines.push(format!("➡ {}", step));
                        if job.log_lines.len() > 400 {
                            let excess = job.log_lines.len() - 400;
                            job.log_lines.drain(0..excess);
                        }
                    }
                    self.push_console_message(format!("🔧 {}", step));
                }
                BuildWorkerMessage::Log(line) => {
                    if let Some(job) = &mut self.build_job {
                        job.log_lines.push(line.clone());
                        let elapsed = job.started_at.elapsed().as_secs_f32();
                        let (progress, estimated_total) = estimate_build_progress(&job.current_step, elapsed, &job.log_lines);
                        job.progress = progress;
                        job.estimated_total = estimated_total;
                        if job.log_lines.len() > 400 {
                            let excess = job.log_lines.len() - 400;
                            job.log_lines.drain(0..excess);
                        }
                    }
                    self.push_console_message(line);
                }
                BuildWorkerMessage::Finished(result) => {
                    if let Some(job) = &mut self.build_job {
                        job.current_step = "Finalizando pacote...".to_string();
                        job.progress = 1.0;
                        job.pending_result = Some(result);
                    }
                }
            }
        }

        // Mantém a barra viva mesmo quando não entram novas linhas de log.
        // Evita sensação de "travou" durante compilação/linkedição.
        if let Some(job) = &mut self.build_job {
            let elapsed = job.started_at.elapsed().as_secs_f32();
            let (predicted_progress, estimated_total) =
                estimate_build_progress(&job.current_step, elapsed, &job.log_lines);
            job.estimated_total = estimated_total;
            let hard_cap = if job.pending_result.is_some() { 1.0 } else { 0.99 };
            job.progress = job.progress.max(predicted_progress).clamp(0.0, hard_cap);
            if job.pending_result.is_some() && Instant::now() >= job.min_visible_until {
                ready_result = job.pending_result.take();
            }
        }

        if let Some(result) = ready_result {
            self.build_job = None;
            match result {
                Ok(path) => {
                    self.status_msg = format!("✅ Build Standalone PC concluído: {}", path.display());
                    self.push_console_message(format!("✅ Build finalizada com sucesso em {}", path.display()));
                }
                Err(error) => {
                    self.status_msg = format!("❌ {}", error);
                    self.push_console_message(format!("❌ {}", error));
                }
            }
        }

        if self.build_job.is_some() {
            // Repaint frequente durante build para manter barra animada.
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn show_build_progress_window(&mut self, ctx: &egui::Context) {
        let Some(job) = &mut self.build_job else {
            return;
        };

        let is_done = job.pending_result.is_some();
        let elapsed = job.started_at.elapsed().as_secs_f32();
        let remaining = (job.estimated_total - elapsed).max(0.0);
        let progress = job.progress.clamp(0.0, 1.0);
        let step = job.current_step.clone();
        // Últimas linhas do log do compilador para o usuário ver que está compilando
        let log_tail: Vec<String> = job.log_lines
            .iter().rev().take(6).cloned().collect::<Vec<_>>().into_iter().rev().collect();

        egui::Window::new("Build Standalone PC")
            .collapsible(false)
            .resizable(true)
            .default_size(egui::vec2(520.0, 240.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                if is_done {
                    ui.label(egui::RichText::new("✅ Build concluída!")
                        .strong()
                        .color(egui::Color32::from_rgb(120, 220, 140)));
                } else {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new());
                        ui.label(egui::RichText::new("Construindo build do jogo...").strong());
                    });
                }
                ui.add_space(4.0);
                ui.label(format!("Etapa: {}", step));
                ui.add_space(4.0);
                ui.add(
                    egui::ProgressBar::new(progress)
                        .show_percentage()
                        .desired_width(ui.available_width()),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(format!("Tempo: {:.0}s", elapsed.ceil()));
                    if !is_done {
                        ui.separator();
                        ui.label(format!("Estimado restante: {:.0}s", remaining.ceil()));
                    }
                });
                ui.add_space(4.0);
                // Log compacto — mostra que a compilação está em andamento
                egui::ScrollArea::vertical()
                    .max_height(80.0)
                    .stick_to_bottom(true)
                    .auto_shrink([false, true])
                    .id_source("build_log_scroll")
                    .show(ui, |ui| {
                        for line in &log_tail {
                            ui.label(
                                egui::RichText::new(line)
                                    .small()
                                    .monospace()
                                    .color(egui::Color32::from_rgb(180, 200, 230)),
                            );
                        }
                    });
            });
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
        // ── Tema escuro profundo — v0.9 ──
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::from_rgb(220, 227, 236));
        visuals.panel_fill              = egui::Color32::from_rgb(22, 27, 34);
        visuals.faint_bg_color          = egui::Color32::from_rgb(30, 36, 46);
        visuals.extreme_bg_color        = egui::Color32::from_rgb(13, 17, 23);
        visuals.code_bg_color           = egui::Color32::from_rgb(20, 26, 35);
        visuals.window_fill             = egui::Color32::from_rgb(28, 33, 42);
        visuals.widgets.noninteractive.bg_fill      = egui::Color32::from_rgb(30, 36, 46);
        visuals.widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(35, 42, 54);
        visuals.widgets.inactive.bg_fill            = egui::Color32::from_rgb(40, 47, 60);
        visuals.widgets.inactive.weak_bg_fill       = egui::Color32::from_rgb(44, 52, 66);
        visuals.widgets.hovered.bg_fill             = egui::Color32::from_rgb(52, 62, 80);
        visuals.widgets.hovered.weak_bg_fill        = egui::Color32::from_rgb(56, 67, 85);
        visuals.widgets.active.bg_fill              = egui::Color32::from_rgb(56, 106, 188);
        visuals.widgets.active.weak_bg_fill         = egui::Color32::from_rgb(46, 92, 168);
        visuals.widgets.open.bg_fill                = egui::Color32::from_rgb(38, 46, 60);
        visuals.selection.bg_fill                   = egui::Color32::from_rgb(56, 106, 188);
        visuals.selection.stroke.color              = egui::Color32::from_rgb(88, 166, 255);
        visuals.hyperlink_color                     = egui::Color32::from_rgb(88, 166, 255);
        visuals.window_stroke.color                         = egui::Color32::from_rgb(48, 56, 70);
        visuals.widgets.noninteractive.bg_stroke.color      = egui::Color32::from_rgb(44, 52, 66);
        visuals.widgets.inactive.bg_stroke.color            = egui::Color32::from_rgb(50, 60, 75);
        visuals.widgets.hovered.bg_stroke.color             = egui::Color32::from_rgb(88, 166, 255);
        visuals.widgets.active.bg_stroke.color              = egui::Color32::from_rgb(120, 190, 255);
        visuals.widgets.open.bg_stroke.color                = egui::Color32::from_rgb(60, 72, 90);
        visuals.widgets.noninteractive.fg_stroke.color      = egui::Color32::from_rgb(48, 56, 70);
        visuals.window_rounding                             = 6.0.into();
        visuals.menu_rounding                               = 5.0.into();
        visuals.widgets.noninteractive.rounding             = 4.0.into();
        visuals.widgets.inactive.rounding                   = 4.0.into();
        visuals.widgets.hovered.rounding                    = 4.0.into();
        visuals.widgets.active.rounding                     = 4.0.into();
        visuals.widgets.open.rounding                       = 4.0.into();
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

        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S))
            && self.new_project_dialog.is_none()
            && self.open_project_dialog.is_none()
            && self.rename_asset_dialog.is_none()
            && self.rename_entity_dialog.is_none()
            && self.new_script_dialog.is_none()
            && self.new_lua_dialog.is_none()
            && self.new_folder_dialog.is_none()
            && self.new_entity_dialog.is_none()
            && self.new_template_dialog.is_none()
            && self.delete_confirmation.is_none()
        {
            match self.save_active_scene() {
                Ok(path) => {
                    self.status_msg = format!(
                        "✅ Cena salva: {}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("cena.scene.json")
                    );
                }
                Err(error) => self.status_msg = format!("❌ {}", error),
            }
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
            .default_width(self.layout.hierarchy_width.clamp(200.0, 380.0))
            .resizable(true)
            .min_width(200.0)
            .max_width(380.0)
            .show(ctx, |ui| {
                hierarchy::show(self, ui);
            });
        self.layout.hierarchy_width = hierarchy_response.response.rect.width().clamp(200.0, 380.0);

        let inspector_response = egui::SidePanel::right("inspector_panel")
            .default_width(self.layout.inspector_width.clamp(260.0, 380.0))
            .resizable(true)
            .min_width(260.0)
            .max_width(380.0)
            .show(ctx, |ui| {
                inspector::show(self, ui);
            });
        self.layout.inspector_width = inspector_response.response.rect.width().clamp(260.0, 380.0);

        let asset_response = egui::TopBottomPanel::bottom("asset_panel")
            .default_height(self.layout.asset_height.clamp(190.0, 420.0).max(220.0))
            .resizable(true)
            .min_height(170.0)
            .max_height(900.0)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let assets_selected = self.bottom_tab == BottomDockTab::Assets;
                    if ui.selectable_label(assets_selected, "📦 Asset Browser").clicked() {
                        self.bottom_tab = BottomDockTab::Assets;
                    }
                    let animator_selected = self.bottom_tab == BottomDockTab::Animator;
                    let animator_tab = ui.selectable_label(animator_selected, "🎞 Animator");
                    if animator_tab.clicked() {
                        self.bottom_tab = BottomDockTab::Animator;
                    }
                    if animator_tab.double_clicked() {
                        self.bottom_tab = BottomDockTab::Animator;
                        self.animator_detached = true;
                    }
                    let console_selected = self.bottom_tab == BottomDockTab::Console;
                    if ui.selectable_label(console_selected, "🖥 Console").clicked() {
                        self.bottom_tab = BottomDockTab::Console;
                    }
                });
                ui.separator();

                match self.bottom_tab {
                    BottomDockTab::Assets => asset_browser::show(self, ui),
                    BottomDockTab::Animator => animator_editor::show(self, ui),
                    BottomDockTab::Console => {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Mensagens do editor e runtime").small().weak());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Limpar console").clicked() {
                                    self.console_history.clear();
                                    self.console_history.push("ℹ Console limpo.".to_string());
                                    self.status_msg = "ℹ Console limpo.".to_string();
                                }
                            });
                        });
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for line in &self.console_history {
                                    let color = if line.contains('❌') || line.to_lowercase().contains("erro") {
                                        egui::Color32::from_rgb(255, 120, 120)
                                    } else if line.contains('✅') || line.contains('✔') || line.to_lowercase().contains("salvo") || line.to_lowercase().contains("criado") {
                                        egui::Color32::from_rgb(120, 220, 140)
                                    } else if line.contains('⚠') || line.to_lowercase().contains("aviso") {
                                        egui::Color32::from_rgb(255, 210, 90)
                                    } else {
                                        egui::Color32::from_rgb(210, 220, 235)
                                    };
                                    ui.colored_label(color, line);
                                }
                                for warning in self.collect_runtime_warnings() {
                                    let prefix = match warning.severity {
                                        warnings::EditorWarningSeverity::Info => "ℹ",
                                        warnings::EditorWarningSeverity::Warning => "⚠",
                                        warnings::EditorWarningSeverity::Error => "❌",
                                    };
                                    ui.colored_label(egui::Color32::from_rgb(220, 220, 120), format!("{} {} — {}", prefix, warning.label, warning.details));
                                }
                            });
                    }
                }
            });
        self.layout.asset_height = asset_response.response.rect.height().clamp(190.0, 420.0);



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

        // Processa build cedo no frame para a janela de progresso aparecer
        // imediatamente após clicar em "Build Standalone PC".
        self.poll_build_dialog(ctx);
        self.poll_build_job(ctx);
        self.show_build_progress_window(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            scene_view::show(self, ui);
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
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
                    ui.label(format!("Seleção: {}", self.selected_entity_ids.len()));
                }
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




fn unique_temp_build_dir_in_parent(parent: &Path, project_name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    parent.join(format!(".rs2br_build_cache_{}_{}", sanitize_filename(project_name), timestamp))
}

fn copy_minimal_engine_workspace(src_root: &Path, dst_root: &Path) -> std::io::Result<()> {
    fn copy_file_if_exists(src_root: &Path, dst_root: &Path, rel: &str) -> std::io::Result<()> {
        let src = src_root.join(rel);
        if src.is_file() {
            let dst = dst_root.join(rel);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(src, dst)?;
        }
        Ok(())
    }

    fn copy_dir_recursive_filtered(src: &Path, dst: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == "target" || name == ".git" || name == "build" || name == ".cargo-lock" {
                continue;
            }
            let target = dst.join(entry.file_name());
            let ty = entry.file_type()?;
            if ty.is_dir() {
                copy_dir_recursive_filtered(&path, &target)?;
            } else if ty.is_file() {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&path, &target)?;
            }
        }
        Ok(())
    }

    if dst_root.exists() {
        fs::remove_dir_all(dst_root)?;
    }
    fs::create_dir_all(dst_root)?;

    copy_file_if_exists(src_root, dst_root, "Cargo.toml")?;
    copy_file_if_exists(src_root, dst_root, "Cargo.lock")?;
    copy_file_if_exists(src_root, dst_root, "build.rs")?;
    copy_file_if_exists(src_root, dst_root, "assets/icon/rs2br_engine_icon.ico")?;
    copy_file_if_exists(src_root, dst_root, "assets/icon/rs2br_engine_icon.png")?;

    let cargo_dir = src_root.join(".cargo");
    if cargo_dir.is_dir() {
        copy_dir_recursive_filtered(&cargo_dir, &dst_root.join(".cargo"))?;
    }

    let src_dir = src_root.join("src");
    if !src_dir.is_dir() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Pasta src da engine não encontrada"));
    }
    copy_dir_recursive_filtered(&src_dir, &dst_root.join("src"))?;

    Ok(())
}

fn estimate_build_progress(current_step: &str, elapsed: f32, log_lines: &[String]) -> (f32, f32) {
    let logs = log_lines.join("\n").to_lowercase();
    let step = current_step.to_lowercase();

    let estimated_total = if logs.contains("downloading crates") || logs.contains("updating crates.io index") {
        180.0
    } else if logs.contains("compiling") {
        120.0
    } else {
        75.0
    };

    let mut progress = if step.contains("preparando pasta") {
        0.10
    } else if step.contains("runtime pré-compilado") {
        0.55
    } else if step.contains("workspace temporário") {
        0.20
    } else if step.contains("compilando") {
        0.35 + (elapsed / estimated_total * 0.45)
    } else if step.contains("montando pasta final") {
        0.88
    } else {
        0.05
    };

    if logs.contains("finished `release`") || logs.contains("finished release") {
        progress = 0.92;
    }
    if logs.contains("build concluída") || logs.contains("build finalizada") {
        progress = 1.0;
    }

    (progress.clamp(0.0, 0.99), estimated_total)
}

fn find_engine_root(project_root: &Path) -> Option<PathBuf> {
    for base in [project_root.to_path_buf(), std::env::current_dir().ok()?] {
        for ancestor in base.ancestors() {
            let candidate = ancestor.join("Cargo.toml");
            if candidate.exists() {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

fn detect_bin_name(cargo_toml_path: &Path) -> Option<String> {
    let content = fs::read_to_string(cargo_toml_path).ok()?;
    let mut in_bin = false;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line == "[[bin]]" {
            in_bin = true;
            continue;
        }
        if line.starts_with('[') && line != "[[bin]]" {
            if in_bin {
                break;
            }
        }
        if in_bin && line.starts_with("name") {
            let value = line.split('=').nth(1)?.trim().trim_matches('"').to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.starts_with("name") {
            let value = line.split('=').nth(1)?.trim().trim_matches('"').to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ty.is_file() {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn detect_command_available(command: &str) -> bool {
    let result = Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    matches!(result, Ok(status) if status.success())
}

fn find_prebuilt_runtime_exe(engine_root: &Path, engine_exe_name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(engine_root.join("standalone_runtime").join("windows").join(engine_exe_name));
    candidates.push(engine_root.join("standalone_runtime").join(engine_exe_name));
    candidates.push(engine_root.join("build").join("standalone_runtime").join("windows").join(engine_exe_name));
    candidates.push(engine_root.join("build").join("standalone_runtime").join(engine_exe_name));
    candidates.push(engine_root.join("target").join("release").join(engine_exe_name));
    if let Ok(custom_dir) = std::env::var("RS2BR_STANDALONE_RUNTIME_DIR") {
        let trimmed = custom_dir.trim();
        if !trimmed.is_empty() {
            candidates.push(PathBuf::from(trimmed).join(engine_exe_name));
        }
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn copy_runtime_binary_set(exe_src: &Path, dst_root: &Path, game_exe_name: &str) -> std::io::Result<()> {
    fs::create_dir_all(dst_root)?;
    fs::copy(exe_src, dst_root.join(game_exe_name))?;
    let Some(runtime_dir) = exe_src.parent() else {
        return Ok(());
    };
    for entry in fs::read_dir(runtime_dir)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if !ty.is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if ext == "dll" {
            fs::copy(&path, dst_root.join(entry.file_name()))?;
        }
    }
    Ok(())
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
        self.status_msg = msg.clone();
        self.sync_console_from_status();
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