// ============================================================
//  engine/entity.rs
//  Entidade: objeto do mundo do jogo com hierarquia e componentes.
// ============================================================

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::component::{
    BoxCollider, Camera2D, Component, ComponentKind, RigidBody2D, Script, Sprite, Transform,
};

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

impl Entity {
    pub fn new(name: impl Into<String>) -> Self {
        let mut entity = Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            visible: true,
            children: Vec::new(),
            components: Vec::new(),
            matr_source: None,
        };
        entity.ensure_required_components();
        entity
    }

    pub fn add_component(&mut self, component: Component) {
        let kind = component.kind();

        if Component::is_unique_kind(kind) {
            if let Some(existing) = self.components.iter_mut().find(|item| item.kind() == kind) {
                *existing = component;
                return;
            }
        }

        self.components.push(component);
        self.ensure_required_components();
    }

    pub fn has_component_kind(&self, kind: ComponentKind) -> bool {
        self.components.iter().any(|component| component.kind() == kind)
    }

    pub fn remove_component(&mut self, index: usize) {
        if index >= self.components.len() {
            return;
        }

        if self.components[index].kind() == ComponentKind::Transform {
            return;
        }

        self.components.remove(index);
    }

    pub fn remove_component_kind(&mut self, kind: ComponentKind) -> bool {
        if kind == ComponentKind::Transform {
            return false;
        }

        if let Some(index) = self.components.iter().position(|component| component.kind() == kind) {
            self.components.remove(index);
            return true;
        }

        false
    }

    pub fn ensure_required_components(&mut self) {
        if !self.has_component_kind(ComponentKind::Transform) {
            self.components.insert(0, Component::transform_default());
        }
    }

    pub fn transform(&self) -> Option<&Transform> {
        self.components.iter().find_map(|component| match component {
            Component::Transform(value) => Some(value),
            _ => None,
        })
    }

    pub fn transform_mut(&mut self) -> Option<&mut Transform> {
        self.components.iter_mut().find_map(|component| match component {
            Component::Transform(value) => Some(value),
            _ => None,
        })
    }

    pub fn sprite(&self) -> Option<&Sprite> {
        self.components.iter().find_map(|component| match component {
            Component::Sprite(value) => Some(value),
            _ => None,
        })
    }

    pub fn sprite_mut(&mut self) -> Option<&mut Sprite> {
        self.components.iter_mut().find_map(|component| match component {
            Component::Sprite(value) => Some(value),
            _ => None,
        })
    }

    pub fn camera(&self) -> Option<&Camera2D> {
        self.components.iter().find_map(|component| match component {
            Component::Camera2D(value) => Some(value),
            _ => None,
        })
    }

    pub fn camera_mut(&mut self) -> Option<&mut Camera2D> {
        self.components.iter_mut().find_map(|component| match component {
            Component::Camera2D(value) => Some(value),
            _ => None,
        })
    }

    pub fn rigidbody(&self) -> Option<&RigidBody2D> {
        self.components.iter().find_map(|component| match component {
            Component::RigidBody2D(value) => Some(value),
            _ => None,
        })
    }

    pub fn rigidbody_mut(&mut self) -> Option<&mut RigidBody2D> {
        self.components.iter_mut().find_map(|component| match component {
            Component::RigidBody2D(value) => Some(value),
            _ => None,
        })
    }

    pub fn collider(&self) -> Option<&BoxCollider> {
        self.components.iter().find_map(|component| match component {
            Component::BoxCollider(value) => Some(value),
            _ => None,
        })
    }

    pub fn collider_mut(&mut self) -> Option<&mut BoxCollider> {
        self.components.iter_mut().find_map(|component| match component {
            Component::BoxCollider(value) => Some(value),
            _ => None,
        })
    }

    pub fn script(&self) -> Option<&Script> {
        self.components.iter().find_map(|component| match component {
            Component::Script(value) => Some(value),
            _ => None,
        })
    }

    pub fn script_mut(&mut self) -> Option<&mut Script> {
        self.components.iter_mut().find_map(|component| match component {
            Component::Script(value) => Some(value),
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

    pub fn add_child(&mut self, mut child: Entity) {
        child.ensure_required_components();
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
        let mut entity: Self = serde_json::from_str(json).ok()?;
        entity.ensure_required_components();
        Some(entity)
    }
}
