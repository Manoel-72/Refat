use eframe::egui;

use crate::{
    editor::EditorApp,
    core::{
        component::{Component, Sprite, TextLabel, UIButton},
        entity::Entity,
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraView {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Default for CameraView {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

pub fn draw_runtime_grid(
    painter: &egui::Painter,
    rect: egui::Rect,
    center: egui::Pos2,
    camera: CameraView,
) {
    let grid_color = egui::Color32::from_rgba_unmultiplied(90, 90, 100, 60);
    let grid_size = (32.0_f32 * camera.zoom).clamp(8.0, 128.0);
    let center = egui::pos2(center.x - camera.x * camera.zoom, center.y + camera.y * camera.zoom);

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

pub fn draw_runtime_entity(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    entity: &Entity,
    center: egui::Pos2,
    camera: CameraView,
) {
    if !entity.visible {
        return;
    }

    let (ex, ey, rotation, scale_x, scale_y) = entity
        .transform()
        .map(|t| (t.x, t.y, t.rotation, t.scale_x, t.scale_y))
        .unwrap_or((0.0, 0.0, 0.0, 1.0, 1.0));

    let world_screen_pos = egui::pos2(
        center.x + ((ex - camera.x) * camera.zoom),
        center.y - ((ey - camera.y) * camera.zoom),
    );
    let screen_space_pos = egui::pos2(center.x + ex, center.y - ey);

    let mut sprite: Option<&Sprite> = None;
    let mut text_label: Option<&TextLabel> = None;
    let mut ui_button: Option<&UIButton> = None;
    let mut has_camera = false;

    for component in &entity.components {
        match component {
            Component::Sprite(s) => sprite = Some(s),
            Component::TextLabel(label) => text_label = Some(label),
            Component::UIButton(button) => ui_button = Some(button),
            Component::Camera2D(_) => has_camera = true,
            _ => {}
        }
    }

    if let Some(label) = text_label {
        draw_text_label(painter, world_screen_pos, screen_space_pos, label);
    }

    if let Some(button) = ui_button {
        draw_ui_button(app, ui, painter, world_screen_pos, screen_space_pos, entity, button);
    }

    if let Some(sprite_comp) = sprite {
        let tint = egui::Color32::from_rgba_unmultiplied(
            (sprite_comp.color_r * 255.0) as u8,
            (sprite_comp.color_g * 255.0) as u8,
            (sprite_comp.color_b * 255.0) as u8,
            (sprite_comp.color_a * 255.0) as u8,
        );

        let default_size = egui::vec2(
            (64.0 * scale_x.abs().max(0.25) * camera.zoom).clamp(8.0, 512.0),
            (64.0 * scale_y.abs().max(0.25) * camera.zoom).clamp(8.0, 512.0),
        );

        if !sprite_comp.texture_path.trim().is_empty() {
            if let Some(texture) = app.load_texture_from_relative_path(ui.ctx(), &sprite_comp.texture_path) {
                let tex_size = texture.size_vec2();
                let draw_size = egui::vec2(
                    (tex_size.x * scale_x.abs().max(0.25) * camera.zoom).clamp(8.0, 512.0),
                    (tex_size.y * scale_y.abs().max(0.25) * camera.zoom).clamp(8.0, 512.0),
                );
                paint_rotated_image(
                    painter,
                    texture.id(),
                    world_screen_pos,
                    draw_size,
                    rotation,
                    tint,
                    scale_x < 0.0,
                    scale_y < 0.0,
                );
            } else {
                paint_rotated_placeholder(
                    painter,
                    world_screen_pos,
                    default_size,
                    rotation,
                    tint.linear_multiply(0.25),
                    egui::Stroke::new(1.0, tint),
                );
            }
        } else {
            paint_rotated_placeholder(
                painter,
                world_screen_pos,
                default_size,
                rotation,
                tint.linear_multiply(0.25),
                egui::Stroke::new(1.0, tint),
            );
        }
    } else if has_camera {
        let rect = egui::Rect::from_center_size(world_screen_pos, egui::vec2(80.0, 52.0));
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.5, egui::Color32::from_rgb(180, 120, 255)),
        );
    } else {
        painter.circle_filled(world_screen_pos, 5.0, egui::Color32::from_rgb(120, 190, 255));
    }

    for child in &entity.children {
        draw_runtime_entity(app, ui, painter, child, center, camera);
    }
}

pub fn count_entities(entities: &[Entity]) -> usize {
    entities
        .iter()
        .map(|entity| 1 + count_entities(&entity.children))
        .sum()
}

pub fn count_scripts(entities: &[Entity]) -> usize {
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

fn rotate_vec2(vec: egui::Vec2, rotation_deg: f32) -> egui::Vec2 {
    let (sin, cos) = rotation_deg.to_radians().sin_cos();
    egui::vec2(vec.x * cos - vec.y * sin, vec.x * sin + vec.y * cos)
}

fn rotated_rect_points(center: egui::Pos2, size: egui::Vec2, rotation_deg: f32) -> [egui::Pos2; 4] {
    let half = size * 0.5;
    let corners = [
        egui::vec2(-half.x, -half.y),
        egui::vec2(half.x, -half.y),
        egui::vec2(half.x, half.y),
        egui::vec2(-half.x, half.y),
    ];

    corners.map(|corner| center + rotate_vec2(corner, rotation_deg))
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
) {
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
}

fn paint_rotated_placeholder(
    painter: &egui::Painter,
    center: egui::Pos2,
    size: egui::Vec2,
    rotation_deg: f32,
    fill: egui::Color32,
    stroke: egui::Stroke,
) {
    let points = rotated_rect_points(center, size, rotation_deg);
    painter.add(egui::Shape::convex_polygon(points.to_vec(), fill, stroke));
}


fn draw_text_label(
    painter: &egui::Painter,
    world_screen_pos: egui::Pos2,
    screen_space_pos: egui::Pos2,
    label: &TextLabel,
) {
    let pos = if label.screen_space {
        screen_space_pos
    } else {
        world_screen_pos
    };

    let color = egui::Color32::from_rgba_unmultiplied(
        (label.color_r.clamp(0.0, 1.0) * 255.0) as u8,
        (label.color_g.clamp(0.0, 1.0) * 255.0) as u8,
        (label.color_b.clamp(0.0, 1.0) * 255.0) as u8,
        (label.color_a.clamp(0.0, 1.0) * 255.0) as u8,
    );

    painter.text(
        pos,
        egui::Align2::CENTER_CENTER,
        &label.text,
        egui::FontId::proportional(label.font_size.max(8.0)),
        color,
    );
}

fn resolve_scene_target_path(app: &EditorApp, target_scene: &str) -> Result<std::path::PathBuf, String> {
    use std::path::{Path, PathBuf};

    let raw = target_scene.trim();
    if raw.is_empty() {
        return Err("UIButton sem target_scene configurado".to_string());
    }

    let normalized = raw.replace('\\', "/");
    let target_path = Path::new(&normalized);

    let mut candidates: Vec<PathBuf> = Vec::new();
    if target_path.is_absolute() {
        candidates.push(target_path.to_path_buf());
    } else {
        candidates.push(app.project_root.join(&normalized));

        if let Some(current_path) = app.runtime.scene_manager.current_path.as_ref() {
            if let Some(current_dir) = current_path.parent() {
                candidates.push(current_dir.join(&normalized));
            }
        }

        if !normalized.starts_with("assets/") {
            candidates.push(app.project_root.join("assets/scenes").join(&normalized));
        }

        let has_scene_suffix = normalized.ends_with(".scene.json");
        if !has_scene_suffix {
            candidates.push(app.project_root.join(format!("{}.scene.json", normalized)));
            candidates.push(app.project_root.join("assets/scenes").join(format!("{}.scene.json", normalized)));
        }
    }

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(format!("Cena alvo não encontrada: {}", normalized))
}

fn draw_ui_button(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    world_screen_pos: egui::Pos2,
    screen_space_pos: egui::Pos2,
    entity: &Entity,
    button: &UIButton,
) {
    let pos = if button.screen_space {
        screen_space_pos
    } else {
        world_screen_pos
    };

    let size = egui::vec2(button.width.max(72.0), button.height.max(28.0));
    let rect = egui::Rect::from_center_size(pos, size);
    let response = ui.interact(
        rect,
        egui::Id::new(format!("runtime_ui_button_{}", entity.id)),
        egui::Sense::click(),
    );

    if response.hovered() {
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
    }

    let to_u8 = |value: f32| -> u8 { (value.clamp(0.0, 1.0) * 255.0).round() as u8 };
    let brighten = |value: u8, amount: u8| -> u8 { value.saturating_add(amount) };
    let darken = |value: u8, amount: u8| -> u8 { value.saturating_sub(amount) };

    let base_r = to_u8(button.color_r);
    let base_g = to_u8(button.color_g);
    let base_b = to_u8(button.color_b);
    let base_a = to_u8(button.color_a.max(0.85));

    let is_pressed = response.is_pointer_button_down_on();
    let fill = if is_pressed {
        egui::Color32::from_rgba_unmultiplied(darken(base_r, 20), darken(base_g, 20), darken(base_b, 20), base_a)
    } else if response.hovered() {
        egui::Color32::from_rgba_unmultiplied(brighten(base_r, 16), brighten(base_g, 16), brighten(base_b, 16), base_a)
    } else {
        egui::Color32::from_rgba_unmultiplied(base_r, base_g, base_b, base_a)
    };
    let border = egui::Color32::from_rgba_unmultiplied(
        brighten(base_r, 28),
        brighten(base_g, 28),
        brighten(base_b, 28),
        base_a,
    );
    let top_highlight = egui::Color32::from_rgba_unmultiplied(
        brighten(base_r, 44),
        brighten(base_g, 44),
        brighten(base_b, 44),
        darken(base_a, 20),
    );
    let shadow = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 72);
    let text_color = egui::Color32::from_rgba_unmultiplied(
        to_u8(button.text_r),
        to_u8(button.text_g),
        to_u8(button.text_b),
        to_u8(button.text_a),
    );

    let shadow_rect = rect.translate(egui::vec2(0.0, 5.0));
    painter.rect_filled(shadow_rect, 14.0, shadow);
    painter.rect_filled(rect, 14.0, fill);
    painter.rect_stroke(rect, 14.0, egui::Stroke::new(1.5, border));
    painter.line_segment(
        [
            egui::pos2(rect.left() + 10.0, rect.top() + 7.0),
            egui::pos2(rect.right() - 10.0, rect.top() + 7.0),
        ],
        egui::Stroke::new(1.0, top_highlight),
    );

    let accent_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 8.0, rect.bottom() - 8.0),
        egui::pos2(rect.right() - 8.0, rect.bottom() - 5.0),
    );
    painter.rect_filled(accent_rect, 2.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 22));

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        &button.text,
        egui::FontId::proportional(button.font_size.max(12.0)),
        text_color,
    );

    if response.clicked() {
        let mut action_messages = Vec::new();

        if !button.target_scene.trim().is_empty() {
            match resolve_scene_target_path(app, &button.target_scene) {
                Ok(scene_path) => {
                    app.runtime.queue_scene_change(scene_path.clone());
                    let label = scene_path.file_name().and_then(|n| n.to_str()).unwrap_or("cena");
                    action_messages.push(format!("troca de cena -> {}", label));
                }
                Err(error) => {
                    app.status_msg = format!("Erro no UIButton '{}': {}", button.text, error);
                }
            }
        }

        if button.close_runtime {
            app.play_state = crate::editor::EditorPlayState::Edit;
            app.runtime.stop();
            app.runtime.window_open = false;
            action_messages.push("fechar runtime".to_string());
        }

        if !action_messages.is_empty() {
            app.status_msg = format!(
                "UIButton '{}' acionado com sucesso: {}",
                button.text,
                action_messages.join(" | ")
            );
        }
    }
}
