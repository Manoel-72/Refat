// ============================================================
//  collision_system.rs
//  Base da colisão 2D: shapes, layers/masks e overlap helpers.
// ============================================================

use crate::core::{
    component::{BodyType, Component, Shape2D},
    entity::Entity,
};

/// Collider coletado das entidades para o frame atual.
#[derive(Debug, Clone)]
pub struct RuntimeCollider {
    pub center_x: f32,
    pub center_y: f32,
    pub width: f32,
    pub height: f32,
    pub shape: Shape2D,
    pub body_type: BodyType,
    pub is_trigger: bool,
    pub collision_enabled: bool,
    pub layer: u32,
    pub mask: u32,
    pub one_way: bool,
    pub one_way_margin: f32,
    pub entity_ptr: *const Entity,
    pub entity_id: String,
    pub entity_name: String,
}

impl RuntimeCollider {
    #[inline]
    pub fn as_box_rect(&self) -> (f32, f32, f32, f32) {
        (
            self.center_x - self.width * 0.5,
            self.center_y - self.height * 0.5,
            self.width,
            self.height,
        )
    }

    #[inline]
    pub fn radius(&self) -> f32 {
        match &self.shape {
            Shape2D::Circle { radius } => (*radius).max(0.0),
            Shape2D::Box { .. } => (self.width.min(self.height) * 0.5).max(0.0),
        }
    }
}

/// Vetor de separação mínima retornado pela resolução MTV.
#[derive(Debug, Clone, Copy, Default)]
pub struct Mtv {
    pub x: f32,
    pub y: f32,
}

impl Mtv {
    #[inline]
    pub fn is_zero(self) -> bool {
        self.x == 0.0 && self.y == 0.0
    }
}

/// Resultado de um raycast.
#[derive(Debug, Clone)]
pub struct RaycastHit {
    pub hit_x: f32,
    pub hit_y: f32,
    pub distance: f32,
    pub entity_ptr: *const Entity,
}

#[inline]
pub fn intersects_box_box(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

#[inline]
pub fn intersects_circle_circle(a: (f32, f32, f32), b: (f32, f32, f32)) -> bool {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let sum = a.2.max(0.0) + b.2.max(0.0);
    (dx * dx) + (dy * dy) <= sum * sum
}

#[inline]
pub fn intersects_box_circle(box_rect: (f32, f32, f32, f32), circle: (f32, f32, f32)) -> bool {
    let (bx, by, bw, bh) = box_rect;
    let (cx, cy, radius) = circle;
    let nearest_x = cx.clamp(bx, bx + bw);
    let nearest_y = cy.clamp(by, by + bh);
    let dx = cx - nearest_x;
    let dy = cy - nearest_y;
    (dx * dx) + (dy * dy) <= radius.max(0.0) * radius.max(0.0)
}

#[inline]
pub fn intersects(a: &RuntimeCollider, b: &RuntimeCollider) -> bool {
    match (&a.shape, &b.shape) {
        (Shape2D::Box { .. }, Shape2D::Box { .. }) => intersects_box_box(a.as_box_rect(), b.as_box_rect()),
        (Shape2D::Circle { .. }, Shape2D::Circle { .. }) => {
            intersects_circle_circle((a.center_x, a.center_y, a.radius()), (b.center_x, b.center_y, b.radius()))
        }
        (Shape2D::Box { .. }, Shape2D::Circle { .. }) => {
            intersects_box_circle(a.as_box_rect(), (b.center_x, b.center_y, b.radius()))
        }
        (Shape2D::Circle { .. }, Shape2D::Box { .. }) => {
            intersects_box_circle(b.as_box_rect(), (a.center_x, a.center_y, a.radius()))
        }
    }
}

/// Mantido por compatibilidade com código existente de box/box.
#[inline]
pub fn aabb_collision(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    intersects_box_box(a, b)
}

/// Calcula o MTV entre dois AABBs.
/// Só deve ser usado para box-vs-box.
pub fn aabb_mtv(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> Mtv {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;

    let acx = ax + aw * 0.5;
    let acy = ay + ah * 0.5;
    let bcx = bx + bw * 0.5;
    let bcy = by + bh * 0.5;

    let overlap_x = (aw + bw) * 0.5 - (acx - bcx).abs();
    let overlap_y = (ah + bh) * 0.5 - (acy - bcy).abs();

    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return Mtv::default();
    }

    if overlap_x < overlap_y {
        let sign = if acx < bcx { -1.0 } else { 1.0 };
        Mtv { x: sign * overlap_x, y: 0.0 }
    } else {
        let sign = if acy < bcy { -1.0 } else { 1.0 };
        Mtv { x: 0.0, y: sign * overlap_y }
    }
}


#[inline]
pub fn mtv_circle_circle(a: (f32, f32, f32), b: (f32, f32, f32)) -> Mtv {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dist_sq = dx * dx + dy * dy;
    let sum = a.2.max(0.0) + b.2.max(0.0);
    if dist_sq >= sum * sum {
        return Mtv::default();
    }
    if dist_sq <= f32::EPSILON {
        return Mtv { x: sum.max(0.001), y: 0.0 };
    }
    let dist = dist_sq.sqrt();
    let penetration = (sum - dist).max(0.0);
    let nx = dx / dist;
    let ny = dy / dist;
    Mtv { x: nx * penetration, y: ny * penetration }
}

#[inline]
pub fn mtv_circle_box(circle: (f32, f32, f32), box_rect: (f32, f32, f32, f32)) -> Mtv {
    let (cx, cy, radius) = circle;
    let (bx, by, bw, bh) = box_rect;
    let nearest_x = cx.clamp(bx, bx + bw);
    let nearest_y = cy.clamp(by, by + bh);
    let dx = cx - nearest_x;
    let dy = cy - nearest_y;
    let dist_sq = dx * dx + dy * dy;
    let radius = radius.max(0.0);

    if dist_sq > f32::EPSILON {
        let dist = dist_sq.sqrt();
        let penetration = radius - dist;
        if penetration <= 0.0 {
            return Mtv::default();
        }
        return Mtv { x: (dx / dist) * penetration, y: (dy / dist) * penetration };
    }

    let left_pen = (cx - bx).abs();
    let right_pen = (bx + bw - cx).abs();
    let down_pen = (cy - by).abs();
    let up_pen = (by + bh - cy).abs();
    let min_pen = left_pen.min(right_pen).min(down_pen).min(up_pen);

    if min_pen == left_pen {
        Mtv { x: -(radius + left_pen).max(0.001), y: 0.0 }
    } else if min_pen == right_pen {
        Mtv { x: (radius + right_pen).max(0.001), y: 0.0 }
    } else if min_pen == down_pen {
        Mtv { x: 0.0, y: -(radius + down_pen).max(0.001) }
    } else {
        Mtv { x: 0.0, y: (radius + up_pen).max(0.001) }
    }
}

#[inline]
pub fn mtv_box_circle(box_rect: (f32, f32, f32, f32), circle: (f32, f32, f32)) -> Mtv {
    let mtv = mtv_circle_box(circle, box_rect);
    Mtv { x: -mtv.x, y: -mtv.y }
}

#[inline]
pub fn mtv(a: &RuntimeCollider, b: &RuntimeCollider) -> Mtv {
    match (&a.shape, &b.shape) {
        (Shape2D::Box { .. }, Shape2D::Box { .. }) => aabb_mtv(a.as_box_rect(), b.as_box_rect()),
        (Shape2D::Circle { .. }, Shape2D::Circle { .. }) => {
            mtv_circle_circle((a.center_x, a.center_y, a.radius()), (b.center_x, b.center_y, b.radius()))
        }
        (Shape2D::Box { .. }, Shape2D::Circle { .. }) => {
            mtv_box_circle(a.as_box_rect(), (b.center_x, b.center_y, b.radius()))
        }
        (Shape2D::Circle { .. }, Shape2D::Box { .. }) => {
            mtv_circle_box((a.center_x, a.center_y, a.radius()), b.as_box_rect())
        }
    }
}

#[inline]
pub fn can_collide(a: &RuntimeCollider, b: &RuntimeCollider) -> bool {
    if !a.collision_enabled || !b.collision_enabled {
        return false;
    }
    if a.entity_id == b.entity_id {
        return false;
    }
    if !a.entity_ptr.is_null() && !b.entity_ptr.is_null() && std::ptr::eq(a.entity_ptr, b.entity_ptr) {
        return false;
    }

    let a_hits_b = a.mask == 0 || b.layer == 0 || (a.mask & b.layer) != 0;
    let b_hits_a = b.mask == 0 || a.layer == 0 || (b.mask & a.layer) != 0;
    a_hits_b && b_hits_a
}

#[inline]
pub fn layers_interact(a: &RuntimeCollider, b: &RuntimeCollider) -> bool {
    can_collide(a, b)
}

pub fn collect_colliders(entities: &[Entity], out: &mut Vec<RuntimeCollider>) {
    for entity in entities {
        let mut transform = None;
        let mut rigidbody = None;
        let mut collider_data = None;

        for component in &entity.components {
            match component {
                Component::Transform(t) => transform = Some(t),
                Component::RigidBody2D(rb) => rigidbody = Some(rb),
                Component::BoxCollider(c) => collider_data = Some(c),
                _ => {}
            }
        }

        if let (Some(t), Some(c)) = (transform, collider_data) {
            if !c.collision_enabled {
                collect_colliders(&entity.children, out);
                continue;
            }

            let resolved_shape = c.resolved_shape();
            let (width, height) = match &resolved_shape {
                Shape2D::Box { width, height } => (*width, *height),
                Shape2D::Circle { radius } => (radius * 2.0, radius * 2.0),
            };
            let body_type = c.resolved_body_type(rigidbody);
            let is_trigger = matches!(body_type, BodyType::Trigger) || c.is_trigger;

            out.push(RuntimeCollider {
                center_x: t.x + c.offset_x,
                center_y: t.y + c.offset_y,
                width,
                height,
                shape: resolved_shape,
                body_type,
                is_trigger,
                collision_enabled: c.collision_enabled,
                layer: c.layer,
                mask: c.mask,
                one_way: c.one_way,
                one_way_margin: c.one_way_margin,
                entity_ptr: entity as *const Entity,
                entity_id: entity.id.clone(),
                entity_name: entity.name.clone(),
            });
        }

        collect_colliders(&entity.children, out);
    }
}

pub fn raycast(
    origin: (f32, f32),
    dir: (f32, f32),
    max_dist: f32,
    layer_mask: u32,
    colliders: &[RuntimeCollider],
) -> Option<RaycastHit> {
    let (ox, oy) = origin;
    let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
    if len < f32::EPSILON || max_dist <= 0.0 {
        return None;
    }

    let inv_dx = if dir.0.abs() > f32::EPSILON { Some(1.0 / dir.0) } else { None };
    let inv_dy = if dir.1.abs() > f32::EPSILON { Some(1.0 / dir.1) } else { None };
    let mut best: Option<RaycastHit> = None;

    for col in colliders {
        if !col.collision_enabled {
            continue;
        }
        if layer_mask != 0 && col.layer != 0 && (col.layer & layer_mask) == 0 {
            continue;
        }

        let min_x = col.center_x - col.width * 0.5;
        let max_x = col.center_x + col.width * 0.5;
        let min_y = col.center_y - col.height * 0.5;
        let max_y = col.center_y + col.height * 0.5;

        let (tx1, tx2) = match inv_dx {
            Some(inv) => ((min_x - ox) * inv, (max_x - ox) * inv),
            None if ox >= min_x && ox <= max_x => (f32::NEG_INFINITY, f32::INFINITY),
            None => continue,
        };
        let (ty1, ty2) = match inv_dy {
            Some(inv) => ((min_y - oy) * inv, (max_y - oy) * inv),
            None if oy >= min_y && oy <= max_y => (f32::NEG_INFINITY, f32::INFINITY),
            None => continue,
        };

        let t_min = tx1.min(tx2).max(ty1.min(ty2)).max(0.0);
        let t_max = tx1.max(tx2).min(ty1.max(ty2));
        if t_max < t_min {
            continue;
        }

        let distance = t_min * len;
        if distance > max_dist {
            continue;
        }

        let hit = RaycastHit {
            hit_x: ox + dir.0 * t_min,
            hit_y: oy + dir.1 * t_min,
            distance,
            entity_ptr: col.entity_ptr,
        };

        match &best {
            None => best = Some(hit),
            Some(prev) if hit.distance < prev.distance => best = Some(hit),
            _ => {}
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_box(layer: u32, mask: u32) -> RuntimeCollider {
        RuntimeCollider {
            center_x: 0.0,
            center_y: 0.0,
            width: 32.0,
            height: 32.0,
            shape: Shape2D::Box { width: 32.0, height: 32.0 },
            body_type: BodyType::Kinematic,
            is_trigger: false,
            collision_enabled: true,
            layer,
            mask,
            entity_ptr: std::ptr::null(),
            entity_id: format!("{layer}:{mask}"),
            entity_name: String::new(),
        }
    }

    #[test]
    fn mtv_sem_sobreposicao() {
        let a = (0.0, 0.0, 32.0, 32.0);
        let b = (64.0, 0.0, 32.0, 32.0);
        let mtv = aabb_mtv(a, b);
        assert!(mtv.is_zero());
    }

    #[test]
    fn intersect_box_box() {
        assert!(intersects_box_box((0.0, 0.0, 32.0, 32.0), (16.0, 16.0, 32.0, 32.0)));
        assert!(!intersects_box_box((0.0, 0.0, 32.0, 32.0), (40.0, 0.0, 16.0, 16.0)));
    }

    #[test]
    fn intersect_circle_circle() {
        assert!(intersects_circle_circle((0.0, 0.0, 10.0), (15.0, 0.0, 10.0)));
        assert!(!intersects_circle_circle((0.0, 0.0, 10.0), (25.1, 0.0, 10.0)));
    }

    #[test]
    fn intersect_box_circle() {
        assert!(intersects_box_circle((0.0, 0.0, 20.0, 20.0), (25.0, 10.0, 6.0)));
        assert!(!intersects_box_circle((0.0, 0.0, 20.0, 20.0), (40.0, 10.0, 5.0)));
    }

    #[test]
    fn layers_zero_sempre_interagem() {
        assert!(can_collide(&make_box(0, 0), &make_box(2, 1)));
    }

    #[test]
    fn layer_mask_bloqueia_colisao() {
        assert!(!can_collide(&make_box(2, 4), &make_box(8, 1)));
    }

    #[test]
    fn collider_desabilitado_bloqueia() {
        let mut a = make_box(1, 1);
        a.collision_enabled = false;
        assert!(!can_collide(&a, &make_box(1, 1)));
    }
}

