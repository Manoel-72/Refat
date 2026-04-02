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
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            visible: true,
            children: Vec::new(),
            components: vec![Component::transform_default()],
            matr_source: None,
        }
    }

    pub fn with_components(name: impl Into<String>, components: Vec<Component>) -> Self {
        let mut entity = Self::new(name);
        entity.components.clear();
        entity.ensure_transform();
        for component in components {
            entity.add_component(component);
        }
        entity.ensure_transform();
        entity
    }

    pub fn ensure_transform(&mut self) {
        if !self.has_component(ComponentKind::Transform) {
            self.components.insert(0, Component::transform_default());
        } else if !matches!(self.components.first(), Some(Component::Transform(_))) {
            if let Some(index) = self
                .components
                .iter()
                .position(|component| matches!(component, Component::Transform(_)))
            {
                let transform = self.components.remove(index);
                self.components.insert(0, transform);
            }
        }
    }

    pub fn add_component(&mut self, component: Component) {
        if component.is_unique() {
            if let Some(existing) = self
                .components
                .iter_mut()
                .find(|existing| existing.kind() == component.kind())
            {
                *existing = component;
                self.ensure_transform();
                return;
            }
        }

        self.components.push(component);
        self.ensure_transform();
    }

    pub fn add_script(&mut self, file_path: impl Into<String>) {
        self.components
            .push(Component::Script(Script { file_path: file_path.into() }));
    }

    pub fn has_component(&self, kind: ComponentKind) -> bool {
        self.components.iter().any(|component| component.kind() == kind)
    }

    pub fn component(&self, kind: ComponentKind) -> Option<&Component> {
        self.components.iter().find(|component| component.kind() == kind)
    }

    pub fn component_mut(&mut self, kind: ComponentKind) -> Option<&mut Component> {
        self.components
            .iter_mut()
            .find(|component| component.kind() == kind)
    }

    pub fn components_of_kind(&self, kind: ComponentKind) -> impl Iterator<Item = &Component> {
        self.components.iter().filter(move |component| component.kind() == kind)
    }

    pub fn remove_component(&mut self, index: usize) {
        if index >= self.components.len() {
            return;
        }

        if matches!(self.components.get(index), Some(Component::Transform(_))) {
            return;
        }

        self.components.remove(index);
        self.ensure_transform();
    }

    pub fn remove_component_by_kind(&mut self, kind: ComponentKind) -> bool {
        if kind == ComponentKind::Transform {
            return false;
        }

        if let Some(index) = self.components.iter().position(|component| component.kind() == kind) {
            self.components.remove(index);
            return true;
        }

        false
    }

    pub fn transform(&self) -> Option<&Transform> {
        match self.component(ComponentKind::Transform) {
            Some(Component::Transform(transform)) => Some(transform),
            _ => None,
        }
    }

    pub fn transform_mut(&mut self) -> Option<&mut Transform> {
        match self.component_mut(ComponentKind::Transform) {
            Some(Component::Transform(transform)) => Some(transform),
            _ => None,
        }
    }

    pub fn sprite(&self) -> Option<&Sprite> {
        match self.component(ComponentKind::Sprite) {
            Some(Component::Sprite(sprite)) => Some(sprite),
            _ => None,
        }
    }

    pub fn sprite_mut(&mut self) -> Option<&mut Sprite> {
        match self.component_mut(ComponentKind::Sprite) {
            Some(Component::Sprite(sprite)) => Some(sprite),
            _ => None,
        }
    }

    pub fn camera(&self) -> Option<&Camera2D> {
        match self.component(ComponentKind::Camera2D) {
            Some(Component::Camera2D(camera)) => Some(camera),
            _ => None,
        }
    }

    pub fn camera_mut(&mut self) -> Option<&mut Camera2D> {
        match self.component_mut(ComponentKind::Camera2D) {
            Some(Component::Camera2D(camera)) => Some(camera),
            _ => None,
        }
    }

    pub fn rigidbody(&self) -> Option<&RigidBody2D> {
        match self.component(ComponentKind::RigidBody2D) {
            Some(Component::RigidBody2D(rigidbody)) => Some(rigidbody),
            _ => None,
        }
    }

    pub fn rigidbody_mut(&mut self) -> Option<&mut RigidBody2D> {
        match self.component_mut(ComponentKind::RigidBody2D) {
            Some(Component::RigidBody2D(rigidbody)) => Some(rigidbody),
            _ => None,
        }
    }

    pub fn collider(&self) -> Option<&BoxCollider> {
        match self.component(ComponentKind::BoxCollider) {
            Some(Component::BoxCollider(collider)) => Some(collider),
            _ => None,
        }
    }

    pub fn collider_mut(&mut self) -> Option<&mut BoxCollider> {
        match self.component_mut(ComponentKind::BoxCollider) {
            Some(Component::BoxCollider(collider)) => Some(collider),
            _ => None,
        }
    }

    pub fn first_script(&self) -> Option<&Script> {
        self.components_of_kind(ComponentKind::Script)
            .find_map(|component| match component {
                Component::Script(script) => Some(script),
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
        child.ensure_transform();
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
        let mut entity: Entity = serde_json::from_str(json).ok()?;
        entity.ensure_transform();
        Some(entity)
    }
}
