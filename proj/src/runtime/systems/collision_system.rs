use crate::core::{component::Component, entity::Entity};

#[derive(Debug, Clone, Copy)]
pub struct RuntimeCollider {
    pub center_x: f32,
    pub center_y: f32,
    pub width: f32,
    pub height: f32,
    pub is_trigger: bool,
    pub entity_ptr: *const Entity,
}

pub fn aabb_collision(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

pub fn collect_colliders(entities: &[Entity], out: &mut Vec<RuntimeCollider>) {
    for entity in entities {
        let mut transform = None;
        let mut collider = None;

        for component in &entity.components {
            match component {
                Component::Transform(t) => transform = Some(t),
                Component::BoxCollider(c) => collider = Some(c),
                _ => {}
            }
        }

        if let (Some(t), Some(c)) = (transform, collider) {
            out.push(RuntimeCollider {
                center_x: t.x + c.offset_x,
                center_y: t.y + c.offset_y,
                width: c.width,
                height: c.height,
                is_trigger: c.is_trigger,
                entity_ptr: entity as *const Entity,
            });
        }

        collect_colliders(&entity.children, out);
    }
}
