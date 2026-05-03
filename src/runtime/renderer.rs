use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use eframe::egui;

use crate::core::{
    component::{Component, Sprite, TextLabel, UIButton},
    entity::Entity,
};
use crate::renderer::gfx::paint_rotated_placeholder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    ChangeScene(PathBuf),
    CloseRuntime,
}

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
    let center = egui::pos2(
        center.x - camera.x * camera.zoom,
        center.y + camera.y * camera.zoom,
    );

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
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    project_root: &Path,
    current_scene_path: Option<&Path>,
    sprite_textures: &mut HashMap<String, egui::TextureHandle>,
    entity: &Entity,
    center: egui::Pos2,
    camera: CameraView,
) -> Option<UiAction> {
    if !entity.visible {
        return None;
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

    let mut action = None;
    if let Some(button) = ui_button {
        action = draw_ui_button(
            ui,
            painter,
            project_root,
            current_scene_path,
            world_screen_pos,
            screen_space_pos,
            entity,
            button,
        );
    }

    // Runtime final: não desenha colliders por padrão para evitar ruído visual
    // e custo extra de renderização em tempo de jogo.

    if let Some(sprite_comp) = sprite {
        let sprite_pos = if sprite_comp.screen_space {
            screen_space_pos
        } else {
            world_screen_pos
        };
        let tint = egui::Color32::from_rgba_unmultiplied(
            (sprite_comp.color_r.clamp(0.0, 1.0) * 255.0) as u8,
            (sprite_comp.color_g.clamp(0.0, 1.0) * 255.0) as u8,
            (sprite_comp.color_b.clamp(0.0, 1.0) * 255.0) as u8,
            (sprite_comp.color_a.clamp(0.0, 1.0) * 255.0) as u8,
        );

        if !sprite_comp.texture_path.trim().is_empty() {
            if let Some(texture) = load_texture_from_relative_path(
                ui.ctx(),
                project_root,
                sprite_textures,
                &sprite_comp.texture_path,
            ) {
                let tex_size = texture.size_vec2();
                let draw_size = egui::vec2(
                    (tex_size.x * scale_x.abs().max(0.25) * camera.zoom).clamp(8.0, 512.0),
                    (tex_size.y * scale_y.abs().max(0.25) * camera.zoom).clamp(8.0, 512.0),
                );
                paint_rotated_image(
                    painter,
                    texture.id(),
                    sprite_pos,
                    draw_size,
                    rotation,
                    tint,
                    scale_x < 0.0,
                    scale_y < 0.0,
                );
            }
        } else {
            // Sem textura: desenha retângulo colorido usando a cor do Sprite
            // Tamanho baseado no BoxCollider se existir, senão 32×32
            let fallback_w = entity
                .components
                .iter()
                .find_map(|c| {
                    if let crate::core::component::Component::BoxCollider(b) = c {
                        Some(b.width)
                    } else {
                        None
                    }
                })
                .unwrap_or(32.0);
            let fallback_h = entity
                .components
                .iter()
                .find_map(|c| {
                    if let crate::core::component::Component::BoxCollider(b) = c {
                        Some(b.height)
                    } else {
                        None
                    }
                })
                .unwrap_or(32.0);
            let draw_size = egui::vec2(
                fallback_w * scale_x.abs().max(0.25) * camera.zoom,
                fallback_h * scale_y.abs().max(0.25) * camera.zoom,
            );
            paint_rotated_placeholder(
                painter,
                sprite_pos,
                draw_size,
                rotation,
                tint,
                egui::Stroke::NONE,
            );
        }
    } else if has_camera {
        // Runtime final: não desenha contorno de câmera.
    } else {
        // Runtime final: entidades sem visual próprio não desenham marcador auxiliar.
    }

    for child in &entity.children {
        if let Some(child_action) = draw_runtime_entity(
            ui,
            painter,
            project_root,
            current_scene_path,
            sprite_textures,
            child,
            center,
            camera,
        ) {
            action = Some(child_action);
        }
    }

    action
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
            if matches!(component, Component::Script(_) | Component::LuaScript(_)) {
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
    mesh.vertices.push(egui::epaint::Vertex {
        pos: points[0],
        uv: egui::pos2(u0, v0),
        color: tint,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: points[1],
        uv: egui::pos2(u1, v0),
        color: tint,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: points[2],
        uv: egui::pos2(u1, v1),
        color: tint,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: points[3],
        uv: egui::pos2(u0, v1),
        color: tint,
    });
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    painter.add(egui::Shape::mesh(mesh));
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

/// Chave estável para cache de texturas (evita entradas duplicadas por espaços ou barras).
fn texture_cache_key(relative_path: &str) -> String {
    relative_path.trim().replace('\\', "/")
}

fn load_texture_from_relative_path(
    ctx: &egui::Context,
    project_root: &Path,
    sprite_textures: &mut HashMap<String, egui::TextureHandle>,
    relative_path: &str,
) -> Option<egui::TextureHandle> {
    let key = texture_cache_key(relative_path);

    if let Some(texture) = sprite_textures.get(&key) {
        return Some(texture.clone());
    }

    #[cfg(windows)]
    {
        for (k, tex) in sprite_textures.iter() {
            if k.eq_ignore_ascii_case(&key) {
                return Some(tex.clone());
            }
        }
    }

    let candidates = [
        project_root.join("assets").join(&key),
        project_root.join(&key),
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

    sprite_textures.insert(key, texture.clone());
    Some(texture)
}

fn resolve_scene_path(
    project_root: &Path,
    current_scene_path: Option<&Path>,
    target: &str,
) -> Result<PathBuf, String> {
    let raw = target.trim();
    if raw.is_empty() {
        return Err("UIButton sem target_scene configurado".to_string());
    }

    let normalized = raw.replace('\\', "/");
    let target_path = Path::new(&normalized);

    let mut candidates: Vec<PathBuf> = Vec::new();
    if target_path.is_absolute() {
        candidates.push(target_path.to_path_buf());
    } else {
        candidates.push(project_root.join(&normalized));

        if let Some(current_path) = current_scene_path {
            if let Some(current_dir) = current_path.parent() {
                candidates.push(current_dir.join(&normalized));
            }
        }

        if !normalized.starts_with("assets/") {
            candidates.push(project_root.join("assets/scenes").join(&normalized));
        }

        if !normalized.ends_with(".scene.json") {
            candidates.push(project_root.join(format!("{}.scene.json", normalized)));
            candidates.push(
                project_root
                    .join("assets/scenes")
                    .join(format!("{}.scene.json", normalized)),
            );
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
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    project_root: &Path,
    current_scene_path: Option<&Path>,
    world_screen_pos: egui::Pos2,
    screen_space_pos: egui::Pos2,
    entity: &Entity,
    button: &UIButton,
) -> Option<UiAction> {
    let pos = if button.screen_space {
        screen_space_pos
    } else {
        world_screen_pos
    };

    let size = egui::vec2(button.width.max(72.0), button.height.max(28.0));
    let rect = egui::Rect::from_center_size(pos, size);
    let id = egui::Id::new(format!("runtime_ui_button_{}", entity.id));
    let response = ui.interact(rect, id, egui::Sense::click());

    if response.hovered() {
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
    }

    let hover_t = ui.ctx().animate_bool(id.with("hover"), response.hovered());
    let press_t = ui
        .ctx()
        .animate_bool(id.with("press"), response.is_pointer_button_down_on());

    let to_u8 = |value: f32| -> u8 { (value.clamp(0.0, 1.0) * 255.0).round() as u8 };
    let brighten = |value: u8, amount: u8| -> u8 { value.saturating_add(amount) };
    let darken = |value: u8, amount: u8| -> u8 { value.saturating_sub(amount) };
    let lerp_u8 = |from: u8, to: u8, t: f32| -> u8 {
        ((from as f32) + ((to as f32) - (from as f32)) * t.clamp(0.0, 1.0)).round() as u8
    };

    let base_r = to_u8(button.color_r);
    let base_g = to_u8(button.color_g);
    let base_b = to_u8(button.color_b);
    let base_a = to_u8(button.color_a.max(0.85));

    let hover_r = brighten(base_r, 20);
    let hover_g = brighten(base_g, 20);
    let hover_b = brighten(base_b, 20);
    let press_r = darken(base_r, 18);
    let press_g = darken(base_g, 18);
    let press_b = darken(base_b, 18);

    let mut fill_r = lerp_u8(base_r, hover_r, hover_t);
    let mut fill_g = lerp_u8(base_g, hover_g, hover_t);
    let mut fill_b = lerp_u8(base_b, hover_b, hover_t);
    fill_r = lerp_u8(fill_r, press_r, press_t);
    fill_g = lerp_u8(fill_g, press_g, press_t);
    fill_b = lerp_u8(fill_b, press_b, press_t);

    let fill = egui::Color32::from_rgba_unmultiplied(fill_r, fill_g, fill_b, base_a);
    let border = egui::Color32::from_rgba_unmultiplied(
        brighten(fill_r, 26),
        brighten(fill_g, 26),
        brighten(fill_b, 26),
        base_a,
    );
    let top_highlight = egui::Color32::from_rgba_unmultiplied(
        brighten(fill_r, 38),
        brighten(fill_g, 38),
        brighten(fill_b, 38),
        darken(base_a, 18),
    );
    let shadow_alpha = lerp_u8(72, 112, hover_t.max(press_t * 0.5));
    let shadow = egui::Color32::from_rgba_unmultiplied(0, 0, 0, shadow_alpha);
    let text_color = egui::Color32::from_rgba_unmultiplied(
        to_u8(button.text_r),
        to_u8(button.text_g),
        to_u8(button.text_b),
        to_u8(button.text_a),
    );

    let visual_offset_y = 5.0 - (hover_t * 1.0) - (press_t * 3.0);
    let visual_rect = rect.translate(egui::vec2(0.0, -press_t * 2.0));
    let shadow_rect = rect.translate(egui::vec2(0.0, visual_offset_y));
    let gloss_rect = egui::Rect::from_min_max(
        egui::pos2(visual_rect.left() + 6.0, visual_rect.top() + 6.0),
        egui::pos2(
            visual_rect.right() - 6.0,
            visual_rect.top() + visual_rect.height() * (0.40 - press_t * 0.08),
        ),
    );

    painter.rect_filled(shadow_rect, 16.0, shadow);
    painter.rect_filled(visual_rect, 16.0, fill);
    painter.rect_stroke(
        visual_rect,
        16.0,
        egui::Stroke::new(1.5 + hover_t * 0.5, border),
    );
    painter.rect_filled(
        gloss_rect,
        12.0,
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, lerp_u8(12, 28, hover_t)),
    );
    painter.line_segment(
        [
            egui::pos2(visual_rect.left() + 10.0, visual_rect.top() + 7.0),
            egui::pos2(visual_rect.right() - 10.0, visual_rect.top() + 7.0),
        ],
        egui::Stroke::new(1.0, top_highlight),
    );

    let accent_rect = egui::Rect::from_min_max(
        egui::pos2(visual_rect.left() + 8.0, visual_rect.bottom() - 8.0),
        egui::pos2(visual_rect.right() - 8.0, visual_rect.bottom() - 5.0),
    );
    painter.rect_filled(
        accent_rect,
        2.0,
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, lerp_u8(18, 30, hover_t)),
    );

    painter.text(
        visual_rect.center() + egui::vec2(0.0, -press_t),
        egui::Align2::CENTER_CENTER,
        &button.text,
        egui::FontId::proportional(button.font_size.max(12.0)),
        text_color,
    );

    if response.clicked() {
        if !button.target_scene.trim().is_empty() {
            match resolve_scene_path(project_root, current_scene_path, &button.target_scene) {
                Ok(scene_path) => return Some(UiAction::ChangeScene(scene_path)),
                Err(_) => return None,
            }
        }

        if button.close_runtime {
            return Some(UiAction::CloseRuntime);
        }
    }

    None
}

/// Fade / flash em tela cheia **depois** de partículas e entidades.
/// `paint_egui` cobre o host eframe; `ScreenFx::draw` usa macroquad (`screen_width` / `screen_height`).
pub fn draw_runtime_screen_fx_after_scene(
    fx: &crate::effects::screen_fx::ScreenFx,
    painter: &egui::Painter,
    rect: egui::Rect,
) {
    fx.paint_egui(painter, rect);
    fx.draw();
}

pub fn draw_runtime_particles(
    painter: &egui::Painter,
    center: egui::Pos2,
    camera: CameraView,
    particles: &[crate::runtime::state::RuntimeParticle],
) {
    for particle in particles {
        let t = if particle.max_life <= f32::EPSILON {
            0.0
        } else {
            (particle.life / particle.max_life).clamp(0.0, 1.0)
        };
        let pos = egui::pos2(
            center.x + ((particle.x - camera.x) * camera.zoom),
            center.y - ((particle.y - camera.y) * camera.zoom),
        );
        let radius = (particle.scale.max(0.25) * 6.0 * camera.zoom.max(0.5)).clamp(2.0, 24.0);
        let color = egui::Color32::from_rgba_unmultiplied(
            (particle.color[0].clamp(0.0, 1.0) * 255.0) as u8,
            (particle.color[1].clamp(0.0, 1.0) * 255.0) as u8,
            (particle.color[2].clamp(0.0, 1.0) * 255.0) as u8,
            ((particle.color[3].clamp(0.0, 1.0) * t) * 255.0) as u8,
        );
        painter.circle_filled(pos, radius, color);
    }
}
