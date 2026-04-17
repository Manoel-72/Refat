// ============================================================
//  engine/entity.rs
//  Entidade: objeto do mundo do jogo com hierarquia e componentes.
// ============================================================

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::component::{BoxCollider, Component, Transform, Velocity};

fn default_entity_hp() -> f32 { 100.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub children: Vec<Entity>,
    pub components: Vec<Component>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub matr_source: Option<String>,
    #[serde(default = "default_entity_hp")]
    pub hp: f32,
    #[serde(default = "default_entity_hp")]
    pub max_hp: f32,
    #[serde(default)]
    pub is_dead: bool,
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
            tags: Vec::new(),
            matr_source: None,
            hp: default_entity_hp(),
            max_hp: default_entity_hp(),
            is_dead: false,
        }
    }



    pub fn add_tag(&mut self, tag: impl Into<String>) -> bool {
        let normalized = tag.into().trim().to_string();
        if normalized.is_empty() {
            return false;
        }
        if self.tags.iter().any(|existing| existing.eq_ignore_ascii_case(&normalized)) {
            return false;
        }
        self.tags.push(normalized);
        true
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|existing| existing.eq_ignore_ascii_case(tag))
    }

    pub fn remove_tag(&mut self, tag: &str) -> bool {
        if let Some(index) = self.tags.iter().position(|existing| existing.eq_ignore_ascii_case(tag)) {
            self.tags.remove(index);
            return true;
        }
        false
    }



    pub fn hp(&self) -> f32 {
        self.hp
    }

    pub fn set_hp(&mut self, value: f32) {
        let clamped = value.max(0.0);
        self.hp = clamped;
        if self.max_hp < clamped {
            self.max_hp = clamped;
        }
        self.is_dead = self.hp <= 0.0;
    }

    pub fn max_hp(&self) -> f32 {
        self.max_hp
    }

    pub fn set_max_hp(&mut self, value: f32) {
        self.max_hp = value.max(0.0);
        if self.hp > self.max_hp && self.max_hp > 0.0 {
            self.hp = self.max_hp;
        }
        self.is_dead = self.hp <= 0.0;
    }

    pub fn damage(&mut self, amount: f32) -> f32 {
        let amount = amount.max(0.0);
        self.hp = (self.hp - amount).max(0.0);
        self.is_dead = self.hp <= 0.0;
        self.hp
    }

    pub fn heal(&mut self, amount: f32) -> f32 {
        let amount = amount.max(0.0);
        let limit = if self.max_hp <= 0.0 { default_entity_hp() } else { self.max_hp };
        self.hp = (self.hp + amount).min(limit);
        self.is_dead = self.hp <= 0.0;
        self.hp
    }

    pub fn is_alive(&self) -> bool {
        !self.is_dead && self.hp > 0.0
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

    pub fn collision_enabled(&self) -> bool {
        self.components.iter().find_map(|component| match component {
            Component::BoxCollider(collider) => Some(collider.collision_enabled),
            _ => None,
        }).unwrap_or(false)
    }

    pub fn set_collision_enabled(&mut self, enabled: bool) -> bool {
        for component in &mut self.components {
            if let Component::BoxCollider(collider) = component {
                collider.collision_enabled = enabled;
                return true;
            }
        }
        false
    }

    pub fn box_collider_mut(&mut self) -> Option<&mut BoxCollider> {
        self.components.iter_mut().find_map(|component| match component {
            Component::BoxCollider(collider) => Some(collider),
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
        Component::LuaScript(_) => 7,
        Component::Audio(_) => 8,
        Component::Animator(_) => 9,
        Component::TextLabel(_) => 10,
        Component::UIButton(_) => 11,
    }
}
