use serde::{Deserialize, Serialize};

use super::{
    component::{Camera2D, Component, ComponentKind},
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
        entity.ensure_transform();
        self.entities.push(entity);
    }

    pub fn ensure_main_camera(&mut self) {
        let has_main_camera = self.entities.iter().any(entity_has_main_camera);
        if has_main_camera {
            return;
        }

        let mut camera = Entity::new("Camera");
        camera.add_component(Component::Camera2D(Camera2D::default()));
        self.entities.insert(0, camera);
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

    pub fn entity_count_recursive(&self) -> usize {
        let mut total = 0;
        self.visit_entities(|_| total += 1);
        total
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

    pub fn main_camera_entity(&self) -> Option<&Entity> {
        let mut found = None;
        self.visit_entities(|entity| {
            if found.is_none() && entity_has_main_camera(entity) {
                found = Some(entity);
            }
        });
        found
    }

    pub fn main_camera_entity_mut(&mut self) -> Option<&mut Entity> {
        fn find_in_children(entity: &mut Entity) -> Option<&mut Entity> {
            if entity.camera().map(|camera| camera.is_main).unwrap_or(false) {
                return Some(entity);
            }
            for child in &mut entity.children {
                if let Some(found) = find_in_children(child) {
                    return Some(found);
                }
            }
            None
        }

        for entity in &mut self.entities {
            if let Some(found) = find_in_children(entity) {
                return Some(found);
            }
        }
        None
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        let mut scene: Scene = serde_json::from_str(json).ok()?;
        for entity in &mut scene.entities {
            entity.ensure_transform();
        }
        scene.ensure_main_camera();
        Some(scene)
    }
}

fn entity_has_main_camera(entity: &Entity) -> bool {
    entity
        .camera()
        .map(|camera| camera.is_main)
        .unwrap_or(false)
}

fn remove_entity_recursive(entities: &mut Vec<Entity>, id: &str) -> bool {
    if let Some(index) = entities.iter().position(|entity| entity.id == id) {
        if entities[index].has_component(ComponentKind::Camera2D)
            && entities[index].camera().map(|camera| camera.is_main).unwrap_or(false)
            && entities.len() == 1
        {
            return false;
        }

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
