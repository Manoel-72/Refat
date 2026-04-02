use crate::{core::{component::Component, entity::Entity}, runtime::renderer::CameraView};

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
