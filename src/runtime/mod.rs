// ============================================================
//  runtime/mod.rs  —  V0.9
//  Runtime desacoplado do editor via RuntimeContext trait.
// ============================================================

pub mod camera;
pub mod context;
pub mod input;
pub mod lua_runtime;
pub mod renderer;
pub mod save;
pub mod scene_manager;
pub mod script;
pub mod state;
pub mod systems;

use eframe::egui;
use std::time::Duration;

/// Intervalo mínimo entre frames: ~120 FPS (8.33 ms).
const FRAME_BUDGET: Duration = Duration::from_micros(8_333);

use self::{
    context::{RuntimeContext, RuntimePlayState},
    renderer::UiAction,
};
use crate::core::version;

pub use state::{RuntimeInput, RuntimeState};

const GROUND_Y: f32 = -260.0;

// ── show_viewport ────────────────────────────────────────────

/// Exibe a janela nativa do runtime (Play/Pause).
/// Aceita qualquer host que implemente RuntimeContext.
pub fn show_viewport<H: RuntimeContext>(
    host: &mut H,
    runtime: &mut RuntimeState,
    ctx: &egui::Context,
) {
    if host.play_state() == RuntimePlayState::Edit || !runtime.window_open {
        return;
    }

    let viewport_id = egui::ViewportId::from_hash_of("rs2br_runtime_viewport");
    ctx.show_viewport_immediate(
        viewport_id,
        egui::ViewportBuilder::default()
            .with_title(&format!(
                "▶ {} {} Runtime",
                version::ENGINE_TITLE,
                version::ENGINE_VERSION
            ))
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([480.0, 320.0]),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                host.set_play_state(RuntimePlayState::Edit);
                runtime.stop();
                runtime.window_open = false;
                host.set_status("Runtime fechado".to_string());
                return;
            }

            if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                let next = match host.play_state() {
                    RuntimePlayState::Playing => RuntimePlayState::Paused,
                    RuntimePlayState::Paused => RuntimePlayState::Playing,
                    RuntimePlayState::Edit => RuntimePlayState::Edit,
                };
                host.set_play_state(next);
                host.set_status(match host.play_state() {
                    RuntimePlayState::Paused => "⏸ Runtime pausado por ESC".to_string(),
                    RuntimePlayState::Playing => "▶ Runtime retomado por ESC".to_string(),
                    RuntimePlayState::Edit => String::new(),
                });
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("▶ Novo jogo").clicked() {
                        host.set_play_state(RuntimePlayState::Playing);
                        runtime.window_open = true;
                        let scene = host.active_scene_snapshot().clone();
                        runtime.start_from_scene_as_new_game(&scene);
                        host.set_status("▶ Novo jogo iniciado".to_string());
                    }
                    if ui.button("⤴ Continuar").clicked() {
                        host.set_play_state(RuntimePlayState::Playing);
                        runtime.window_open = true;
                        let scene = host.active_scene_snapshot().clone();
                        let project_root = host.project_root().to_path_buf();
                        match runtime.continue_from_save(&project_root, &scene) {
                            Ok(_) => host.set_status("⤴ Jogo continuado do save".to_string()),
                            Err(err) => {
                                runtime.start_from_scene(&scene);
                                host.set_status(format!("ℹ {} Abrindo cena atual do editor.", err));
                            }
                        }
                    }
                    if ui.button("⏹ Parar").clicked() {
                        host.set_play_state(RuntimePlayState::Edit);
                        runtime.stop();
                        runtime.window_open = false;
                        host.set_status("⏹ Runtime parado".to_string());
                    }
                    if ui.button("↻ Recarregar Cena").clicked() {
                        let scene = host.active_scene_snapshot().clone();
                        runtime.reload_current_scene(&scene);
                        host.set_status("↻ Cena recarregada".to_string());
                    }
                    ui.separator();
                    if ui.button("✖ Fechar runtime").clicked() {
                        host.set_play_state(RuntimePlayState::Edit);
                        runtime.stop();
                        runtime.window_open = false;
                        host.set_status("Runtime fechado".to_string());
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    let scene_candidates = host.scene_file_candidates();
                    let count = scene_candidates.len();
                    if count == 0 {
                        ui.label(
                            egui::RichText::new("Nenhuma cena salva encontrada.")
                                .small()
                                .weak(),
                        );
                    } else {
                        ui.menu_button(
                            egui::RichText::new(format!("Trocar cena ({}) v", count)).small(),
                            |ui| {
                                ui.set_min_width(220.0);
                                ui.label(egui::RichText::new("Cenas disponíveis:").small().weak());
                                ui.separator();
                                for path in scene_candidates {
                                    let label = path
                                        .file_stem()
                                        .and_then(|n| n.to_str())
                                        .map(|n| n.replace(".scene", ""))
                                        .unwrap_or_else(|| "Cena".to_string());
                                    if ui.button(format!("  Cena 2D  {}", label)).clicked() {
                                        runtime.queue_scene_change(path.clone());
                                        host.set_status(format!("Cena agendada: {}", label));
                                        ui.close_menu();
                                    }
                                }
                            },
                        );
                    }
                });

                show(host, runtime, ui);
            });
        },
    );
}

// ── show (preview inline) ────────────────────────────────────

/// Renderiza o preview do runtime na viewport principal do editor.
pub fn show<H: RuntimeContext>(host: &mut H, runtime: &mut RuntimeState, ui: &mut egui::Ui) {
    let play_state = host.play_state();
    let scene_snap = host.active_scene_snapshot().clone();
    let project_root = host.project_root().to_path_buf();

    runtime.sync_with_mode(&play_state, &scene_snap, &project_root, GROUND_Y);

    // captura input egui → RuntimeInput
    apply_egui_inputs(runtime, ui.ctx());

    if runtime.active_scene.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label("Runtime inativo.");
        });
        return;
    }

    if play_state == RuntimePlayState::Playing {
        // Limita a ~120 FPS para não saturar a CPU em spin-loop.
        ui.ctx().request_repaint_after(FRAME_BUDGET);
    }

    let (scene_name, ent_count, script_count) = {
        let s = runtime.active_scene.as_ref().expect("checked above");
        (
            s.name.clone(),
            renderer::count_entities(&s.entities),
            renderer::count_scripts(&s.entities),
        )
    };

    // ── HUD de debug ─────────────────────────────────────────
    ui.horizontal_wrapped(|ui| {
        ui.heading("▶ Runtime Preview");
        ui.separator();
        ui.label(format!("📌 {}", scene_name));
        ui.separator();
        ui.label(format!("⏱ {:.2}s", runtime.elapsed_time));
        ui.separator();
        ui.label(format!("FPS ~ {:.0}", runtime.estimated_fps()));
        ui.separator();
        ui.label(match play_state {
            RuntimePlayState::Playing => "Status: Executando",
            RuntimePlayState::Paused => "Status: Pausado",
            RuntimePlayState::Edit => "Status: Edição",
        });
        ui.separator();
        ui.label(format!("Flow: {:?}", runtime.game_state.flow));
        ui.separator();
        ui.label(format!("Etapa: {:?}", runtime.last_stage));
    });

    ui.label("WASD = player | Setas = câmera | Q/E ou scroll = zoom | ESC = pausa/continua");
    ui.separator();

    ui.horizontal_wrapped(|ui| {
        if ui.button("💾 Salvar").clicked() {
            match runtime.save_game(&project_root) {
                Ok(_) => host.set_status("💾 Jogo salvo".to_string()),
                Err(e) => host.set_status(format!("❌ Erro ao salvar: {e}")),
            }
        }
        if ui.button("📂 Carregar").clicked() {
            if runtime.load_game(&project_root) {
                host.set_status("📂 Save carregado".to_string());
            } else {
                host.set_status("ℹ Nenhum save encontrado".to_string());
            }
        }
    });
    ui.separator();

    // ── render ───────────────────────────────────────────────
    let available = ui.available_rect_before_wrap();
    // Sense::click() torna a área de jogo interativa para que o egui
    // entregue eventos de teclado (Space, etc.) ao runtime em vez de
    // consumi-los nos botões da barra de controle acima.
    let game_response = ui.allocate_rect(available, egui::Sense::click());
    if game_response.clicked() || game_response.hovered() {
        ui.ctx()
            .memory_mut(|mem| mem.request_focus(game_response.id));
    }
    let _response = game_response;
    let painter = ui.painter_at(available);

    let mut pending_ui_action = None;

    {
        let scene = runtime.active_scene.as_ref().expect("checked above");
        let bg = scene.background_color;
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
        let mut camera = camera::find_main_camera(&scene.entities);
        if let Some(zoom) = runtime.camera_zoom_override {
            camera.zoom = zoom.clamp(0.2, 4.0);
        }
        if runtime.camera_shake_time > f32::EPSILON && runtime.camera_shake_intensity > f32::EPSILON
        {
            let phase = runtime.elapsed_time * 40.0;
            let shake = runtime.camera_shake_intensity * runtime.camera_shake_time.clamp(0.0, 1.0);
            camera.x += phase.sin() * shake;
            camera.y += (phase * 1.37).cos() * shake;
        }

        let current_scene_path = runtime.scene_manager.current_path.clone();

        renderer::draw_runtime_particles(&painter, center, camera, &runtime.particles);

        for entity in &scene.entities {
            if let Some(action) = renderer::draw_runtime_entity(
                ui,
                &painter,
                &project_root,
                current_scene_path.as_deref(),
                host.sprite_textures(),
                entity,
                center,
                camera,
            ) {
                pending_ui_action = Some(action);
            }
        }
    }

    // ── ações de UI (UIButton) ───────────────────────────────
    if let Some(action) = pending_ui_action {
        match action {
            UiAction::ChangeScene(path) => {
                if let Some((scene, file_path)) = host.scene_snapshot_by_path(&path) {
                    let label = scene.name.clone();
                    runtime.queue_scene_change_snapshot(scene, file_path, Some(label.clone()));
                    host.set_status(format!("UIButton → cena {} (memória)", label));
                } else {
                    let label = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("cena")
                        .to_string();
                    runtime.queue_scene_change(path);
                    host.set_status(format!("UIButton → cena {}", label));
                }
            }
            UiAction::CloseRuntime => {
                host.set_play_state(RuntimePlayState::Edit);
                runtime.stop();
                runtime.window_open = false;
                host.set_status("UIButton → fechar runtime".to_string());
            }
        }
    }

    // ── overlays ─────────────────────────────────────────────
    if matches!(
        runtime.game_state.flow,
        crate::runtime::state::RuntimeGameFlow::Loading
    ) {
        painter.text(
            available.center(),
            egui::Align2::CENTER_CENTER,
            format!(
                "Loading... {}",
                runtime.game_state.loading_label.clone().unwrap_or_default()
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
            ent_count,
            script_count,
            runtime.delta_time * 1000.0,
        ),
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(220, 220, 220),
    );

    painter.text(
        egui::pos2(available.right() - 8.0, available.top() + 8.0),
        egui::Align2::RIGHT_TOP,
        format!(
            "HUD/UI  •  Cena: {}  •  Score: {}  •  Flow: {:?}  •  Partículas: {}",
            scene_name,
            runtime.game_state.score,
            runtime.game_state.flow,
            runtime.particles.len(),
        ),
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(180, 235, 180),
    );
}

// ── input helper ─────────────────────────────────────────────

fn apply_egui_inputs(runtime: &mut RuntimeState, ctx: &egui::Context) {
    use crate::runtime::systems::input_system;
    let new_input = input_system::capture_runtime_input(ctx, &runtime.input);
    runtime.input = new_input;

    // câmera com setas
    let cam_axis = input_system::camera_axis(&runtime.input);
    let zoom_d = input_system::zoom_delta(ctx);
    if let Some(scene) = &mut runtime.active_scene {
        if cam_axis != egui::Vec2::ZERO || zoom_d.abs() > f32::EPSILON {
            crate::runtime::camera::move_main_camera(
                &mut scene.entities,
                cam_axis.x * 4.0,
                cam_axis.y * 4.0,
                zoom_d * 0.05,
            );
        }
    }
}
