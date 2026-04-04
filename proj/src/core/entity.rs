// ============================================================
//  engine/entity.rs
//  Entidade: objeto do mundo do jogo com hierarquia e componentes.
// ============================================================

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::component::{Component, Transform, Velocity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub children: Vec<Entity>,
    pub components: Vec<Component>,
    #[serde(default)]
    pub matr_source: Option<String>,
}

#[allow(dead_code)]
impl Entity {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            visible: true,
            children: Vec::new(),
            components: vec![Component::transform_default()],
            matr_source: None,
        }
    }

    pub fn add_component(&mut self, component: Component) {
        if let Some(index) = self.components.iter().position(|existing| component_kind(existing) == component_kind(&component)) {
            self.components[index] = component;
        } else {
            self.components.push(component);
        }
    }

    pub fn remove_component(&mut self, index: usize) {
        if index < self.components.len() && !matches!(self.components.get(index), Some(Component::Transform(_))) {
            self.components.remove(index);
        }
    }

    pub fn transform(&self) -> Option<&Transform> {
        self.components.iter().find_map(|component| match component {
            Component::Transform(transform) => Some(transform),
            _ => None,
        })
    }

    pub fn transform_mut(&mut self) -> Option<&mut Transform> {
        self.components.iter_mut().find_map(|component| match component {
            Component::Transform(transform) => Some(transform),
            _ => None,
        })
    }


    pub fn velocity(&self) -> Option<&Velocity> {
        self.components.iter().find_map(|component| match component {
            Component::Velocity(velocity) => Some(velocity),
            _ => None,
        })
    }

    pub fn velocity_mut(&mut self) -> Option<&mut Velocity> {
        self.components.iter_mut().find_map(|component| match component {
            Component::Velocity(velocity) => Some(velocity),
            _ => None,
        })
    }

    pub fn find(&self, id: &str) -> Option<&Entity> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut Entity> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter_mut().find_map(|child| child.find_mut(id))
    }

    pub fn regenerate_ids_recursive(&mut self) {
        self.id = Uuid::new_v4().to_string();
        for child in &mut self.children {
            child.regenerate_ids_recursive();
        }
    }

    pub fn add_child(&mut self, child: Entity) {
        self.children.push(child);
    }

    pub fn visit<F>(&self, f: &mut F)
    where
        F: FnMut(&Entity),
    {
        f(self);
        for child in &self.children {
            child.visit(f);
        }
    }

    pub fn visit_mut<F>(&mut self, f: &mut F)
    where
        F: FnMut(&mut Entity),
    {
        f(self);
        for child in &mut self.children {
            child.visit_mut(f);
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

fn component_kind(component: &Component) -> u8 {
    match component {
        Component::Transform(_) => 0,
        Component::Sprite(_) => 1,
        Component::Camera2D(_) => 2,
        Component::RigidBody2D(_) => 3,
        Component::Velocity(_) => 4,
        Component::BoxCollider(_) => 5,
        Component::Script(_) => 6,
        Component::Audio(_) => 7,
        Component::Animator(_) => 8,
        Component::TextLabel(_) => 9,
        Component::UIButton(_) => 10,
    }
}
