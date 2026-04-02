use crate::core::entity::Entity;

pub fn apply_script_movement(entity: &mut Entity, delta_time: f32, move_x: f32, move_y: f32, rotate_speed: f32) {
    if let Some(transform) = entity.transform_mut() {
        transform.x += move_x * delta_time;
        transform.y += move_y * delta_time;
        transform.rotation += rotate_speed * delta_time;
    }
}

pub fn apply_player_controller(entity: &mut Entity, delta_time: f32, input: eframe::egui::Vec2, speed: f32) {
    if let Some(transform) = entity.transform_mut() {
        transform.x += input.x * speed * delta_time;
        transform.y += input.y * speed * delta_time;
    }
}
