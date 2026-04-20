use std::{collections::HashMap, env, path::{Path, PathBuf}};

use eframe::egui;

use crate::{
    core::{project::ProjectConfig, scene::Scene},
    runtime::{RuntimeState, camera, renderer::{self, UiAction}, context::{RuntimeContext, RuntimePlayState}},
};

pub const STANDALONE_MARKER_FILE: &str = ".rs2br_standalone";

pub fn detect_standalone_project_root() -> Option<PathBuf> {
    let exe_dir = env::current_exe().ok()?.parent()?.to_path_buf();
    if exe_dir.join(STANDALONE_MARKER_FILE).exists()
        && exe_dir.join("project.json").exists()
        && exe_dir.join("assets").exists()
    {
        Some(exe_dir)
    } else {
        None
    }
}

pub struct StandaloneApp {
    project_root: PathBuf,
    scene: Scene,
    scene_path: Option<PathBuf>,
    runtime: RuntimeState,
    play_state: RuntimePlayState,
    status_msg: String,
    sprite_textures: HashMap<String, egui::TextureHandle>,
}

impl StandaloneApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, project_root: PathBuf) -> Self {
        let project_config = ProjectConfig::load_or_create(&project_root);
        let scene_path = project_root.join(&project_config.initial_scene);
        let scene = crate::serialization::scene_serializer::try_load_scene_from_path(&scene_path)
            .unwrap_or_else(|_| Scene::new(&project_config.name));

        let mut runtime = RuntimeState::new();
        runtime.window_open = true;
        runtime.start_from_scene_as_new_game(&scene);

        Self {
            project_root,
            scene,
            scene_path: Some(scene_path),
            runtime,
            play_state: RuntimePlayState::Playing,
            status_msg: String::new(),
            sprite_textures: HashMap::new(),
        }
    }
}

impl eframe::App for StandaloneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            // Para apenas o áudio (não-bloqueante) antes de fechar.
            // Não chamamos runtime.stop() completo aqui pois limpar Lua VMs
            // pode travar o thread principal. O OS vai destruir o processo logo.
            self.runtime.audio_runtime.stop_all();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.play_state = match self.play_state {
                RuntimePlayState::Playing => RuntimePlayState::Paused,
                RuntimePlayState::Paused => RuntimePlayState::Playing,
                RuntimePlayState::Edit => RuntimePlayState::Playing,
            };
        }

        egui::CentralPanel::default().frame(
            egui::Frame::default().fill(egui::Color32::BLACK)
        ).show(ctx, |ui| {
            show_game(self, ui);
        });
    }
}

impl RuntimeContext for StandaloneApp {
    fn play_state(&self) -> RuntimePlayState { self.play_state }
    fn set_play_state(&mut self, state: RuntimePlayState) { self.play_state = state; }
    fn project_root(&self) -> &Path { &self.project_root }
    fn active_scene_snapshot(&self) -> &Scene { &self.scene }
    fn scene_file_candidates(&self) -> Vec<PathBuf> {
        let scenes_dir = self.project_root.join("assets").join("scenes");
        let mut result = Vec::new();
        if let Ok(rd) = std::fs::read_dir(scenes_dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("json") {
                    result.push(p);
                }
            }
        }
        result
    }
    fn scene_snapshot_by_path(&self, path: &Path) -> Option<(Scene, Option<PathBuf>)> {
        crate::serialization::scene_serializer::try_load_scene_from_path(path)
            .ok()
            .map(|scene| (scene, Some(path.to_path_buf())))
    }
    fn set_status(&mut self, msg: String) { self.status_msg = msg; }
    fn sprite_textures(&mut self) -> &mut HashMap<String, egui::TextureHandle> { &mut self.sprite_textures }
}

fn show_game(host: &mut StandaloneApp, ui: &mut egui::Ui) {
    let play_state = host.play_state();
    let scene_snap = host.active_scene_snapshot().clone();
    let project_root = host.project_root().to_path_buf();

    host.runtime.sync_with_mode(&play_state, &scene_snap, &project_root, -260.0);
    apply_egui_inputs(&mut host.runtime, ui.ctx());

    if host.runtime.active_scene.is_none() {
        ui.centered_and_justified(|ui| ui.label("Nenhuma cena ativa."));
        return;
    }

    if play_state == RuntimePlayState::Playing {
        ui.ctx().request_repaint();
    }

    let available = ui.available_rect_before_wrap();
    let game_response = ui.allocate_rect(available, egui::Sense::click());
    if game_response.clicked() || game_response.hovered() {
        ui.ctx().memory_mut(|mem| mem.request_focus(game_response.id));
    }
    let painter = ui.painter_at(available);

    let mut pending_ui_action = None;

    // Snapshot só das entidades para desenho: `sprite_textures` exige `&mut host`, incompatível
    // com emprestar `host.runtime` ao mesmo tempo. Mais barato que clonar a `Scene` inteira.
    let (bg, entities_snapshot) = {
        let scene = host.runtime.active_scene.as_ref().expect("checked above");
        (scene.background_color, scene.entities.clone())
    };

    painter.rect_filled(
        available,
        0.0,
        egui::Color32::from_rgb(
            (bg[0] * 255.0) as u8,
            (bg[1] * 255.0) as u8,
            (bg[2] * 255.0) as u8,
        ),
    );

    let center = available.center();
    let mut camera_state = camera::find_main_camera(&entities_snapshot);
    if let Some(zoom) = host.runtime.camera_zoom_override {
        camera_state.zoom = zoom.clamp(0.2, 4.0);
    }
    if host.runtime.camera_shake_time > f32::EPSILON && host.runtime.camera_shake_intensity > f32::EPSILON {
        let phase = host.runtime.elapsed_time * 40.0;
        let shake = host.runtime.camera_shake_intensity * host.runtime.camera_shake_time.clamp(0.0, 1.0);
        camera_state.x += phase.sin() * shake;
        camera_state.y += (phase * 1.37).cos() * shake;
    }

    let current_scene_path = host.runtime.scene_manager.current_path.clone();

    renderer::draw_runtime_particles(&painter, center, camera_state, &host.runtime.particles);

    for entity in &entities_snapshot {
        if let Some(action) = renderer::draw_runtime_entity(
            ui,
            &painter,
            &project_root,
            current_scene_path.as_deref(),
            host.sprite_textures(),
            entity,
            center,
            camera_state,
        ) {
            pending_ui_action = Some(action);
        }
    }

    if let Some(action) = pending_ui_action {
        match action {
            UiAction::ChangeScene(path) => {
                if let Some((scene, file_path)) = host.scene_snapshot_by_path(&path) {
                    host.scene = scene.clone();
                    host.scene_path = file_path.clone();
                    let label = scene.name.clone();
                    host.runtime.queue_scene_change_snapshot(scene, file_path, Some(label.clone()));
                    host.set_status(format!("Cena alterada: {}", label));
                } else {
                    host.runtime.queue_scene_change(path);
                }
            }
            UiAction::CloseRuntime => {
                host.set_play_state(RuntimePlayState::Edit);
                host.runtime.stop();
                host.runtime.window_open = false;
            }
        }
    }

    if matches!(host.runtime.game_state.flow, crate::runtime::state::RuntimeGameFlow::Loading) {
        painter.text(
            available.center(),
            egui::Align2::CENTER_CENTER,
            format!("Loading... {}", host.runtime.game_state.loading_label.clone().unwrap_or_default()),
            egui::FontId::proportional(22.0),
            egui::Color32::WHITE,
        );
    }

    if !host.status_msg.is_empty() {
        painter.text(
            egui::pos2(available.left() + 12.0, available.bottom() - 12.0),
            egui::Align2::LEFT_BOTTOM,
            &host.status_msg,
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(210, 230, 255),
        );
    }

    if host.play_state == RuntimePlayState::Paused {
        painter.text(
            available.center_top() + egui::vec2(0.0, 16.0),
            egui::Align2::CENTER_TOP,
            "Pausado — pressione ESC para continuar",
            egui::FontId::proportional(16.0),
            egui::Color32::WHITE,
        );
    }

    painter.text(
        egui::pos2(available.right() - 12.0, available.top() + 12.0),
        egui::Align2::RIGHT_TOP,
        format!("FPS: {:.0}", host.runtime.estimated_fps()),
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(220, 255, 220),
    );
}

fn apply_egui_inputs(runtime: &mut RuntimeState, ctx: &egui::Context) {
    use crate::runtime::systems::input_system;
    let new_input = input_system::capture_runtime_input(ctx, &runtime.input);
    runtime.input = new_input;


    let cam_axis = input_system::camera_axis(&runtime.input);
    let zoom_d = input_system::zoom_delta(ctx);
    if let Some(scene) = &mut runtime.active_scene {
        if cam_axis != egui::Vec2::ZERO || zoom_d.abs() > f32::EPSILON {
            crate::runtime::camera::move_main_camera(&mut scene.entities, cam_axis.x * 4.0, cam_axis.y * 4.0, zoom_d * 0.05);
        }
    }
}
