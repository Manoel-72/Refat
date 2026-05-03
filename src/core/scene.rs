// ============================================================
//  engine/scene.rs
//  Cena 2D completa. Responsável por operações de hierarquia.
// ============================================================

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    component::{Camera2D, Component},
    entity::Entity,
};

pub const SCENE_FILE_EXTENSION: &str = "scene.json";

/// Referência a um mapa Tiled (JSON) guardada na cena — carregada no Play e na pré-visualização do editor.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SceneTilemapRef {
    /// Caminho relativo à raiz do projeto (ex.: `assets/tilemaps/nivel.json`).
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub entities: Vec<Entity>,
    pub background_color: [f32; 3],
    #[serde(default)]
    pub tilemaps: Vec<SceneTilemapRef>,
}

#[allow(dead_code)]
impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        let mut camera = Entity::new("Camera");
        camera.add_component(Component::Camera2D(Camera2D::default()));

        Self {
            name: name.into(),
            entities: vec![camera],
            background_color: [0.15, 0.15, 0.18],
            tilemaps: Vec::new(),
        }
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn find_entity(&self, id: &str) -> Option<&Entity> {
        self.entities.iter().find_map(|entity| entity.find(id))
    }

    pub fn find_entity_mut(&mut self, id: &str) -> Option<&mut Entity> {
        self.entities
            .iter_mut()
            .find_map(|entity| entity.find_mut(id))
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
        crate::serialization::scene_serializer::scene_to_json(self)
    }

    pub fn from_json(json: &str) -> Option<Self> {
        crate::serialization::scene_serializer::scene_from_json(json)
    }

    pub fn sanitized_file_stem(&self) -> String {
        sanitize_scene_name(&self.name)
    }

    pub fn default_file_name(&self) -> String {
        format!("{}.{}", self.sanitized_file_stem(), SCENE_FILE_EXTENSION)
    }

    pub fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        crate::serialization::scene_serializer::save_scene_to_path(self, path)
    }

    pub fn load_from_path(path: &Path) -> Option<Self> {
        crate::serialization::scene_serializer::load_scene_from_path(path)
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

pub fn sanitize_scene_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.trim_matches('_').is_empty() {
        "scene".to_string()
    } else {
        sanitized
    }
}
