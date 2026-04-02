// ============================================================
//  engine/scene.rs
//  Cena 2D completa. Responsável por operações de hierarquia.
// ============================================================

use serde::{Deserialize, Serialize};

use super::{
    component::{Camera2D, Component},
    entity::Entity,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub entities: Vec<Entity>,
    pub background_color: [f32; 3],
}

impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        let mut camera = Entity::new("Camera");
        camera.add_component(Component::Camera2D(Camera2D::default()));

        Self {
            name: name.into(),
            entities: vec![camera],
            background_color: [0.15, 0.15, 0.18],
        }
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn find_entity(&self, id: &str) -> Option<&Entity> {
        self.entities.iter().find_map(|entity| entity.find(id))
    }

    pub fn find_entity_mut(&mut self, id: &str) -> Option<&mut Entity> {
        self.entities.iter_mut().find_map(|entity| entity.find_mut(id))
    }

    pub fn remove_entity_by_id(&mut self, id: &str) -> bool {
        remove_entity_recursive(&mut self.entities, id)
    }

    pub fn visit_entities<F>(&self, mut f: F)
    where
        F: FnMut(&Entity),
    {
        for entity in &self.entities {
            entity.visit(&mut f);
        }
    }

    pub fn visit_entities_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Entity),
    {
        for entity in &mut self.entities {
            entity.visit_mut(&mut f);
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

fn remove_entity_recursive(entities: &mut Vec<Entity>, id: &str) -> bool {
    if let Some(index) = entities.iter().position(|entity| entity.id == id) {
        entities.remove(index);
        return true;
    }

    for entity in entities.iter_mut() {
        if remove_entity_recursive(&mut entity.children, id) {
            return true;
        }
    }

    false
}
