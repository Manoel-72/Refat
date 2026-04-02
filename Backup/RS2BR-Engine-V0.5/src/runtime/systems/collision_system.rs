use crate::core::{component::Component, entity::Entity};

pub fn aabb_collision(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

pub fn collect_colliders<'a>(entities: &'a [Entity], out: &mut Vec<(f32, f32, f32, f32, *const Entity)>) {
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
            out.push((
                t.x + c.offset_x,
                t.y + c.offset_y,
                c.width,
                c.height,
                entity as *const Entity,
            ));
        }

        collect_colliders(&entity.children, out);
    }
}
