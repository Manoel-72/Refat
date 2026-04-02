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
        let mut scene = Self {
            name: name.into(),
            entities: Vec::new(),
            background_color: [0.15, 0.15, 0.18],
        };
        scene.ensure_main_camera();
        scene
    }

    pub fn add_entity(&mut self, mut entity: Entity) {
        entity.ensure_required_components();
        self.entities.push(entity);
        self.ensure_main_camera();
    }

    pub fn find_entity(&self, id: &str) -> Option<&Entity> {
        self.entities.iter().find_map(|entity| entity.find(id))
    }

    pub fn find_entity_mut(&mut self, id: &str) -> Option<&mut Entity> {
        self.entities.iter_mut().find_map(|entity| entity.find_mut(id))
    }

    pub fn remove_entity_by_id(&mut self, id: &str) -> bool {
        let removed = remove_entity_recursive(&mut self.entities, id);
        if removed {
            self.ensure_main_camera();
        }
        removed
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

    pub fn ensure_main_camera(&mut self) {
        let mut found_main = false;

        self.visit_entities_mut(|entity| {
            if let Some(camera) = entity.camera_mut() {
                if !found_main && camera.is_main {
                    found_main = true;
                } else if found_main && camera.is_main {
                    camera.is_main = false;
                }
            }
        });

        if !found_main {
            let mut camera = Entity::new("Camera");
            camera.add_component(Component::Camera2D(Camera2D::default()));
            self.entities.insert(0, camera);
        }
    }

    pub fn main_camera(&self) -> Option<&Entity> {
        self.entities.iter().find_map(find_main_camera_recursive)
    }

    pub fn main_camera_mut(&mut self) -> Option<&mut Entity> {
        self.entities.iter_mut().find_map(find_main_camera_recursive_mut)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        let mut scene: Self = serde_json::from_str(json).ok()?;
        for entity in &mut scene.entities {
            entity.ensure_required_components();
        }
        scene.ensure_main_camera();
        Some(scene)
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

fn find_main_camera_recursive(entity: &Entity) -> Option<&Entity> {
    if entity.camera().is_some_and(|camera| camera.is_main) {
        return Some(entity);
    }

    entity.children.iter().find_map(find_main_camera_recursive)
}

fn find_main_camera_recursive_mut(entity: &mut Entity) -> Option<&mut Entity> {
    if entity.camera().is_some_and(|camera| camera.is_main) {
        return Some(entity);
    }

    entity
        .children
        .iter_mut()
        .find_map(find_main_camera_recursive_mut)
}
