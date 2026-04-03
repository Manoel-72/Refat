use serde::{Deserialize, Serialize};

use super::entity::Entity;

pub const PREFAB_FILE_EXTENSION: &str = "prefab.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefab {
    #[serde(default = "default_prefab_name")]
    pub name: String,
    #[serde(default = "default_prefab_root_entity")]
    pub root_entity: Entity,
}

impl Prefab {
    pub fn new(name: impl Into<String>, root_entity: Entity) -> Self {
        Self {
            name: name.into(),
            root_entity,
        }
    }

    pub fn to_json(&self) -> String {
        crate::serialization::prefab_serializer::prefab_to_json(self)
    }

    pub fn from_json(json: &str) -> Option<Self> {
        crate::serialization::prefab_serializer::prefab_from_json(json)
    }
}


fn default_prefab_name() -> String {
    "prefab".to_string()
}

fn default_prefab_root_entity() -> Entity {
    Entity::new("PrefabRoot")
}
