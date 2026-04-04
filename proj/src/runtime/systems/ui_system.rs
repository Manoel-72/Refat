use crate::core::{component::Component, entity::Entity};

pub fn count_scripted_ui_entities(entities: &[Entity]) -> usize {
    let mut total = 0;
    for entity in entities {
        if entity.components.iter().any(|component| matches!(component, Component::Script(_) | Component::TextLabel(_) | Component::UIButton(_))) {
            total += 1;
        }
        total += count_scripted_ui_entities(&entity.children);
    }
    total
}
