use crate::{
    core::{component::Component, entity::Entity},
    runtime::renderer::CameraView,
};

pub fn find_main_camera(entities: &[Entity]) -> CameraView {
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
            return CameraView { x, y, zoom };
        }

        let nested = find_main_camera(&entity.children);
        if nested != CameraView::default() {
            return nested;
        }
    }

    CameraView::default()
}

pub fn move_main_camera(entities: &mut [Entity], dx: f32, dy: f32, zoom_delta: f32) -> bool {
    for entity in entities {
        let has_main_camera = entity
            .components
            .iter()
            .any(|component| matches!(component, Component::Camera2D(cam) if cam.is_main));

        if has_main_camera {
            if let Some(transform) = entity.transform_mut() {
                transform.x += dx;
                transform.y += dy;
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

pub fn set_main_camera_position(entities: &mut [Entity], x: f32, y: f32) -> bool {
    for entity in entities {
        let has_main_camera = entity
            .components
            .iter()
            .any(|component| matches!(component, Component::Camera2D(cam) if cam.is_main));

        if has_main_camera {
            if let Some(transform) = entity.transform_mut() {
                transform.x = x;
                transform.y = y;
            }
            return true;
        }

        if set_main_camera_position(&mut entity.children, x, y) {
            return true;
        }
    }

    false
}


#[derive(Debug, Clone)]
pub struct CameraController {
    pub enabled: bool,
    pub target_entity_id: Option<String>,
    pub follow_speed: f32,
    pub deadzone_w: f32,
    pub deadzone_h: f32,
    pub lookahead_dist: f32,
    pub lookahead_speed: f32,
    pub lookahead_x: f32,
    pub lookahead_y: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub limits: Option<(f32, f32, f32, f32)>,
    pub last_view_w: f32,
    pub last_view_h: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            enabled: false,
            target_entity_id: None,
            follow_speed: 8.0,
            deadzone_w: 0.0,
            deadzone_h: 0.0,
            lookahead_dist: 0.0,
            lookahead_speed: 6.0,
            lookahead_x: 0.0,
            lookahead_y: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            limits: None,
            last_view_w: 0.0,
            last_view_h: 0.0,
        }
    }
}

impl CameraController {
    pub fn set_target(&mut self, entity_id: impl Into<String>, speed: f32) {
        let id = entity_id.into().trim().to_string();
        if id.is_empty() {
            self.camera_off();
            return;
        }
        self.enabled = true;
        self.target_entity_id = Some(id);
        self.follow_speed = speed.max(0.0);
    }

    pub fn set_deadzone(&mut self, w: f32, h: f32) {
        self.deadzone_w = w.max(0.0);
        self.deadzone_h = h.max(0.0);
    }

    pub fn set_lookahead(&mut self, dist: f32, speed: f32) {
        self.lookahead_dist = dist.max(0.0);
        self.lookahead_speed = speed.max(0.0);
    }

    pub fn set_limits(&mut self, left: f32, top: f32, right: f32, bottom: f32) {
        let min_x = left.min(right);
        let max_x = left.max(right);
        let min_y = top.min(bottom);
        let max_y = top.max(bottom);
        self.limits = Some((min_x, min_y, max_x, max_y));
    }

    pub fn set_offset(&mut self, x: f32, y: f32) {
        self.offset_x = x;
        self.offset_y = y;
    }

    pub fn camera_off(&mut self) {
        self.enabled = false;
        self.target_entity_id = None;
        self.lookahead_x = 0.0;
        self.lookahead_y = 0.0;
    }

    pub fn screen_to_world(&self, sx: f32, sy: f32, camera: CameraView) -> (f32, f32) {
        let zoom = camera.zoom.max(0.01);
        let half_w = self.last_view_w * 0.5;
        let half_h = self.last_view_h * 0.5;
        (camera.x + (sx - half_w) / zoom, camera.y + (sy - half_h) / zoom)
    }

    pub fn update(&mut self, entities: &mut [Entity], dt: f32, view_w: f32, view_h: f32) {
        self.last_view_w = view_w.max(0.0);
        self.last_view_h = view_h.max(0.0);
        if !self.enabled || dt <= 0.0 {
            return;
        }

        let Some(target_id) = self.target_entity_id.as_deref() else {
            return;
        };

        let Some((target_x, target_y, target_vx, target_vy)) = find_entity_motion(entities, target_id) else {
            return;
        };

        let camera = find_main_camera(entities);
        let zoom = camera.zoom.max(0.01);
        let mut desired_x = camera.x;
        let mut desired_y = camera.y;

        let target_lookahead_x = target_vx.signum() * self.lookahead_dist;
        let target_lookahead_y = target_vy.signum() * self.lookahead_dist;
        let lookahead_t = smoothing_alpha(self.lookahead_speed, dt);
        self.lookahead_x = lerp(self.lookahead_x, target_lookahead_x, lookahead_t);
        self.lookahead_y = lerp(self.lookahead_y, target_lookahead_y, lookahead_t);

        let focus_x = target_x + self.offset_x + self.lookahead_x;
        let focus_y = target_y + self.offset_y + self.lookahead_y;

        let half_dead_w = self.deadzone_w.max(0.0) * 0.5 / zoom;
        let half_dead_h = self.deadzone_h.max(0.0) * 0.5 / zoom;

        if half_dead_w <= f32::EPSILON {
            desired_x = focus_x;
        } else if focus_x < camera.x - half_dead_w {
            desired_x = focus_x + half_dead_w;
        } else if focus_x > camera.x + half_dead_w {
            desired_x = focus_x - half_dead_w;
        }

        if half_dead_h <= f32::EPSILON {
            desired_y = focus_y;
        } else if focus_y < camera.y - half_dead_h {
            desired_y = focus_y + half_dead_h;
        } else if focus_y > camera.y + half_dead_h {
            desired_y = focus_y - half_dead_h;
        }

        let follow_t = smoothing_alpha(self.follow_speed, dt);
        let mut next_x = lerp(camera.x, desired_x, follow_t);
        let mut next_y = lerp(camera.y, desired_y, follow_t);

        if let Some((min_x, min_y, max_x, max_y)) = self.limits {
            let half_view_w = self.last_view_w * 0.5 / zoom;
            let half_view_h = self.last_view_h * 0.5 / zoom;
            let min_center_x = min_x + half_view_w;
            let max_center_x = max_x - half_view_w;
            let min_center_y = min_y + half_view_h;
            let max_center_y = max_y - half_view_h;
            next_x = clamp_camera_axis(next_x, min_center_x, max_center_x);
            next_y = clamp_camera_axis(next_y, min_center_y, max_center_y);
        }

        set_main_camera_position(entities, next_x, next_y);
    }
}

fn find_entity_motion(entities: &[Entity], target_id: &str) -> Option<(f32, f32, f32, f32)> {
    for entity in entities {
        if entity.id == target_id {
            let transform = entity.transform()?;
            let (vx, vy) = entity.velocity().map(|v| (v.x, v.y)).unwrap_or((0.0, 0.0));
            return Some((transform.x, transform.y, vx, vy));
        }
        if let Some(found) = find_entity_motion(&entity.children, target_id) {
            return Some(found);
        }
    }
    None
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[inline]
fn smoothing_alpha(speed: f32, dt: f32) -> f32 {
    if speed <= 0.0 {
        1.0
    } else {
        1.0 - (-speed * dt.max(0.0)).exp()
    }
}

#[inline]
fn clamp_camera_axis(value: f32, min: f32, max: f32) -> f32 {
    if min <= max {
        value.clamp(min, max)
    } else {
        (min + max) * 0.5
    }
}
