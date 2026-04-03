use crate::core::entity::Entity;

pub fn apply_script_movement(
    entity: &mut Entity,
    delta_time: f32,
    move_x: f32,
    move_y: f32,
    rotate_speed: f32,
) {
    if let Some(velocity) = entity.velocity_mut() {
        velocity.x += move_x;
        velocity.y += move_y;
    } else if let Some(transform) = entity.transform_mut() {
        transform.x += move_x * delta_time;
        transform.y += move_y * delta_time;
    }

    if let Some(transform) = entity.transform_mut() {
        transform.rotation += rotate_speed * delta_time;
    }
}

pub fn apply_player_controller(entity: &mut Entity, input: eframe::egui::Vec2, speed: f32) {
    if let Some(velocity) = entity.velocity_mut() {
        velocity.x = input.x * speed;
        velocity.y = input.y * speed;
    } else if let Some(transform) = entity.transform_mut() {
        transform.x += input.x * speed;
        transform.y += input.y * speed;
    }
}

pub fn apply_velocity(entity: &mut Entity, delta_time: f32) {
    let Some((vx, vy)) = entity.velocity().map(|v| (v.x, v.y)) else {
        return;
    };

    if let Some(transform) = entity.transform_mut() {
        transform.x += vx * delta_time;
        transform.y += vy * delta_time;
    }
}
