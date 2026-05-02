use crate::core::{component::Component, entity::Entity};

/// Velocidade máxima de queda (pixels/s). Evita tunelamento e "slide infinito".
#[allow(dead_code)]
pub const MAX_FALL_SPEED: f32 = 800.0;

pub fn apply_gravity(entity: &mut Entity, delta_time: f32, gravity_scale: Option<f32>) {
    let Some(gravity) = gravity_scale else {
        return;
    };

    if let Some(velocity) = entity.velocity_mut() {
        velocity.y -= 540.0 * gravity.max(0.0) * delta_time;
        // Terminal velocity — impede queda infinita / tunelamento
        if velocity.y < -MAX_FALL_SPEED {
            velocity.y = -MAX_FALL_SPEED;
        }
        set_grounded(entity, false);
        return;
    }

    if let Some(transform) = entity.transform_mut() {
        transform.y -= 540.0 * gravity.max(0.0) * delta_time;
    }
}

/// Aplica impulso de pulo se a entidade estiver no chão.
/// Retorna true se o pulo foi aplicado.
#[allow(dead_code)]
pub fn try_jump(entity: &mut Entity, impulse: f32) -> bool {
    let grounded = entity
        .components
        .iter()
        .find_map(|c| {
            if let Component::RigidBody2D(rb) = c {
                Some(rb.grounded)
            } else {
                None
            }
        })
        .unwrap_or(false);

    if !grounded {
        return false;
    }

    if let Some(velocity) = entity.velocity_mut() {
        velocity.y = impulse.abs(); // y positivo = cima na coordenada da engine
        return true;
    }
    false
}

pub fn clamp_to_ground(entity: &mut Entity, ground_y: f32, collider_half_height: f32) {
    // Chão de emergência: entidade não pode descer abaixo de ground_y + meia altura
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

#[allow(dead_code)]
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

pub fn reset_contact_flags(entity: &mut Entity) {
    for component in &mut entity.components {
        if let Component::RigidBody2D(rb) = component {
            rb.grounded = false;
            rb.hit_ceiling = false;
            rb.hit_left = false;
            rb.hit_right = false;
        }
    }
}

pub fn set_hit_ceiling(entity: &mut Entity, value: bool) {
    for component in &mut entity.components {
        if let Component::RigidBody2D(rb) = component {
            rb.hit_ceiling = value;
        }
    }
}

pub fn set_hit_left(entity: &mut Entity, value: bool) {
    for component in &mut entity.components {
        if let Component::RigidBody2D(rb) = component {
            rb.hit_left = value;
        }
    }
}

pub fn set_hit_right(entity: &mut Entity, value: bool) {
    for component in &mut entity.components {
        if let Component::RigidBody2D(rb) = component {
            rb.hit_right = value;
        }
    }
}
