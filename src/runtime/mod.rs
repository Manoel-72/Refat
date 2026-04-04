// ============================================================
//  runtime/mod.rs
//  Runtime V0.2 da RS2BR-Engine.
//  Objetivo: Play/Pause/Stop estáveis, preview em tempo real,
//  recarga/troca de cena e separação básica de responsabilidades.
// ============================================================

pub mod camera;
pub mod input;
pub mod renderer;
pub mod scene_manager;
pub mod script;
pub mod state;
pub mod systems;

use std::{collections::HashMap, path::{Path, PathBuf}};

use eframe::egui;

use crate::{
    core::{scene::Scene, version},
    editor::{EditorPlayState, OpenSceneDocument},
};
use self::renderer::UiAction;

pub use state::{RuntimeInput, RuntimeState};

const GROUND_Y: f32 = -260.0;

pub struct RuntimeContext<'a> {
    pub play_state: &'a mut EditorPlayState,
    pub runtime: &'a mut RuntimeState,
    pub status_msg: &'a mut String,
    pub project_root: &'a Path,
    pub sprite_textures: &'a mut HashMap<String, egui::TextureHandle>,
    pub scene_file_candidates: Vec<PathBuf>,
    pub active_scene: &'a Scene,
    pub open_scenes: &'a Vec<OpenSceneDocument>,
}

#[derive(Debug, Clone)]
pub enum EditorAction {
    SyncActiveSceneDocument,
    QueueSceneChangeFromMemory {
        scene: Scene,
        source_path: Option<PathBuf>,
        label: String,
    },
    QueueSceneChangeFromPath(PathBuf),
    CloseRuntime,
    SetPlayState(EditorPlayState),
    SetStatusMsg(String),
}

/// Exibe a janela nativa do runtime quando a engine está em Play/Pause.
pub fn show_viewport(rtx: &mut RuntimeContext, ctx: &egui::Context) -> Vec<EditorAction> {
    let current_play_state = *rtx.play_state;
    if current_play_state == EditorPlayState::Edit || !rtx.runtime.window_open {
        return Vec::new();
    }

    let mut actions = Vec::new();
    let viewport_id = egui::ViewportId::from_hash_of("rs2br_runtime_viewport");
    ctx.show_viewport_immediate(
        viewport_id,
        egui::ViewportBuilder::default()
            .with_title(&format!("▶ {} {} Runtime", version::ENGINE_TITLE, version::ENGINE_VERSION))
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([480.0, 320.0]),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                actions.push(EditorAction::CloseRuntime);
                actions.push(EditorAction::SetPlayState(EditorPlayState::Edit));
                actions.push(EditorAction::SetStatusMsg("Runtime fechado".to_string()));
                return;
            }

            if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                let next_play_state = match *rtx.play_state {
                    EditorPlayState::Playing => EditorPlayState::Paused,
                    EditorPlayState::Paused => EditorPlayState::Playing,
                    EditorPlayState::Edit => EditorPlayState::Edit,
                };
                actions.push(EditorAction::SetPlayState(next_play_state));
                let next_status = match next_play_state {
                    EditorPlayState::Paused => "⏸ Runtime pausado por ESC".to_string(),
                    EditorPlayState::Playing => "▶ Runtime retomado por ESC".to_string(),
                    EditorPlayState::Edit => rtx.status_msg.clone(),
                };
                actions.push(EditorAction::SetStatusMsg(next_status));
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("▶ Play").clicked() {
                        let source_path = find_active_scene_source_path(rtx.open_scenes, rtx.active_scene);
                        rtx.runtime.start_from_document(rtx.active_scene, source_path.clone());
                        rtx.runtime.window_open = true;
                        actions.push(EditorAction::SyncActiveSceneDocument);
                        actions.push(EditorAction::SetPlayState(EditorPlayState::Playing));
                        actions.push(EditorAction::SetStatusMsg(format!(
                            "▶ Runtime iniciado com a cena em memória '{}'{}",
                            rtx.active_scene.name,
                            if source_path.is_some() { " (preserva alterações não salvas)" } else { "" }
                        )));
                    }

                    if ui.button("⏸ Pause").clicked() {
                        actions.push(EditorAction::SetPlayState(EditorPlayState::Paused));
                        actions.push(EditorAction::SetStatusMsg("⏸ Runtime pausado".to_string()));
                    }

                    if ui.button("⏹ Parar").clicked() {
                        actions.push(EditorAction::CloseRuntime);
                        actions.push(EditorAction::SetPlayState(EditorPlayState::Edit));
                        actions.push(EditorAction::SetStatusMsg("⏹ Runtime parado".to_string()));
                    }

                    if ui.button("↻ Recarregar Cena").clicked() {
                        rtx.runtime.reload_current_scene(rtx.active_scene);
                        actions.push(EditorAction::SetStatusMsg("↻ Cena recarregada no runtime".to_string()));
                    }

                    ui.separator();

                    if ui.button("✖ Fechar runtime").clicked() {
                        actions.push(EditorAction::CloseRuntime);
                        actions.push(EditorAction::SetPlayState(EditorPlayState::Edit));
                        actions.push(EditorAction::SetStatusMsg("Runtime fechado".to_string()));
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label("Trocar cena no runtime:");
                    for path in &rtx.scene_file_candidates {
                        let label = path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .map(|name| name.replace(".scene", ""))
                            .unwrap_or_else(|| "Cena".to_string());

                        if ui.small_button(format!("🎬 {}", label)).clicked() {
                            actions.push(EditorAction::SyncActiveSceneDocument);
                            actions.push(EditorAction::QueueSceneChangeFromPath(path.clone()));
                            actions.push(EditorAction::SetStatusMsg(format!("Cena agendada para runtime: {}", label)));
                        }
                    }
                });

                actions.extend(show(rtx, ui));
            });
        },
    );

    actions
}

/// Exibe o preview do runtime dentro da viewport principal.
pub fn show(rtx: &mut RuntimeContext, ui: &mut egui::Ui) -> Vec<EditorAction> {
    let mut actions = Vec::new();
    let play_state = *rtx.play_state;
    rtx.runtime
        .sync_with_mode(play_state, rtx.active_scene, rtx.project_root, GROUND_Y);

    apply_runtime_inputs(rtx, ui.ctx());

    let Some(runtime_scene) = rtx.runtime.active_scene.clone() else {
        ui.centered_and_justified(|ui| {
            ui.label("Runtime inativo.");
        });
        return actions;
    };

    if *rtx.play_state != EditorPlayState::Edit {
        ui.ctx().request_repaint();
    }

    ui.horizontal_wrapped(|ui| {
        ui.heading("▶ Runtime Preview");
        ui.separator();
        ui.label(format!("📌 {}", runtime_scene.name));
        ui.separator();
        ui.label(format!("⏱ {:.2}s", rtx.runtime.elapsed_time));
        ui.separator();
        ui.label(format!("FPS ~ {:.0}", rtx.runtime.estimated_fps()));
        ui.separator();
        ui.label(match *rtx.play_state {
            EditorPlayState::Playing => "Status: Executando",
            EditorPlayState::Paused => "Status: Pausado",
            EditorPlayState::Edit => "Status: Edição",
        });
        ui.separator();
        ui.label(format!("Flow: {:?}", rtx.runtime.game_state.flow));
        ui.separator();
        ui.label(format!("Etapa: {:?}", rtx.runtime.last_stage));
    });

    ui.label("Runtime jogável: carrega a cena atual, renderiza sprites, atualiza física e executa scripts anexados.");
    ui.label("WASD = mover entidade com @player_controller | Setas = mover câmera | Q/E ou scroll = zoom | ESC = pause");
    ui.separator();

    ui.horizontal_wrapped(|ui| {
        if ui.button("Spawn Enemy").clicked() {
            let spawn_x = 120.0 + (rtx.runtime.frame_count % 5) as f32 * 36.0;
            rtx.runtime.queue_spawn("enemy", spawn_x, 48.0);
            rtx.runtime.game_state.score += 1;
            actions.push(EditorAction::SetStatusMsg("Spawn agendado para o fim do frame".to_string()));
        }

        if ui.button("Destroy Last Spawn").clicked() {
            if rtx.runtime.queue_destroy_last_spawned() {
                actions.push(EditorAction::SetStatusMsg("Destroy agendado para o fim do frame".to_string()));
            } else {
                actions.push(EditorAction::SetStatusMsg("Nenhuma entidade spawnada para destruir".to_string()));
            }
        }

        if ui.button("Pause/Resume").clicked() {
            let next_play_state = match *rtx.play_state {
                EditorPlayState::Playing => EditorPlayState::Paused,
                EditorPlayState::Paused => EditorPlayState::Playing,
                EditorPlayState::Edit => EditorPlayState::Edit,
            };
            actions.push(EditorAction::SetPlayState(next_play_state));
        }
    });
    ui.separator();

    let available = ui.available_rect_before_wrap();
    let _response = ui.allocate_rect(available, egui::Sense::hover());
    let painter = ui.painter_at(available);

    let bg = runtime_scene.background_color;
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
    let camera = camera::find_main_camera(&runtime_scene.entities);

    renderer::draw_runtime_grid(&painter, available, center, camera);

    let ground_screen_y = center.y - ((GROUND_Y - camera.y) * camera.zoom);
    painter.line_segment(
        [
            egui::pos2(available.left(), ground_screen_y),
            egui::pos2(available.right(), ground_screen_y),
        ],
        egui::Stroke::new(2.0, egui::Color32::from_rgb(150, 110, 70)),
    );
    painter.text(
        egui::pos2(available.left() + 10.0, ground_screen_y - 6.0),
        egui::Align2::LEFT_BOTTOM,
        "Chão",
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(200, 180, 140),
    );

    let current_scene_path = rtx.runtime.scene_manager.current_path.clone();
    let mut pending_ui_action = None;

    for entity in &runtime_scene.entities {
        if let Some(action) = renderer::draw_runtime_entity(
            ui,
            &painter,
            rtx.project_root,
            current_scene_path.as_deref(),
            rtx.sprite_textures,
            entity,
            center,
            camera,
        ) {
            pending_ui_action = Some(action);
        }
    }

    if let Some(action) = pending_ui_action {
        match action {
            UiAction::ChangeScene(path) => {
                let label = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("cena")
                    .to_string();
                actions.push(EditorAction::SyncActiveSceneDocument);
                actions.push(EditorAction::QueueSceneChangeFromPath(path));
                actions.push(EditorAction::SetStatusMsg(format!("UIButton acionado: troca de cena -> {}", label)));
            }
            UiAction::CloseRuntime => {
                actions.push(EditorAction::CloseRuntime);
                actions.push(EditorAction::SetPlayState(EditorPlayState::Edit));
                actions.push(EditorAction::SetStatusMsg("UIButton acionado: fechar runtime".to_string()));
            }
        }
    }

    if matches!(rtx.runtime.game_state.flow, crate::runtime::state::RuntimeGameFlow::Loading) {
        painter.text(
            available.center(),
            egui::Align2::CENTER_CENTER,
            format!(
                "Loading... {}",
                rtx.runtime.game_state.loading_label.clone().unwrap_or_default()
            ),
            egui::FontId::proportional(22.0),
            egui::Color32::WHITE,
        );
    }

    painter.text(
        egui::pos2(available.left() + 8.0, available.top() + 8.0),
        egui::Align2::LEFT_TOP,
        format!(
            "Entidades: {}  •  Scripts: {}  •  Delta: {:.3} ms",
            renderer::count_entities(&runtime_scene.entities),
            renderer::count_scripts(&runtime_scene.entities),
            rtx.runtime.delta_time * 1000.0,
        ),
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(220, 220, 220),
    );

    painter.text(
        egui::pos2(available.right() - 8.0, available.top() + 8.0),
        egui::Align2::RIGHT_TOP,
        format!(
            "HUD/UI  •  Cena: {}  •  Score: {}  •  Flow: {:?}",
            runtime_scene.name,
            rtx.runtime.game_state.score,
            rtx.runtime.game_state.flow,
        ),
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(180, 235, 180),
    );

    actions
}

fn apply_runtime_inputs(rtx: &mut RuntimeContext, ctx: &egui::Context) {
    let step = if *rtx.play_state == EditorPlayState::Paused {
        1.0 / 60.0
    } else {
        rtx.runtime.delta_time.max(1.0 / 120.0)
    };

    let previous_input = rtx.runtime.input.clone();
    rtx.runtime.input = systems::input_system::capture_runtime_input(ctx, &previous_input);

    let camera_input = systems::input_system::camera_axis(&rtx.runtime.input);
    let zoom_input = systems::input_system::zoom_delta(ctx);
    let reload_scene = ctx.input(|i| i.key_pressed(egui::Key::R));

    if reload_scene {
        rtx.runtime.reload_current_scene(rtx.active_scene);
        return;
    }

    let Some(scene) = rtx.runtime.active_scene.as_mut() else {
        return;
    };

    if camera_input != egui::Vec2::ZERO || zoom_input.abs() > f32::EPSILON {
        let camera_speed = 260.0 * step;
        camera::move_main_camera(
            &mut scene.entities,
            camera_input.x * camera_speed,
            camera_input.y * camera_speed,
            zoom_input * step.max(0.02),
        );
    }

    rtx.runtime.scene_manager.current_scene = Some(scene.clone());
}

fn find_active_scene_source_path(open_scenes: &Vec<OpenSceneDocument>, active_scene: &Scene) -> Option<PathBuf> {
    open_scenes
        .iter()
        .find(|doc| doc.scene.name == active_scene.name)
        .and_then(|doc| doc.file_path.clone())
}
