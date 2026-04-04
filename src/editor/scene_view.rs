// ============================================================
//  editor/scene_view.rs
//  Viewport da cena 2D — área central do editor.
//  Agora renderiza sprites reais com rotação/escala avançada,
//  aceita drop de assets/MATRs e cria objetos via clique direito.
// ============================================================

use eframe::egui;

use crate::{component::Component, entity::Entity};
use super::{EditorApp, EditorPlayState};

/// Ação disparada por interação do mouse dentro da viewport.
enum SceneInteraction {
    Select { id: String, add_to_selection: bool },
    Drag { id: String, dx: f32, dy: f32 },
    Rotate { id: String, rotation: f32 },
}

enum QuickCreateKind {
    Empty,
    Sprite,
    Camera,
}

pub fn show(app: &mut EditorApp, ui: &mut egui::Ui) {
    let entity_count = count_entities(&app.scene.entities);

    // Cabeçalho da viewport
    ui.horizontal_wrapped(|ui| {
        ui.heading(egui::RichText::new("Cena 2D").strong());
        ui.separator();
        ui.label(format!("📌 {}", app.scene.name));
        ui.separator();
        ui.label(format!("{:.0}%", app.scene_zoom * 100.0));
        ui.separator();
        ui.label(format!("🎯 {} entidades", entity_count));

        if app.play_state != EditorPlayState::Edit {
            ui.separator();
            ui.colored_label(egui::Color32::from_rgb(120, 220, 140), "Runtime em janela separada");
        }

        ui.checkbox(&mut app.show_entity_names, "Nomes");
        ui.checkbox(&mut app.show_colliders, "Colliders");
        ui.checkbox(&mut app.snap_to_grid, "Snap 32px");

        if ui.small_button("🎯 Resetar Visão").clicked() {
            app.scene_zoom = 1.0;
            app.scene_pan = egui::Vec2::ZERO;
        }

        if let Some(selected_id) = &app.selected_entity_id {
            if ui.small_button("📍 Focar Seleção").clicked() {
                if let Some((x, y)) = find_entity_position(&app.scene.entities, selected_id) {
                    app.scene_pan = egui::vec2(-(x * app.scene_zoom), y * app.scene_zoom);
                }
            }
        }
    });

    ui.label(
        "Clique para selecionar | Ctrl + clique = multi-seleção | Arraste entidade = mover | Arraste no vazio = caixa de seleção | Scroll = zoom",
    );
    ui.separator();

    // Área de renderização
    let available = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(available, egui::Sense::click_and_drag());
    let painter = ui.painter_at(available);

    // Zoom com scroll quando o mouse está sobre a viewport
    let scroll_y = ui.ctx().input(|i| {
        if i.raw_scroll_delta.y.abs() > f32::EPSILON {
            i.raw_scroll_delta.y
        } else {
            i.smooth_scroll_delta.y
        }
    });
    if response.hovered() && scroll_y.abs() > f32::EPSILON {
        let zoom_factor = if scroll_y > 0.0 { 1.10 } else { 0.90 };
        app.scene_zoom = (app.scene_zoom * zoom_factor).clamp(0.25, 4.0);
    }

    // Pan da câmera do editor com botão do meio ou direito
    if response.dragged_by(egui::PointerButton::Middle)
        || response.dragged_by(egui::PointerButton::Secondary)
    {
        let delta = ui.ctx().input(|i| i.pointer.delta());
        app.scene_pan += delta;
    }

    // Fundo da cena
    let bg = app.scene.background_color;
    painter.rect_filled(
        available,
        0.0,
        egui::Color32::from_rgb(
            (bg[0] * 255.0) as u8,
            (bg[1] * 255.0) as u8,
            (bg[2] * 255.0) as u8,
        ),
    );

    // Centro da viewport = ponto (0,0) do mundo + deslocamento manual
    let center = available.center() + app.scene_pan;

    // Menu de contexto do viewport: cria entidades no local do mouse
    let mut create_at: Option<QuickCreateKind> = None;
    let pointer_screen_pos = response.hover_pos().or_else(|| response.interact_pointer_pos());
    let pointer_world_pos = pointer_screen_pos.map(|p| screen_to_world(p, center, app.scene_zoom));

    response.context_menu(|ui| {
        ui.label(egui::RichText::new("Criar na Cena").strong());
        ui.separator();
        if ui.button("🔷 Entidade vazia aqui").clicked() {
            create_at = Some(QuickCreateKind::Empty);
            ui.close_menu();
        }
        if ui.button("🖼 Sprite vazio aqui").clicked() {
            create_at = Some(QuickCreateKind::Sprite);
            ui.close_menu();
        }
        if ui.button("📷 Câmera aqui").clicked() {
            create_at = Some(QuickCreateKind::Camera);
            ui.close_menu();
        }
    });

    // ── Grid ──
    draw_grid(&painter, available, center, app.scene_zoom, app.snap_to_grid);

    // ── Eixos X/Y ──
    painter.line_segment(
        [egui::pos2(available.left(), center.y), egui::pos2(available.right(), center.y)],
        egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(255, 80, 80, 120)),
    );
    painter.line_segment(
        [egui::pos2(center.x, available.top()), egui::pos2(center.x, available.bottom())],
        egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(80, 255, 80, 120)),
    );

    // Indicação visual de drag-and-drop vindo do painel Assets
    if let Some(path) = app.dragging_asset_path.clone() {
        if response.hovered() {
            let hint = if is_image_file(&path) {
                "Solte aqui para criar um Sprite"
            } else if is_matr_file(&path) {
                "Solte aqui para instanciar o MATR"
            } else {
                "Solte aqui"
            };

            let overlay_rect = egui::Rect::from_center_size(center, egui::vec2(280.0, 34.0));
            painter.rect_filled(
                overlay_rect,
                6.0,
                egui::Color32::from_rgba_unmultiplied(20, 20, 20, 180),
            );
            painter.rect_stroke(
                overlay_rect,
                6.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 180, 255)),
            );
            painter.text(
                overlay_rect.center(),
                egui::Align2::CENTER_CENTER,
                hint,
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
        }
    }

    // ── Entidades ──
    let selected_id = app.selected_entity_id.clone();
    let entities = app.scene.entities.clone();
    let mut interaction: Option<SceneInteraction> = None;

    for entity in &entities {
        if let Some(result) = draw_entity(app, ui, &painter, entity, center, app.scene_zoom, &selected_id) {
            interaction = Some(result);
        }
    }

    let primary_down = ui.ctx().input(|i| i.pointer.primary_down());

    if interaction.is_none() && app.dragging_asset_path.is_none() && response.hovered() && primary_down {
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            if app.selection_box_start.is_none() {
                app.selection_box_start = Some(pointer_pos);
                app.selection_box_current = Some(pointer_pos);
            } else {
                app.selection_box_current = Some(pointer_pos);
            }
        }
    }

    if let (Some(start), Some(current)) = (app.selection_box_start, app.selection_box_current) {
        let selection_rect = egui::Rect::from_two_pos(start, current);
        if selection_rect.width() > 4.0 || selection_rect.height() > 4.0 {
            painter.rect_filled(
                selection_rect,
                2.0,
                egui::Color32::from_rgba_unmultiplied(120, 180, 255, 30),
            );
            painter.rect_stroke(
                selection_rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 180, 255)),
            );
        }
    }

    let has_interaction = interaction.is_some();

    // Aplicar seleção / arrasto
    match interaction {
        Some(SceneInteraction::Select { id, add_to_selection }) => {
            app.selection_box_start = None;
            app.selection_box_current = None;

            if add_to_selection {
                app.toggle_entity_selection(id);
            } else {
                app.select_single_entity(Some(id));
            }
        }
        Some(SceneInteraction::Drag { id, dx, dy }) => {
            app.selection_box_start = None;
            app.selection_box_current = None;

            if app.active_drag_entity_id.is_none() {
                app.push_undo_state();
                app.active_drag_entity_id = Some(id.clone());
            }

            let snap_to_grid = app.snap_to_grid;
            let target_ids = if app.is_entity_selected(&id) && app.selected_entity_ids.len() > 1 {
                app.selected_entity_ids.clone()
            } else {
                vec![id.clone()]
            };

            if !app.is_entity_selected(&id) {
                app.select_single_entity(Some(id.clone()));
            }

            for target_id in target_ids {
                if let Some(e) = app.find_entity_mut(&target_id) {
                    if let Some(t) = e.transform_mut() {
                        t.x += dx;
                        t.y += dy;

                        if snap_to_grid {
                            t.x = (t.x / 32.0).round() * 32.0;
                            t.y = (t.y / 32.0).round() * 32.0;
                        }
                    }
                }
            }
        }
        Some(SceneInteraction::Rotate { id, rotation }) => {
            app.selection_box_start = None;
            app.selection_box_current = None;

            if app.active_drag_entity_id.is_none() {
                app.push_undo_state();
                app.active_drag_entity_id = Some(id.clone());
            }

            if let Some(e) = app.find_entity_mut(&id) {
                if let Some(t) = e.transform_mut() {
                    t.rotation = rotation;
                }
            }
        }
        None => {
            if response.clicked() {
                app.clear_entity_selection();
                app.selection_box_start = None;
                app.selection_box_current = None;
            }
        }
    }

    if !has_interaction && app.selection_box_start.is_some() && !primary_down {
        if let (Some(start), Some(end)) = (app.selection_box_start.take(), app.selection_box_current.take()) {
            let selection_rect = egui::Rect::from_two_pos(start, end);
            if selection_rect.width() > 6.0 || selection_rect.height() > 6.0 {
                let mut hits = Vec::new();
                collect_entities_in_rect(&app.scene.entities, center, app.scene_zoom, selection_rect, &mut hits);

                let add_to_selection = ui.ctx().input(|i| i.modifiers.command || i.modifiers.ctrl);
                if !add_to_selection {
                    app.clear_entity_selection();
                }

                for id in hits {
                    if !app.is_entity_selected(&id) {
                        app.selected_entity_ids.push(id.clone());
                    }
                    app.selected_entity_id = Some(id);
                }
            }
        }
    }

    // Drop de sprite/MATR vindo do painel Assets
    if let (Some(path), Some((x, y))) = (app.dragging_asset_path.clone(), pointer_world_pos) {
        if response.hovered() && ui.ctx().input(|i| i.pointer.any_released()) {
            if is_image_file(&path) {
                match app.create_sprite_entity_from_asset(&path, Some((x, y))) {
                    Ok(name) => {
                        app.status_msg = format!(
                            "🖼 Sprite '{}' criado via drag-and-drop.",
                            name
                        );
                    }
                    Err(e) => app.status_msg = format!("❌ {}", e),
                }
            } else if is_matr_file(&path) {
                match app.instantiate_matr_from_path(&path, Some((x, y))) {
                    Ok(name) => {
                        app.status_msg = format!(
                            "🧱 MATR '{}' instanciado via drag-and-drop.",
                            name
                        );
                    }
                    Err(e) => app.status_msg = format!("❌ {}", e),
                }
            }
        }
    }

    // Criar entidade via clique direito no ponto do mouse
    if let (Some(kind), Some((x, y))) = (create_at, pointer_world_pos) {
        create_entity_at(app, kind, x, y);
    }

    // Legenda
    let legend_pos = egui::pos2(available.left() + 8.0, available.top() + 8.0);
    painter.text(
        legend_pos,
        egui::Align2::LEFT_TOP,
        "● X",
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(255, 120, 120),
    );
    painter.text(
        egui::pos2(legend_pos.x + 34.0, legend_pos.y),
        egui::Align2::LEFT_TOP,
        "● Y",
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(120, 255, 120),
    );

    if app.snap_to_grid {
        painter.text(
            egui::pos2(available.left() + 8.0, available.top() + 24.0),
            egui::Align2::LEFT_TOP,
            "📏 Snap em grade: 32px ativo",
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(120, 255, 180),
        );
    }
}

/// Renderiza o grid de fundo levando em conta zoom, pan e snap.
fn draw_grid(
    painter: &egui::Painter,
    rect: egui::Rect,
    center: egui::Pos2,
    zoom: f32,
    snap_active: bool,
) {
    let grid_color = if snap_active {
        egui::Color32::from_rgba_unmultiplied(100, 150, 120, 90)
    } else {
        egui::Color32::from_rgba_unmultiplied(80, 80, 90, 60)
    };
    let grid_size = (32.0_f32 * zoom).clamp(8.0, 128.0);

    // Linhas verticais
    let start_x = center.x % grid_size;
    let mut x = rect.left() + (start_x - rect.left()).rem_euclid(grid_size);
    while x <= rect.right() {
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(0.5, grid_color),
        );
        x += grid_size;
    }

    // Linhas horizontais
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

/// Renderiza uma entidade como gizmo/sprite na viewport.
/// Retorna uma ação caso o usuário clique ou arraste o objeto.
fn draw_entity(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    entity: &Entity,
    center: egui::Pos2,
    zoom: f32,
    selected_id: &Option<String>,
) -> Option<SceneInteraction> {
    if !entity.visible {
        return None;
    }

    // Lê os principais componentes da entidade
    let (ex, ey, rotation, scale_x, scale_y) = match entity.components.first() {
        Some(Component::Transform(t)) => (t.x, t.y, t.rotation, t.scale_x, t.scale_y),
        _ => (0.0, 0.0, 0.0, 1.0, 1.0),
    };

    let mut sprite: Option<&crate::core::component::Sprite> = None;
    let mut collider: Option<&crate::core::component::BoxCollider> = None;
    let mut has_camera = false;

    for comp in &entity.components {
        match comp {
            Component::Sprite(s) => sprite = Some(s),
            Component::BoxCollider(bc) => collider = Some(bc),
            Component::Camera2D(_) => has_camera = true,
            _ => {}
        }
    }

    let screen_pos = egui::pos2(center.x + (ex * zoom), center.y - (ey * zoom));
    let is_selected = app.is_entity_selected(&entity.id);
    let is_primary_selected = selected_id.as_deref() == Some(&entity.id);

    let color = if is_selected {
        egui::Color32::from_rgb(255, 220, 50)
    } else {
        egui::Color32::from_rgb(80, 180, 255)
    };

    let size = (12.0_f32 * zoom).clamp(8.0, 22.0);
    let gizmo_rect = egui::Rect::from_center_size(screen_pos, egui::vec2(size * 2.0, size * 2.0));
    let mut interactive_rect = gizmo_rect;

    // Sprite real com rotação/escala ou placeholder visual
    if let Some(sprite_comp) = sprite {
        let tint = egui::Color32::from_rgba_unmultiplied(
            (sprite_comp.color_r * 255.0) as u8,
            (sprite_comp.color_g * 255.0) as u8,
            (sprite_comp.color_b * 255.0) as u8,
            (sprite_comp.color_a * 255.0) as u8,
        );

        let base_size = egui::vec2(
            (64.0 * scale_x.abs().max(0.25) * zoom).clamp(12.0, 512.0),
            (64.0 * scale_y.abs().max(0.25) * zoom).clamp(12.0, 512.0),
        );

        let sprite_rect = if !sprite_comp.texture_path.trim().is_empty() {
            if let Some(texture) = app.load_texture_from_relative_path(ui.ctx(), &sprite_comp.texture_path) {
                let tex_size = texture.size_vec2();
                let draw_size = egui::vec2(
                    (tex_size.x * scale_x.abs().max(0.25) * zoom).clamp(12.0, 512.0),
                    (tex_size.y * scale_y.abs().max(0.25) * zoom).clamp(12.0, 512.0),
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
                )
            } else {
                paint_rotated_placeholder(
                    painter,
                    screen_pos,
                    base_size,
                    rotation,
                    tint.linear_multiply(0.22),
                    egui::Stroke::new(1.0, tint),
                )
            }
        } else {
            paint_rotated_placeholder(
                painter,
                screen_pos,
                base_size,
                rotation,
                tint.linear_multiply(0.22),
                egui::Stroke::new(1.0, tint),
            )
        };

        interactive_rect = interactive_rect.union(sprite_rect);

        if is_selected {
            paint_rotated_outline(
                painter,
                screen_pos,
                sprite_rect.size() + egui::vec2(8.0, 8.0),
                rotation,
                egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(255, 240, 120, 190)),
            );
        }
    }

    // Collider visual
    if app.show_colliders {
        if let Some(collider_comp) = collider {
            let collider_center = egui::pos2(
                screen_pos.x + (collider_comp.offset_x * zoom),
                screen_pos.y - (collider_comp.offset_y * zoom),
            );
            let collider_size = egui::vec2(
                (collider_comp.width * scale_x.abs().max(0.25) * zoom).clamp(8.0, 512.0),
                (collider_comp.height * scale_y.abs().max(0.25) * zoom).clamp(8.0, 512.0),
            );
            let collider_rect = paint_rotated_outline(
                painter,
                collider_center,
                collider_size,
                rotation,
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(120, 255, 180, 180)),
            );
            interactive_rect = interactive_rect.union(collider_rect);
        }
    }

    // Gizmo de câmera
    if has_camera {
        let camera_rect = egui::Rect::from_center_size(
            screen_pos,
            egui::vec2((80.0 * zoom).clamp(20.0, 140.0), (52.0 * zoom).clamp(14.0, 100.0)),
        );
        let cam_color = egui::Color32::from_rgb(180, 120, 255);
        painter.rect_stroke(camera_rect, 4.0, egui::Stroke::new(1.5, cam_color));
        painter.line_segment(
            [camera_rect.left_top(), camera_rect.right_top()],
            egui::Stroke::new(1.5, cam_color),
        );
        painter.text(
            egui::pos2(camera_rect.center().x, camera_rect.top() - 4.0),
            egui::Align2::CENTER_BOTTOM,
            "📷",
            egui::FontId::proportional(12.0),
            cam_color,
        );
        interactive_rect = interactive_rect.union(camera_rect);
    }

    // Desenha o gizmo central
    painter.circle_filled(screen_pos, size * 0.45, color.linear_multiply(0.85));
    painter.circle_stroke(screen_pos, size * 0.55, egui::Stroke::new(1.0, color));

    // Nome / badges da entidade
    if app.show_entity_names {
        painter.text(
            egui::pos2(screen_pos.x, screen_pos.y - size - 10.0),
            egui::Align2::CENTER_BOTTOM,
            &entity.name,
            egui::FontId::proportional(11.0),
            color,
        );
    }

    if is_primary_selected {
        let badges = component_badges(entity);
        if !badges.is_empty() {
            painter.text(
                egui::pos2(screen_pos.x, screen_pos.y + size + 8.0),
                egui::Align2::CENTER_TOP,
                badges,
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(220, 220, 220),
            );
        }

        // Gizmo de rotação visual + interativo
        let handle_distance = interactive_rect.height() * 0.5 + 18.0;
        let handle_pos = screen_pos + rotate_vec2(egui::vec2(0.0, -handle_distance), rotation);
        painter.line_segment(
            [screen_pos, handle_pos],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 210, 120)),
        );
        painter.circle_filled(handle_pos, 6.0, egui::Color32::from_rgb(255, 170, 80));
        painter.circle_stroke(
            handle_pos,
            6.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 245, 200)),
        );

        let rotate_rect = egui::Rect::from_center_size(handle_pos, egui::vec2(16.0, 16.0));
        interactive_rect = interactive_rect.union(rotate_rect);
        let rotate_id = egui::Id::new(("scene_rotate", &entity.id));
        let rotate_response = ui.interact(rotate_rect.expand(3.0), rotate_id, egui::Sense::drag());
        if rotate_response.dragged_by(egui::PointerButton::Primary) {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos - screen_pos;
                let new_rotation = delta.y.atan2(delta.x).to_degrees() + 90.0;
                return Some(SceneInteraction::Rotate {
                    id: entity.id.clone(),
                    rotation: new_rotation,
                });
            }
        }
    }

    // Área interativa para drag / clique
    let id = egui::Id::new(("scene_entity", &entity.id));
    let response = ui.interact(interactive_rect.expand(6.0), id, egui::Sense::click_and_drag());

    if response.clicked() {
        let add_to_selection = ui.ctx().input(|i| i.modifiers.command || i.modifiers.ctrl);
        return Some(SceneInteraction::Select {
            id: entity.id.clone(),
            add_to_selection,
        });
    }

    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = ui.ctx().input(|i| i.pointer.delta());
        return Some(SceneInteraction::Drag {
            id: entity.id.clone(),
            dx: delta.x / zoom.max(0.01),
            dy: -delta.y / zoom.max(0.01),
        });
    }

    // Filhos
    for child in &entity.children {
        if let Some(action) = draw_entity(app, ui, painter, child, center, zoom, selected_id) {
            return Some(action);
        }
    }

    None
}

fn component_badges(entity: &Entity) -> String {
    let mut badges = Vec::new();

    if entity.matr_source.is_some() {
        badges.push("MATR");
    }

    for component in &entity.components {
        match component {
            Component::Sprite(_) => badges.push("Sprite"),
            Component::Camera2D(_) => badges.push("Camera2D"),
            Component::RigidBody2D(_) => badges.push("RigidBody2D"),
            Component::Velocity(_) => badges.push("Velocity"),
            Component::BoxCollider(collider) => badges.push(if collider.is_trigger { "Trigger" } else { "Collider" }),
            Component::Script(_) => badges.push("Script"),
            Component::LuaScript(_) => badges.push("LuaScript"),
            Component::Audio(_) => badges.push("Audio"),
            Component::Animator(_) => badges.push("Animator"),
            Component::TextLabel(_) => badges.push("TextLabel"),
            Component::UIButton(_) => badges.push("UIButton"),
            Component::Transform(_) => {}
        }
    }

    badges.join(" • ")
}

fn count_entities(entities: &[Entity]) -> usize {
    entities.iter().map(|e| 1 + count_entities(&e.children)).sum()
}

fn collect_entities_in_rect(
    entities: &[Entity],
    center: egui::Pos2,
    zoom: f32,
    rect: egui::Rect,
    out: &mut Vec<String>,
) {
    for entity in entities {
        if entity.visible {
            let (x, y) = match entity.components.first() {
                Some(Component::Transform(t)) => (t.x, t.y),
                _ => (0.0, 0.0),
            };

            let screen_pos = egui::pos2(center.x + (x * zoom), center.y - (y * zoom));
            if rect.contains(screen_pos) {
                out.push(entity.id.clone());
            }
        }

        collect_entities_in_rect(&entity.children, center, zoom, rect, out);
    }
}

fn find_entity_position(entities: &[Entity], id: &str) -> Option<(f32, f32)> {
    for entity in entities {
        if entity.id == id {
            return match entity.components.first() {
                Some(Component::Transform(t)) => Some((t.x, t.y)),
                _ => Some((0.0, 0.0)),
            };
        }

        if let Some(found) = find_entity_position(&entity.children, id) {
            return Some(found);
        }
    }

    None
}

fn screen_to_world(screen: egui::Pos2, center: egui::Pos2, zoom: f32) -> (f32, f32) {
    (
        (screen.x - center.x) / zoom.max(0.01),
        (center.y - screen.y) / zoom.max(0.01),
    )
}

fn create_entity_at(app: &mut EditorApp, kind: QuickCreateKind, x: f32, y: f32) {
    app.push_undo_state();

    let mut entity = match kind {
        QuickCreateKind::Empty => Entity::new("Entidade"),
        QuickCreateKind::Sprite => {
            let mut entity = Entity::new("Sprite");
            entity.add_component(Component::Sprite(crate::core::component::Sprite::default()));
            entity
        }
        QuickCreateKind::Camera => {
            let mut entity = Entity::new("Camera");
            entity.add_component(Component::Camera2D(crate::core::component::Camera2D::default()));
            entity
        }
    };

    if let Some(t) = entity.transform_mut() {
        t.x = x;
        t.y = y;
    }

    let id = entity.id.clone();
    let name = entity.name.clone();
    app.scene.add_entity(entity);
    app.select_single_entity(Some(id));
    app.status_msg = format!("✅ '{}' criada em ({:.0}, {:.0}).", name, x, y);
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
    let (sin, cos) = rotation_deg.to_radians().sin_cos();
    let half = size * 0.5;
    let corners = [
        egui::vec2(-half.x, -half.y),
        egui::vec2(half.x, -half.y),
        egui::vec2(half.x, half.y),
        egui::vec2(-half.x, half.y),
    ];

    corners.map(|corner| {
        let rotated = egui::vec2(
            corner.x * cos - corner.y * sin,
            corner.x * sin + corner.y * cos,
        );
        center + rotated
    })
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

fn paint_rotated_outline(
    painter: &egui::Painter,
    center: egui::Pos2,
    size: egui::Vec2,
    rotation_deg: f32,
    stroke: egui::Stroke,
) -> egui::Rect {
    let points = rotated_rect_points(center, size, rotation_deg);
    for index in 0..4 {
        painter.line_segment([points[index], points[(index + 1) % 4]], stroke);
    }
    rect_from_points(&points)
}

fn is_image_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()),
        Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp")
    )
}

fn is_matr_file(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.ends_with(".matr.json") || n.ends_with(".prefab.json"))
        .unwrap_or(false)
}
