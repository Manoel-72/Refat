// ============================================================
//  collision_system.rs  —  V0.9
//  Detecção AABB + resolução MTV + layers/masks + raycast
// ============================================================

use crate::core::{component::Component, entity::Entity};

/// Collider coletado das entidades para o frame atual.
#[derive(Debug, Clone)]
pub struct RuntimeCollider {
    pub center_x: f32,
    pub center_y: f32,
    pub width: f32,
    pub height: f32,
    pub is_trigger: bool,
    pub collision_enabled: bool,
    /// Máscara de layer (bits 0–7). Colisão só ocorre se (a.layer & b.mask) != 0.
    pub layer: u8,
    pub mask: u8,
    pub entity_ptr: *const Entity,
    pub entity_id: String,
    pub entity_name: String,
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
    /// Posição do ponto de impacto.
    pub hit_x: f32,
    pub hit_y: f32,
    /// Distância desde a origem.
    pub distance: f32,
    /// Ponteiro para a entidade atingida (para leitura segura de nome/id).
    pub entity_ptr: *const Entity,
}

// ── detecção ────────────────────────────────────────────────

/// Detecta sobreposição AABB simples. Retorna true se há colisão.
#[inline]
pub fn aabb_collision(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

/// Calcula o MTV (Minimum Translation Vector) entre dois AABBs.
/// Retorna o vetor que separa `a` de `b` pelo menor deslocamento possível.
/// Retorna Mtv { 0, 0 } se não há sobreposição.
pub fn aabb_mtv(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> Mtv {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;

    // centros
    let acx = ax + aw * 0.5;
    let acy = ay + ah * 0.5;
    let bcx = bx + bw * 0.5;
    let bcy = by + bh * 0.5;

    let overlap_x = (aw + bw) * 0.5 - (acx - bcx).abs();
    let overlap_y = (ah + bh) * 0.5 - (acy - bcy).abs();

    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return Mtv::default();
    }

    // empurra pelo eixo de menor sobreposição
    if overlap_x < overlap_y {
        let sign = if acx < bcx { -1.0 } else { 1.0 };
        Mtv { x: sign * overlap_x, y: 0.0 }
    } else {
        let sign = if acy < bcy { -1.0 } else { 1.0 };
        Mtv { x: 0.0, y: sign * overlap_y }
    }
}

// ── coleta ───────────────────────────────────────────────────

pub fn collect_colliders(entities: &[Entity], out: &mut Vec<RuntimeCollider>) {
    for entity in entities {
        let mut transform = None;
        let mut collider_data = None;

        for component in &entity.components {
            match component {
                Component::Transform(t) => transform = Some(t),
                Component::BoxCollider(c) => collider_data = Some(c),
                _ => {}
            }
        }

        if let (Some(t), Some(c)) = (transform, collider_data) {
            if !c.collision_enabled {
                collect_colliders(&entity.children, out);
                continue;
            }
            out.push(RuntimeCollider {
                center_x: t.x + c.offset_x,
                center_y: t.y + c.offset_y,
                width: c.width,
                height: c.height,
                is_trigger: c.is_trigger,
                collision_enabled: c.collision_enabled,
                layer: c.layer,
                mask: c.mask,
                entity_ptr: entity as *const Entity,
                entity_id: entity.id.clone(),
                entity_name: entity.name.clone(),
            });
        }

        collect_colliders(&entity.children, out);
    }
}

// ── layers ───────────────────────────────────────────────────

/// Verifica se dois colliders devem interagir considerando layer/mask.
/// Regra: colisão ocorre se (a.layer & b.mask) != 0 OU (b.layer & a.mask) != 0.
#[inline]
pub fn layers_interact(a: &RuntimeCollider, b: &RuntimeCollider) -> bool {
    // layer 0 / mask 0 = colidem com tudo (compatibilidade com cenas antigas)
    if a.layer == 0 && a.mask == 0 && b.layer == 0 && b.mask == 0 {
        return true;
    }
    let a_hits_b = a.mask == 0 || (a.mask & b.layer) != 0;
    let b_hits_a = b.mask == 0 || (b.mask & a.layer) != 0;
    a_hits_b || b_hits_a
}

// ── raycast ──────────────────────────────────────────────────

/// Lança um raio a partir de `origin` na direção `dir` (normalizado)
/// até `max_dist`. Retorna o primeiro collider atingido que pertença
/// a `layer_mask` (0 = qualquer layer).
///
/// Algoritmo: amostragem por passos de meio-largura mínima para manter
/// simplicidade sem depender de hit-parametric, adequado para V0.9.
pub fn raycast(
    origin: (f32, f32),
    dir: (f32, f32),
    max_dist: f32,
    layer_mask: u8,
    colliders: &[RuntimeCollider],
) -> Option<RaycastHit> {
    let (ox, oy) = origin;
    let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
    if len < f32::EPSILON {
        return None;
    }
    let (dx, dy) = (dir.0 / len, dir.1 / len);

    // passo = metade do menor collider presente (min 2 px)
    let step = colliders
        .iter()
        .map(|c| c.width.min(c.height))
        .fold(f32::MAX, f32::min)
        * 0.5;
    let step = step.max(2.0).min(16.0);

    // Coleta TODOS os hits e retorna o de menor distância.
    // Isso corrige o bug onde dois colliders sobrepostos na mesma amostra
    // retornavam o da posição 0 da lista em vez do geometricamente mais próximo.
    let mut best: Option<RaycastHit> = None;

    let mut dist = 0.0_f32;
    while dist <= max_dist {
        let px = ox + dx * dist;
        let py = oy + dy * dist;

        for col in colliders {
            if layer_mask != 0 && (col.layer & layer_mask) == 0 {
                continue;
            }
            let half_w = col.width * 0.5;
            let half_h = col.height * 0.5;
            if (px - col.center_x).abs() <= half_w && (py - col.center_y).abs() <= half_h {
                let hit = RaycastHit {
                    hit_x: px,
                    hit_y: py,
                    distance: dist,
                    entity_ptr: col.entity_ptr,
                };
                // Guarda apenas o hit de menor distância
                match &best {
                    None => best = Some(hit),
                    Some(prev) if dist < prev.distance => best = Some(hit),
                    _ => {}
                }
            }
        }

        // Se já encontrou um hit neste passo e o próximo passo estaria além,
        // podemos retornar imediatamente — não haverá hit mais próximo adiante.
        if best.is_some() {
            return best;
        }

        dist += step;
    }

    best
}

// ── testes ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtv_sem_sobreposicao() {
        let a = (0.0, 0.0, 32.0, 32.0);
        let b = (64.0, 0.0, 32.0, 32.0);
        let mtv = aabb_mtv(a, b);
        assert!(mtv.is_zero(), "sem sobreposição deve retornar zero");
    }

    #[test]
    fn mtv_sobreposicao_horizontal() {
        // a está à esquerda, sobreposição de 8px em X
        let a = (0.0, 0.0, 32.0, 32.0);
        let b = (24.0, 0.0, 32.0, 32.0);
        let mtv = aabb_mtv(a, b);
        assert!(mtv.x < 0.0, "MTV deve empurrar à esquerda");
        assert_eq!(mtv.y, 0.0);
    }

    #[test]
    fn mtv_sobreposicao_vertical() {
        // a acima de b, sobreposição pequena em Y
        let a = (0.0, 0.0, 32.0, 32.0);
        let b = (0.0, 28.0, 32.0, 32.0);
        let mtv = aabb_mtv(a, b);
        assert!(mtv.y < 0.0, "MTV deve empurrar para cima");
        assert_eq!(mtv.x, 0.0);
    }

    #[test]
    fn layers_zero_sempre_interagem() {
        let make = |layer: u8, mask: u8| RuntimeCollider {
            center_x: 0.0, center_y: 0.0,
            width: 32.0, height: 32.0,
            is_trigger: false, layer, mask,
            entity_ptr: std::ptr::null(),
            entity_id: String::new(), entity_name: String::new(),
        };
        assert!(layers_interact(&make(0, 0), &make(0, 0)));
    }

    #[test]
    fn layers_sem_intersecao_nao_interagem() {
        let a = RuntimeCollider { center_x:0.0, center_y:0.0, width:32.0, height:32.0,
            is_trigger:false, layer:0b0001, mask:0b0001, entity_ptr:std::ptr::null(), entity_id:String::new(), entity_name:String::new() };
        let b = RuntimeCollider { center_x:0.0, center_y:0.0, width:32.0, height:32.0,
            is_trigger:false, layer:0b0010, mask:0b0010, entity_ptr:std::ptr::null(), entity_id:String::new(), entity_name:String::new() };
        assert!(!layers_interact(&a, &b));
    }
}
