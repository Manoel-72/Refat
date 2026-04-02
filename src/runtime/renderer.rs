use eframe::egui;

use crate::{
    editor::EditorApp,
    core::{
        component::{Component, Sprite},
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

    let screen_pos = egui::pos2(
        center.x + ((ex - camera.x) * camera.zoom),
        center.y - ((ey - camera.y) * camera.zoom),
    );

    let mut sprite: Option<&Sprite> = None;
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
        painter.circle_filled(screen_pos, 5.0, egui::Color32::from_rgb(120, 190, 255));
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
