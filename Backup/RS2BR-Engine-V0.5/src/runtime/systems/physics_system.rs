use crate::core::entity::Entity;

pub fn apply_gravity(entity: &mut Entity, delta_time: f32, gravity_scale: Option<f32>) {
    if let (Some(transform), Some(gravity)) = (entity.transform_mut(), gravity_scale) {
        transform.y -= 180.0 * gravity.max(0.0) * delta_time;
    }
}

pub fn clamp_to_ground(entity: &mut Entity, ground_y: f32, collider_half_height: f32) {
    if let Some(transform) = entity.transform_mut() {
        let floor_y = ground_y + collider_half_height.max(16.0);
        if transform.y < floor_y {
            transform.y = floor_y;
        }
    }
}
