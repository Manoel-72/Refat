use crate::core::entity::Entity;

pub fn count_renderables(entities: &[Entity]) -> usize {
    let mut total = 0;
    for entity in entities {
        if entity.visible {
            total += 1;
        }
        total += count_renderables(&entity.children);
    }
    total
}
