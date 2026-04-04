use crate::core::{component::Component, entity::Entity};

pub fn apply_gravity(entity: &mut Entity, delta_time: f32, gravity_scale: Option<f32>) {
    let Some(gravity) = gravity_scale else {
        return;
    };

    if let Some(velocity) = entity.velocity_mut() {
        velocity.y -= 180.0 * gravity.max(0.0) * delta_time;
        set_grounded(entity, false);
        return;
    }

    if let Some(transform) = entity.transform_mut() {
        transform.y -= 180.0 * gravity.max(0.0) * delta_time;
    }
}

pub fn clamp_to_ground(entity: &mut Entity, ground_y: f32, collider_half_height: f32) {
    let floor_y = ground_y + collider_half_height.max(16.0);
    let Some(current_y) = entity.transform().map(|t| t.y) else {
        return;
    };

    if current_y >= floor_y {
        return;
    }

    if let Some(transform) = entity.transform_mut() {
        transform.y = floor_y;
    }

    if let Some(velocity) = entity.velocity_mut() {
        if velocity.y < 0.0 {
            velocity.y = 0.0;
        }
    }

    set_grounded(entity, true);
}

pub fn zero_velocity(entity: &mut Entity) {
    if let Some(velocity) = entity.velocity_mut() {
        velocity.x = 0.0;
        velocity.y = 0.0;
    }
}

pub fn set_grounded(entity: &mut Entity, grounded: bool) {
    for component in &mut entity.components {
        if let Component::RigidBody2D(rb) = component {
            rb.grounded = grounded;
        }
    }
}
