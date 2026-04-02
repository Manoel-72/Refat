// ============================================================
//  runtime/mod.rs
//  Runtime básico da engine — preview de jogo dentro do editor.
//  Responsável por clonar a cena, simular um loop simples e
//  renderizar sprites/câmera em modo Play.
// ============================================================

pub mod scene_manager;

use std::{collections::HashSet, fs, path::{Path, PathBuf}, time::Instant};
#[derive(Default, Clone)]
pub struct RuntimeInput {
    pub key_w: bool,
    pub key_a: bool,
    pub key_s: bool,
    pub key_d: bool,
    pub key_up: bool,
    pub key_down: bool,
    pub key_left: bool,
    pub key_right: bool,
    pub key_space: bool,
    pub key_enter: bool,
    pub mouse_left: bool,
    pub mouse_right: bool,
    pub mouse_middle: bool,
    pub mouse_pos: (f32, f32),
}

use eframe::egui;

use self::scene_manager::SceneManager;

use crate::{
    editor::{EditorApp, EditorPlayState},
    engine::{component::Component, entity::Entity, scene::Scene},
};

const GROUND_Y: f32 = -260.0;

/// Estado transitório do runtime em execução dentro do editor.
pub struct RuntimeState {
    /// Cena usada na simulação em modo Play (clone da cena do editor)
    pub active_scene: Option<Scene>,
    /// Tempo total desde o começo da execução
    pub elapsed_time: f32,
    /// Delta time do último frame em segundos
    pub delta_time: f32,
    /// Quantidade de frames simulados
    pub frame_count: u64,
    /// Instante do último frame processado
    last_frame_at: Option<Instant>,
    /// Se a janela nativa do runtime está aberta
    pub window_open: bool,
    /// Scripts que já receberam o ciclo de start durante a sessão atual
    started_scripts: HashSet<String>,
    /// Estado de input do frame atual
    pub input: RuntimeInput,
    /// Gerencia trocas e recarga de cenas no runtime
    pub scene_manager: SceneManager,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            active_scene: None,
            elapsed_time: 0.0,
            delta_time: 0.0,
            frame_count: 0,
            last_frame_at: None,
            window_open: false,
            started_scripts: HashSet::new(),
            input: RuntimeInput::default(),
            scene_manager: SceneManager::new(),
        }
    }

    /// Inicia o runtime clonando a cena atual do editor.
    pub fn start_from_scene(&mut self, scene: &Scene) {
        self.scene_manager.set_editor_scene(scene.clone());
        self.active_scene = self.scene_manager.current_scene.clone();
        self.reset_timing_state();
    }

    pub fn start_from_path(&mut self, path: PathBuf) -> Result<(), String> {
        self.scene_manager
            .load_scene(path)
            .map_err(|e| format!("Falha ao carregar cena: {}", e))?;
        self.active_scene = self.scene_manager.current_scene.clone();
        self.reset_timing_state();
        Ok(())
    }

    pub fn queue_scene_change(&mut self, path: PathBuf) {
        self.scene_manager.change_scene(path);
    }

    pub fn reload_current_scene(&mut self, fallback_scene: &Scene) {
        if self.scene_manager.reload_scene().is_err() {
            self.start_from_scene(fallback_scene);
        } else {
            self.active_scene = self.scene_manager.current_scene.clone();
            self.reset_timing_state();
        }
    }

    fn reset_timing_state(&mut self) {
        self.elapsed_time = 0.0;
        self.delta_time = 0.0;
        self.frame_count = 0;
        self.last_frame_at = Some(Instant::now());
        self.started_scripts.clear();
        self.input = RuntimeInput::default();
    }

    /// Encerra a execução temporária do runtime.
    pub fn stop(&mut self) {
        self.active_scene = None;
        self.scene_manager.clear_runtime_scene();
        self.elapsed_time = 0.0;
        self.delta_time = 0.0;
        self.frame_count = 0;
        self.last_frame_at = None;
        self.started_scripts.clear();
        self.input = RuntimeInput::default();
    }

    /// Mantém o runtime sincronizado com o estado atual do editor.
    pub fn sync_with_mode(&mut self, mode: EditorPlayState, source_scene: &Scene, project_root: &Path) {
        if matches!(self.scene_manager.apply_pending_change(), Ok(true)) {
            self.active_scene = self.scene_manager.current_scene.clone();
            self.reset_timing_state();
        }

        match mode {
            EditorPlayState::Edit => {
                if self.active_scene.is_some() {
                    self.stop();
                }
                self.window_open = false;
            }
            EditorPlayState::Playing => {
                if self.active_scene.is_none() {
                    self.start_from_scene(source_scene);
                }
                self.window_open = true;
                self.update_frame(project_root);
            }
            EditorPlayState::Paused => {
                if self.active_scene.is_none() {
                    self.start_from_scene(source_scene);
                }
                self.window_open = true;
                self.delta_time = 0.0;
                self.last_frame_at = Some(Instant::now());
            }
        }
    }

    fn update_frame(&mut self, project_root: &Path) {
        let now = Instant::now();
        let dt = self
            .last_frame_at
            .map(|last| (now - last).as_secs_f32().clamp(1.0 / 240.0, 1.0 / 20.0))
            .unwrap_or(1.0 / 60.0);

        self.last_frame_at = Some(now);
        self.delta_time = dt;
        self.elapsed_time += dt;
        self.frame_count += 1;

        // input será atualizado externamente a cada frame

        if let Some(scene) = &mut self.active_scene {
            let mut camera_follow_target = None;
            update_entities_runtime(
                &mut scene.entities,
                dt,
                self.elapsed_time,
                project_root,
                &mut self.started_scripts,
                &mut camera_follow_target,
            );
            if let Some((x, y)) = camera_follow_target {
                set_main_camera_position(&mut scene.entities, x, y);
            }
            self.scene_manager.current_scene = Some(scene.clone());
        }
    }

    pub fn estimated_fps(&self) -> f32 {
        if self.delta_time <= f32::EPSILON {
            0.0
        } else {
            1.0 / self.delta_time
        }
    }
}

/// Exibe o runtime em uma janela nativa separada enquanto o editor está em Play/Pause.
pub fn show_viewport(app: &mut EditorApp, ctx: &egui::Context) {
    if app.play_state == EditorPlayState::Edit || !app.runtime.window_open {
        return;
    }

    let viewport_id = egui::ViewportId::from_hash_of("rust2d_runtime_viewport");
    ctx.show_viewport_immediate(
        viewport_id,
        egui::ViewportBuilder::default()
            .with_title("▶ RS2BR Runtime")
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([480.0, 320.0]),
        |ctx, _class| {
            // Detecta fechamento nativo da janela (botão X)
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
                        let label = path.file_stem()
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
    app.runtime.sync_with_mode(play_state, &editor_scene_snapshot, &app.project_root);
    apply_runtime_inputs(app, ui.ctx());

    let Some(runtime_scene) = app.runtime.active_scene.clone() else {
        ui.centered_and_justified(|ui| {
            ui.label("Runtime inativo.");
        });
        return;
    };
    let _script_on_update: Vec<String> = Vec::new();

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
    let (camera_x, camera_y, camera_zoom) = find_main_camera(&runtime_scene.entities);

    draw_runtime_grid(&painter, available, center, camera_x, camera_y, camera_zoom);

    let ground_screen_y = center.y - ((GROUND_Y - camera_y) * camera_zoom);
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
        draw_runtime_entity(app, ui, &painter, entity, center, camera_x, camera_y, camera_zoom);
    }

    painter.text(
        egui::pos2(available.left() + 8.0, available.top() + 8.0),
        egui::Align2::LEFT_TOP,
        format!(
            "Entidades: {}  •  Scripts: {}  •  Delta: {:.3} ms",
            count_entities(&runtime_scene.entities),
            count_scripts(&runtime_scene.entities),
            app.runtime.delta_time * 1000.0,
        ),
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(220, 220, 220),
    );
}

// Função utilitária de colisão AABB
fn aabb_collision(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

fn update_entities_runtime(
    entities: &mut [Entity],
    delta_time: f32,
    _elapsed_time: f32,
    project_root: &Path,
    started_scripts: &mut HashSet<String>,
    camera_follow_target: &mut Option<(f32, f32)>,
) {
    // Coletar todas as entidades com Transform e BoxCollider
    let mut colliders = Vec::new();
    for entity in entities.iter() {
        let mut t_opt = None;
        let mut c_opt = None;
        for comp in &entity.components {
            match comp {
                Component::Transform(t) => t_opt = Some(t),
                Component::BoxCollider(c) => c_opt = Some(c),
                _ => {}
            }
        }
        if let (Some(t), Some(c)) = (t_opt, c_opt) {
            colliders.push((t.x + c.offset_x, t.y + c.offset_y, c.width, c.height, entity as *const Entity));
        }
    }

    for entity in entities {
        let mut gravity_scale = None;
        let mut is_static = false;
        let mut collider_half_height = 0.0_f32;
        let mut script_move_x = 0.0_f32;
        let mut script_move_y = 0.0_f32;
        let mut script_rotate_speed = 0.0_f32;
        let mut should_follow_camera = false;

        for component in &entity.components {
            match component {
                Component::RigidBody2D(rb) => {
                    gravity_scale = Some(rb.gravity_scale);
                    is_static = rb.is_static;
                }
                Component::BoxCollider(collider) => {
                    collider_half_height = collider.height.max(0.0) * 0.5;
                }
                Component::Script(script) => {
                    if let Some(behavior) = load_script_behavior(project_root, &script.file_path) {
                        let script_key = format!("{}::{}", entity.id, script.file_path);
                        if started_scripts.insert(script_key) {
                            if let Some(message) = &behavior.start_message {
                                println!("▶ Script '{}' em '{}': {}", script.file_path, entity.name, message);
                            } else {
                                println!("▶ Script '{}' iniciado em '{}'", script.file_path, entity.name);
                            }
                        }

                        script_move_x += behavior.move_x;
                        script_move_y += behavior.move_y;
                        script_rotate_speed += behavior.rotate_speed;
                        should_follow_camera |= behavior.camera_follow;
                    }
                }
                _ => {}
            }
        }

        // ── CORREÇÃO: extrai dados do collider e o ponteiro ANTES do get_mut ──
        // Isso evita conflito de borrow: get_mut segura uma &mut em components,
        // mas iterar ou chamar ptr::eq também precisa de acesso a entity.
        let my_collider_data = entity.components.iter().find_map(|comp| {
            if let Component::BoxCollider(c) = comp {
                Some((c.offset_x, c.offset_y, c.width, c.height))
            } else {
                None
            }
        });
        let entity_ptr = entity as *const Entity;

        if let Some(t) = entity.transform_mut() {
            t.x += script_move_x * delta_time;
            t.y += script_move_y * delta_time;
            t.rotation += script_rotate_speed * delta_time;

            // Salva posição anterior para resposta de colisão
            let (old_x, old_y) = (t.x, t.y);

            if !is_static {
                if let Some(gravity) = gravity_scale {
                    t.y -= 180.0 * gravity.max(0.0) * delta_time;
                }
            }

            // --- Colisão entre entidades ---
            // Usa os dados já extraídos antes do get_mut, sem precisar
            // reler &entity.components dentro do bloco de borrow mutável.
            if let Some((off_x, off_y, width, height)) = my_collider_data {
                let my_rect = (
                    t.x + off_x - width * 0.5,
                    t.y + off_y - height * 0.5,
                    width,
                    height,
                );
                for (ox, oy, ow, oh, other_ptr) in &colliders {
                    // Não colidir consigo mesmo — usa o ponteiro salvo antes
                    if std::ptr::eq(entity_ptr, *other_ptr) { continue; }
                    let other_rect = (
                        *ox - *ow * 0.5,
                        *oy - *oh * 0.5,
                        *ow,
                        *oh,
                    );
                    if aabb_collision(my_rect, other_rect) {
                        // Resposta simples: volta para posição anterior
                        t.x = old_x;
                        t.y = old_y;
                        break;
                    }
                }
            }

            // Chão fixo (GROUND_Y) ainda impede cair infinitamente
            let floor_y = GROUND_Y + collider_half_height.max(16.0);
            if t.y < floor_y {
                t.y = floor_y;
            }

            if should_follow_camera {
                *camera_follow_target = Some((t.x, t.y));
            }
        }

        update_entities_runtime(
            &mut entity.children,
            delta_time,
            _elapsed_time,
            project_root,
            started_scripts,
            camera_follow_target,
        );
    }
}

fn find_main_camera(entities: &[Entity]) -> (f32, f32, f32) {
    for entity in entities {
        let mut transform = None;
        let mut camera_zoom = None;

        for component in &entity.components {
            match component {
                Component::Transform(t) => transform = Some((t.x, t.y)),
                Component::Camera2D(cam) if cam.is_main => camera_zoom = Some(cam.zoom.max(0.1)),
                _ => {}
            }
        }

        if let (Some((x, y)), Some(zoom)) = (transform, camera_zoom) {
            return (x, y, zoom);
        }

        let nested = find_main_camera(&entity.children);
        if nested != (0.0, 0.0, 1.0) {
            return nested;
        }
    }

    (0.0, 0.0, 1.0)
}

fn draw_runtime_grid(
    painter: &egui::Painter,
    rect: egui::Rect,
    center: egui::Pos2,
    camera_x: f32,
    camera_y: f32,
    camera_zoom: f32,
) {
    let grid_color = egui::Color32::from_rgba_unmultiplied(90, 90, 100, 60);
    let grid_size = (32.0_f32 * camera_zoom).clamp(8.0, 128.0);
    let center = egui::pos2(center.x - camera_x * camera_zoom, center.y + camera_y * camera_zoom);

    let start_x = center.x % grid_size;
    let mut x = rect.left() + (start_x - rect.left()).rem_euclid(grid_size);
    while x <= rect.right() {
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(0.5, grid_color),
        );
        x += grid_size;
    }

    let start_y = center.y % grid_size;
    let mut y = rect.top() + (start_y - rect.top()).rem_euclid(grid_size);
    while y <= rect.bottom() {
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(0.5, grid_color),
        );
        y += grid_size;
    }
}

fn draw_runtime_entity(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    entity: &Entity,
    center: egui::Pos2,
    camera_x: f32,
    camera_y: f32,
    camera_zoom: f32,
) {
    if !entity.visible {
        return;
    }

    let (ex, ey, rotation, scale_x, scale_y) = match entity.components.first() {
        Some(Component::Transform(t)) => (t.x, t.y, t.rotation, t.scale_x, t.scale_y),
        _ => (0.0, 0.0, 0.0, 1.0, 1.0),
    };

    let screen_pos = egui::pos2(
        center.x + ((ex - camera_x) * camera_zoom),
        center.y - ((ey - camera_y) * camera_zoom),
    );

    let mut sprite: Option<&crate::engine::component::Sprite> = None;
    let mut has_camera = false;

    for component in &entity.components {
        match component {
            Component::Sprite(s) => sprite = Some(s),
            Component::Camera2D(_) => has_camera = true,
            _ => {}
        }
    }

    if let Some(sprite_comp) = sprite {
        let tint = egui::Color32::from_rgba_unmultiplied(
            (sprite_comp.color_r * 255.0) as u8,
            (sprite_comp.color_g * 255.0) as u8,
            (sprite_comp.color_b * 255.0) as u8,
            (sprite_comp.color_a * 255.0) as u8,
        );

        let default_size = egui::vec2(
            (64.0 * scale_x.abs().max(0.25) * camera_zoom).clamp(8.0, 512.0),
            (64.0 * scale_y.abs().max(0.25) * camera_zoom).clamp(8.0, 512.0),
        );

        if !sprite_comp.texture_path.trim().is_empty() {
            if let Some(texture) = app.load_texture_from_relative_path(ui.ctx(), &sprite_comp.texture_path) {
                let tex_size = texture.size_vec2();
                let draw_size = egui::vec2(
                    (tex_size.x * scale_x.abs().max(0.25) * camera_zoom).clamp(8.0, 512.0),
                    (tex_size.y * scale_y.abs().max(0.25) * camera_zoom).clamp(8.0, 512.0),
                );
                paint_rotated_image(
                    painter,
                    texture.id(),
                    screen_pos,
                    draw_size,
                    rotation,
                    tint,
                    scale_x < 0.0,
                    scale_y < 0.0,
                );
            } else {
                paint_rotated_placeholder(
                    painter,
                    screen_pos,
                    default_size,
                    rotation,
                    tint.linear_multiply(0.25),
                    egui::Stroke::new(1.0, tint),
                );
            }
        } else {
            paint_rotated_placeholder(
                painter,
                screen_pos,
                default_size,
                rotation,
                tint.linear_multiply(0.25),
                egui::Stroke::new(1.0, tint),
            );
        }
    } else if has_camera {
        let rect = egui::Rect::from_center_size(screen_pos, egui::vec2(80.0, 52.0));
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.5, egui::Color32::from_rgb(180, 120, 255)),
        );
    } else {
        painter.circle_filled(
            screen_pos,
            5.0,
            egui::Color32::from_rgb(120, 190, 255),
        );
    }

    for child in &entity.children {
        draw_runtime_entity(app, ui, painter, child, center, camera_x, camera_y, camera_zoom);
    }
}

fn count_entities(entities: &[Entity]) -> usize {
    entities.iter().map(|e| 1 + count_entities(&e.children)).sum()
}

fn count_scripts(entities: &[Entity]) -> usize {
    let mut count = 0;
    for entity in entities {
        for component in &entity.components {
            if matches!(component, Component::Script(_)) {
                count += 1;
            }
        }
        count += count_scripts(&entity.children);
    }
    count
}

#[derive(Debug, Default, Clone)]
struct ScriptBehavior {
    move_x: f32,
    move_y: f32,
    rotate_speed: f32,
    player_controller_speed: f32,
    camera_follow: bool,
    start_message: Option<String>,
    on_update: Vec<String>, // cada linha é um comando
}

fn apply_runtime_inputs(app: &mut EditorApp, ctx: &egui::Context) {
    let step = if app.play_state == EditorPlayState::Paused {
        1.0 / 60.0
    } else {
        app.runtime.delta_time.max(1.0 / 120.0)
    };

    // Atualiza input global do runtime
    ctx.input(|i| {
        let mut input = crate::runtime::RuntimeInput::default();
        input.key_w = i.key_down(egui::Key::W);
        input.key_a = i.key_down(egui::Key::A);
        input.key_s = i.key_down(egui::Key::S);
        input.key_d = i.key_down(egui::Key::D);
        input.key_up = i.key_down(egui::Key::ArrowUp);
        input.key_down = i.key_down(egui::Key::ArrowDown);
        input.key_left = i.key_down(egui::Key::ArrowLeft);
        input.key_right = i.key_down(egui::Key::ArrowRight);
        input.key_space = i.key_down(egui::Key::Space);
        input.key_enter = i.key_down(egui::Key::Enter);
        input.mouse_left = i.pointer.button_down(egui::PointerButton::Primary);
        input.mouse_right = i.pointer.button_down(egui::PointerButton::Secondary);
        input.mouse_middle = i.pointer.button_down(egui::PointerButton::Middle);
        input.mouse_pos = i.pointer.hover_pos().map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
        app.runtime.input = input;
    });

    let (player_input, camera_input, zoom_input, reload_scene) = ctx.input(|i| {
        let mut player = egui::vec2(0.0, 0.0);
        if i.key_down(egui::Key::A) { player.x -= 1.0; }
        if i.key_down(egui::Key::D) { player.x += 1.0; }
        if i.key_down(egui::Key::W) { player.y += 1.0; }
        if i.key_down(egui::Key::S) { player.y -= 1.0; }

        let mut camera = egui::vec2(0.0, 0.0);
        if i.key_down(egui::Key::ArrowLeft) { camera.x -= 1.0; }
        if i.key_down(egui::Key::ArrowRight) { camera.x += 1.0; }
        if i.key_down(egui::Key::ArrowUp) { camera.y += 1.0; }
        if i.key_down(egui::Key::ArrowDown) { camera.y -= 1.0; }

        let mut zoom = 0.0;
        if i.key_down(egui::Key::Q) { zoom -= 1.0; }
        if i.key_down(egui::Key::E) { zoom += 1.0; }
        zoom += i.raw_scroll_delta.y * 0.02;

        (player, camera, zoom, i.key_pressed(egui::Key::R))
    });

    if reload_scene {
        app.runtime.reload_current_scene(&app.scene);
        return;
    }

    let Some(scene) = app.runtime.active_scene.as_mut() else {
        return;
    };

    if player_input != egui::Vec2::ZERO {
        apply_player_controller_input(&mut scene.entities, &app.project_root, step, player_input);
    }

    if camera_input != egui::Vec2::ZERO || zoom_input.abs() > f32::EPSILON {
        let camera_speed = 260.0 * step;
        move_main_camera(
            &mut scene.entities,
            camera_input.x * camera_speed,
            camera_input.y * camera_speed,
            zoom_input * step.max(0.02),
        );
    }
}

fn apply_player_controller_input(
    entities: &mut [Entity],
    project_root: &Path,
    delta_time: f32,
    input: egui::Vec2,
) {
    for entity in entities {
        let mut controller_speed = 0.0_f32;

        for component in &entity.components {
            if let Component::Script(script) = component {
                if let Some(behavior) = load_script_behavior(project_root, &script.file_path) {
                    controller_speed = controller_speed.max(behavior.player_controller_speed.abs());
                }
            }
        }

        if controller_speed > 0.0 {
            if let Some(t) = entity.transform_mut() {
                t.x += input.x * controller_speed * delta_time;
                t.y += input.y * controller_speed * delta_time;
            }
        }

        apply_player_controller_input(&mut entity.children, project_root, delta_time, input);
    }
}

fn move_main_camera(entities: &mut [Entity], dx: f32, dy: f32, zoom_delta: f32) -> bool {
    for entity in entities {
        let has_main_camera = entity.components.iter().any(|component| {
            matches!(component, Component::Camera2D(cam) if cam.is_main)
        });

        if has_main_camera {
            if let Some(t) = entity.transform_mut() {
                t.x += dx;
                t.y += dy;
            }

            for component in &mut entity.components {
                if let Component::Camera2D(cam) = component {
                    if cam.is_main {
                        cam.zoom = (cam.zoom + zoom_delta).clamp(0.2, 4.0);
                    }
                }
            }
            return true;
        }

        if move_main_camera(&mut entity.children, dx, dy, zoom_delta) {
            return true;
        }
    }

    false
}

fn set_main_camera_position(entities: &mut [Entity], x: f32, y: f32) -> bool {
    for entity in entities {
        let has_main_camera = entity.components.iter().any(|component| {
            matches!(component, Component::Camera2D(cam) if cam.is_main)
        });

        if has_main_camera {
            if let Some(t) = entity.transform_mut() {
                t.x = x;
                t.y = y;
            }
            return true;
        }

        if set_main_camera_position(&mut entity.children, x, y) {
            return true;
        }
    }

    false
}

fn load_script_behavior(project_root: &Path, raw_path: &str) -> Option<ScriptBehavior> {
    let full_path = resolve_script_path(project_root, raw_path)?;
    let source = fs::read_to_string(full_path).ok()?;
    Some(parse_script_behavior(&source))
}

fn resolve_script_path(project_root: &Path, raw_path: &str) -> Option<PathBuf> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.replace('\\', "/");
    let candidates = [
        project_root.join(&normalized),
        project_root.join("assets").join(&normalized),
        project_root.join("assets/scripts").join(&normalized),
    ];

    candidates.into_iter().find(|path| path.exists())
}

fn parse_script_behavior(source: &str) -> ScriptBehavior {
    let mut behavior = ScriptBehavior::default();

    for line in source.lines() {
        if let Some(value) = parse_script_number(line, &["@move_x", "move_x"]) {
            behavior.move_x = value;
        }
        if let Some(value) = parse_script_number(line, &["@move_y", "move_y"]) {
            behavior.move_y = value;
        }
        if let Some(value) = parse_script_number(line, &["@rotate_speed", "rotate_speed"]) {
            behavior.rotate_speed = value;
        }
        if let Some(value) = parse_script_number(
            line,
            &["@player_controller", "player_controller", "player_controller_speed"],
        ) {
            behavior.player_controller_speed = value.abs();
        }
        if let Some(value) = parse_script_text(line, &["@start_message", "start_message"]) {
            behavior.start_message = Some(value);
        }

        let clean = line.trim().trim_start_matches('/').trim();
        if clean.contains("@camera_follow") || clean.contains("camera_follow = true") {
            behavior.camera_follow = true;
        }
        if let Some(rest) = clean.strip_prefix("@on_update") {
            behavior.on_update.push(rest.trim().to_string());
        }
    }

    behavior
}

fn parse_script_number(line: &str, names: &[&str]) -> Option<f32> {
    let clean = line.trim().trim_start_matches('/').trim().trim_end_matches(';');
    for name in names {
        if let Some(rest) = clean.strip_prefix(name) {
            let value = rest.trim_start_matches(':').trim_start_matches('=').trim();
            if let Ok(parsed) = value.parse::<f32>() {
                return Some(parsed);
            }
        }
    }
    None
}

fn parse_script_text(line: &str, names: &[&str]) -> Option<String> {
    let clean = line.trim().trim_start_matches('/').trim().trim_end_matches(';');
    for name in names {
        if let Some(rest) = clean.strip_prefix(name) {
            let value = rest
                .trim_start_matches(':')
                .trim_start_matches('=')
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn rotate_vec2(vec: egui::Vec2, rotation_deg: f32) -> egui::Vec2 {
    let (sin, cos) = rotation_deg.to_radians().sin_cos();
    egui::vec2(
        vec.x * cos - vec.y * sin,
        vec.x * sin + vec.y * cos,
    )
}

fn rotated_rect_points(
    center: egui::Pos2,
    size: egui::Vec2,
    rotation_deg: f32,
) -> [egui::Pos2; 4] {
    let half = size * 0.5;
    let corners = [
        egui::vec2(-half.x, -half.y),
        egui::vec2(half.x, -half.y),
        egui::vec2(half.x, half.y),
        egui::vec2(-half.x, half.y),
    ];

    corners.map(|corner| center + rotate_vec2(corner, rotation_deg))
}

fn rect_from_points(points: &[egui::Pos2; 4]) -> egui::Rect {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    egui::Rect::from_min_max(egui::pos2(min_x, min_y), egui::pos2(max_x, max_y))
}

fn paint_rotated_image(
    painter: &egui::Painter,
    texture_id: egui::TextureId,
    center: egui::Pos2,
    size: egui::Vec2,
    rotation_deg: f32,
    tint: egui::Color32,
    flip_x: bool,
    flip_y: bool,
) -> egui::Rect {
    let points = rotated_rect_points(center, size, rotation_deg);
    let (u0, u1) = if flip_x { (1.0, 0.0) } else { (0.0, 1.0) };
    let (v0, v1) = if flip_y { (1.0, 0.0) } else { (0.0, 1.0) };

    let mut mesh = egui::epaint::Mesh::with_texture(texture_id);
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex { pos: points[0], uv: egui::pos2(u0, v0), color: tint });
    mesh.vertices.push(egui::epaint::Vertex { pos: points[1], uv: egui::pos2(u1, v0), color: tint });
    mesh.vertices.push(egui::epaint::Vertex { pos: points[2], uv: egui::pos2(u1, v1), color: tint });
    mesh.vertices.push(egui::epaint::Vertex { pos: points[3], uv: egui::pos2(u0, v1), color: tint });
    mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

    painter.add(egui::Shape::mesh(mesh));
    rect_from_points(&points)
}

fn paint_rotated_placeholder(
    painter: &egui::Painter,
    center: egui::Pos2,
    size: egui::Vec2,
    rotation_deg: f32,
    fill: egui::Color32,
    stroke: egui::Stroke,
) -> egui::Rect {
    let points = rotated_rect_points(center, size, rotation_deg);
    painter.add(egui::Shape::convex_polygon(points.to_vec(), fill, stroke));
    painter.line_segment([points[0], points[2]], stroke);
    painter.line_segment([points[1], points[3]], stroke);
    rect_from_points(&points)
}