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

use eframe::egui;

use crate::{editor::{EditorApp, EditorPlayState}, core::version};

pub use state::{RuntimeInput, RuntimeState};

const GROUND_Y: f32 = -260.0;

/// Exibe a janela nativa do runtime quando a engine está em Play/Pause.
pub fn show_viewport(app: &mut EditorApp, ctx: &egui::Context) {
    if app.play_state == EditorPlayState::Edit || !app.runtime.window_open {
        return;
    }

    let viewport_id = egui::ViewportId::from_hash_of("rs2br_runtime_viewport");
    ctx.show_viewport_immediate(
        viewport_id,
        egui::ViewportBuilder::default()
            .with_title(&format!("▶ {} {} Runtime", version::ENGINE_TITLE, version::ENGINE_VERSION))
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([480.0, 320.0]),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                app.play_state = EditorPlayState::Edit;
                app.runtime.stop();
                app.runtime.window_open = false;
                app.status_msg = "Runtime fechado".to_string();
                return;
            }

            if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                app.play_state = match app.play_state {
                    EditorPlayState::Playing => EditorPlayState::Paused,
                    EditorPlayState::Paused => EditorPlayState::Playing,
                    EditorPlayState::Edit => EditorPlayState::Edit,
                };
                app.status_msg = match app.play_state {
                    EditorPlayState::Paused => "⏸ Runtime pausado por ESC".to_string(),
                    EditorPlayState::Playing => "▶ Runtime retomado por ESC".to_string(),
                    EditorPlayState::Edit => app.status_msg.clone(),
                };
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("▶ Play").clicked() {
                        app.play_state = EditorPlayState::Playing;
                        app.runtime.window_open = true;

                        let maybe_path = app
                            .open_scenes
                            .get(app.active_scene_index)
                            .and_then(|doc| doc.file_path.clone());

                        if let Some(path) = maybe_path {
                            if let Err(error) = app.runtime.start_from_path(path.clone()) {
                                app.runtime.start_from_scene(&app.scene);
                                app.status_msg = format!(
                                    "▶ Runtime iniciado com cena em memória (falha ao carregar arquivo: {})",
                                    error
                                );
                            } else {
                                app.status_msg = format!(
                                    "▶ Runtime iniciado com '{}'",
                                    path.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("cena")
                                );
                            }
                        } else {
                            app.runtime.start_from_scene(&app.scene);
                            app.status_msg = "▶ Runtime iniciado com a cena atual em memória".to_string();
                        }
                    }

                    if ui.button("⏸ Pause").clicked() {
                        app.play_state = EditorPlayState::Paused;
                        app.status_msg = "⏸ Runtime pausado".to_string();
                    }

                    if ui.button("⏹ Parar").clicked() {
                        app.play_state = EditorPlayState::Edit;
                        app.runtime.stop();
                        app.runtime.window_open = false;
                        app.status_msg = "⏹ Runtime parado".to_string();
                    }

                    if ui.button("↻ Recarregar Cena").clicked() {
                        app.runtime.reload_current_scene(&app.scene);
                        app.status_msg = "↻ Cena recarregada no runtime".to_string();
                    }

                    ui.separator();

                    if ui.button("✖ Fechar runtime").clicked() {
                        app.play_state = EditorPlayState::Edit;
                        app.runtime.stop();
                        app.runtime.window_open = false;
                        app.status_msg = "Runtime fechado".to_string();
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label("Trocar cena no runtime:");
                    for path in app.scene_file_candidates() {
                        let label = path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .map(|name| name.replace(".scene", ""))
                            .unwrap_or_else(|| "Cena".to_string());

                        if ui.small_button(format!("🎬 {}", label)).clicked() {
                            app.runtime.queue_scene_change(path.clone());
                            app.status_msg = format!("Cena agendada para runtime: {}", label);
                        }
                    }
                });

                show(app, ui);
            });
        },
    );
}

/// Exibe o preview do runtime dentro da viewport principal.
pub fn show(app: &mut EditorApp, ui: &mut egui::Ui) {
    let play_state = app.play_state;
    let editor_scene_snapshot = app.scene.clone();
    app.runtime
        .sync_with_mode(play_state, &editor_scene_snapshot, &app.project_root, GROUND_Y);

    input::apply_runtime_inputs(app, ui.ctx());

    let Some(runtime_scene) = app.runtime.active_scene.clone() else {
        ui.centered_and_justified(|ui| {
            ui.label("Runtime inativo.");
        });
        return;
    };

    if app.play_state != EditorPlayState::Edit {
        ui.ctx().request_repaint();
    }

    ui.horizontal_wrapped(|ui| {
        ui.heading("▶ Runtime Preview");
        ui.separator();
        ui.label(format!("📌 {}", runtime_scene.name));
        ui.separator();
        ui.label(format!("⏱ {:.2}s", app.runtime.elapsed_time));
        ui.separator();
        ui.label(format!("FPS ~ {:.0}", app.runtime.estimated_fps()));
        ui.separator();
        ui.label(match app.play_state {
            EditorPlayState::Playing => "Status: Executando",
            EditorPlayState::Paused => "Status: Pausado",
            EditorPlayState::Edit => "Status: Edição",
        });
        ui.separator();
        ui.label(format!("Flow: {:?}", app.runtime.game_state.flow));
        ui.separator();
        ui.label(format!("Etapa: {:?}", app.runtime.last_stage));
    });

    ui.label("Runtime jogável: carrega a cena atual, renderiza sprites, atualiza física e executa scripts anexados.");
    ui.label("WASD = mover entidade com @player_controller | Setas = mover câmera | Q/E ou scroll = zoom | ESC = pause");
    ui.separator();

    ui.horizontal_wrapped(|ui| {
        if ui.button("Spawn Enemy").clicked() {
            let spawn_x = 120.0 + (app.runtime.frame_count % 5) as f32 * 36.0;
            app.runtime.queue_spawn("enemy", spawn_x, 48.0);
            app.runtime.game_state.score += 1;
            app.status_msg = "Spawn agendado para o fim do frame".to_string();
        }

        if ui.button("Destroy Last Spawn").clicked() {
            if app.runtime.queue_destroy_last_spawned() {
                app.status_msg = "Destroy agendado para o fim do frame".to_string();
            } else {
                app.status_msg = "Nenhuma entidade spawnada para destruir".to_string();
            }
        }

        if ui.button("Pause/Resume").clicked() {
            app.play_state = match app.play_state {
                EditorPlayState::Playing => EditorPlayState::Paused,
                EditorPlayState::Paused => EditorPlayState::Playing,
                EditorPlayState::Edit => EditorPlayState::Edit,
            };
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

    for entity in &runtime_scene.entities {
        renderer::draw_runtime_entity(app, ui, &painter, entity, center, camera);
    }

    if matches!(app.runtime.game_state.flow, crate::runtime::state::RuntimeGameFlow::Loading) {
        painter.text(
            available.center(),
            egui::Align2::CENTER_CENTER,
            format!("Loading... {}", app.runtime.game_state.loading_label.clone().unwrap_or_default()),
            egui::FontId::proportional(22.0),
            egui::Color32::WHITE,
        );
    }

    if runtime_scene.name.to_ascii_lowercase().contains("menu") {
        let menu_center = available.center();
        egui::Area::new(egui::Id::new("runtime_main_menu_overlay"))
            .fixed_pos(egui::pos2(menu_center.x - 140.0, menu_center.y - 88.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::window(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(18, 20, 24, 240))
                    .stroke(egui::Stroke::new(1.5, egui::Color32::from_rgb(92, 76, 140)))
                    .rounding(egui::Rounding::same(14.0))
                    .inner_margin(egui::Margin::symmetric(18.0, 16.0))
                    .show(ui, |ui| {
                        ui.set_min_width(280.0);
                        ui.vertical_centered(|ui| {
                            ui.heading(
                                egui::RichText::new("Menu Principal")
                                    .size(28.0)
                                    .strong()
                                    .color(egui::Color32::from_rgb(235, 238, 245)),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new("Selecione uma ação para continuar")
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(165, 172, 185)),
                            );
                        });

                        ui.add_space(14.0);

                        let button_size = egui::vec2(220.0, 44.0);

                        ui.vertical_centered(|ui| {
                            let play_response = ui.scope(|ui| {
                                let visuals = &mut ui.style_mut().visuals;
                                visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(36, 148, 76);
                                visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(46, 168, 90);
                                visuals.widgets.active.bg_fill = egui::Color32::from_rgb(28, 125, 64);
                                visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(36, 148, 76);
                                visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(46, 168, 90);
                                visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(28, 125, 64);
                                visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                                visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                                visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                                visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(72, 190, 108));
                                visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(108, 210, 136));
                                visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(56, 170, 92));

                                ui.add_sized(
                                    button_size,
                                    egui::Button::new(
                                        egui::RichText::new("▶ Entrar no Jogo")
                                            .size(20.0)
                                            .strong(),
                                    ),
                                )
                            }).inner;

                            if play_response.clicked() {
                                if let Some(path) = app.scene_file_candidates().into_iter().find(|path| {
                                    path.file_stem()
                                        .and_then(|n| n.to_str())
                                        .map(|name| !name.to_ascii_lowercase().contains("menu"))
                                        .unwrap_or(false)
                                }) {
                                    app.runtime.queue_scene_change(path.clone());
                                    app.status_msg = format!("Cena agendada: {}", path.display());
                                }
                            }

                            ui.add_space(10.0);

                            let exit_response = ui.scope(|ui| {
                                let visuals = &mut ui.style_mut().visuals;
                                visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(172, 52, 52);
                                visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(194, 66, 66);
                                visuals.widgets.active.bg_fill = egui::Color32::from_rgb(144, 40, 40);
                                visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(172, 52, 52);
                                visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(194, 66, 66);
                                visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(144, 40, 40);
                                visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                                visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                                visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                                visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(215, 94, 94));
                                visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(228, 118, 118));
                                visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(192, 76, 76));

                                ui.add_sized(
                                    button_size,
                                    egui::Button::new(
                                        egui::RichText::new("✕ Sair")
                                            .size(20.0)
                                            .strong(),
                                    ),
                                )
                            }).inner;

                            if exit_response.clicked() {
                                app.play_state = EditorPlayState::Edit;
                                app.runtime.stop();
                                app.runtime.window_open = false;
                                app.status_msg = "Runtime encerrado pelo menu".to_string();
                            }
                        });
                    });
            });
    }

    painter.text(
        egui::pos2(available.left() + 8.0, available.top() + 8.0),
        egui::Align2::LEFT_TOP,
        format!(
            "Entidades: {}  •  Scripts: {}  •  Delta: {:.3} ms",
            renderer::count_entities(&runtime_scene.entities),
            renderer::count_scripts(&runtime_scene.entities),
            app.runtime.delta_time * 1000.0,
        ),
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(220, 220, 220),
    );

    painter.text(
        egui::pos2(available.right() - 8.0, available.top() + 8.0),
        egui::Align2::RIGHT_TOP,
        format!(
            "HUD  •  Cena: {}  •  Score: {}  •  Flow: {:?}",
            runtime_scene.name,
            app.runtime.game_state.score,
            app.runtime.game_state.flow,
        ),
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(180, 235, 180),
    );
}
