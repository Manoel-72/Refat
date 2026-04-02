// ============================================================
//  runtime/mod.rs
//  Runtime modularizado da engine.
// ============================================================

pub mod camera;
pub mod input;
pub mod renderer;
pub mod script;
pub mod state;
pub mod systems;

use eframe::egui;

use crate::{editor::{EditorApp, EditorPlayState}};

pub use state::{RuntimeInput, RuntimeState};

const GROUND_Y: f32 = -260.0;

pub fn show_viewport(app: &mut EditorApp, ctx: &egui::Context) {
    if app.play_state == EditorPlayState::Edit || !app.runtime.window_open {
        return;
    }

    let viewport_id = egui::ViewportId::from_hash_of("rust2d_runtime_viewport");
    ctx.show_viewport_immediate(
        viewport_id,
        egui::ViewportBuilder::default()
            .with_title("▶ Rust2D Runtime")
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
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("▶ Play").clicked() {
                        app.play_state = EditorPlayState::Playing;
                        app.runtime.window_open = true;
                        app.runtime.start_from_scene(&app.scene);
                        app.status_msg = "▶ Runtime em execução".to_string();
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
                        app.runtime.start_from_scene(&app.scene);
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
                show(app, ui);
            });
        },
    );
}

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
    });

    ui.label("Runtime jogável: carrega a cena atual, renderiza sprites, atualiza física e executa scripts anexados.");
    ui.label("WASD = mover entidade com @player_controller | Setas = mover câmera | Q/E ou scroll = zoom | R = recarregar cena");
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
}
