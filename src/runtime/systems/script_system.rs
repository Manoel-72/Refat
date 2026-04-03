use std::{collections::HashSet, path::Path};

use crate::{
    core::{component::Component, entity::Entity},
    runtime::script::{load_script_behavior_checked, ScriptAction},
};

#[derive(Debug, Default)]
pub struct ScriptScanResult {
    pub move_x: f32,
    pub move_y: f32,
    pub rotate_speed: f32,
    pub should_follow_camera: bool,
    pub collision_actions: Vec<ScriptAction>,
    pub trigger_actions: Vec<ScriptAction>,
    pub gravity_scale: Option<f32>,
    pub is_static: bool,
    pub collider_half_height: f32,
}

pub fn scan_script_behavior(
    entity: &Entity,
    project_root: &Path,
    started_scripts: &mut HashSet<String>,
) -> ScriptScanResult {
    let mut result = ScriptScanResult::default();

    for component in &entity.components {
        match component {
            Component::RigidBody2D(rb) => {
                result.gravity_scale = Some(rb.gravity_scale);
                result.is_static = rb.is_static;
            }
            Component::BoxCollider(collider) => {
                result.collider_half_height = collider.height.max(0.0) * 0.5;
            }
            Component::Script(script) => {
                if let Ok(behavior) = load_script_behavior_checked(project_root, &script.file_path) {
                    let script_key = format!("{}::{}", entity.id, script.file_path);
                    if started_scripts.insert(script_key) {
                        if let Some(message) = &behavior.start_message {
                            println!("▶ Script '{}' em '{}': {}", script.file_path, entity.name, message);
                        } else {
                            println!("▶ Script '{}' iniciado em '{}'", script.file_path, entity.name);
                        }

                        for event in &behavior.on_start {
                            println!("▶ on_start '{}' em '{}': {}", script.file_path, entity.name, event);
                        }
                    }

                    result.move_x += behavior.move_x;
                    result.move_y += behavior.move_y;
                    result.rotate_speed += behavior.rotate_speed;
                    result.should_follow_camera |= behavior.camera_follow;
                    result.collision_actions.extend(behavior.on_collision.into_iter());
                    result.trigger_actions.extend(behavior.on_trigger.into_iter());
                }
            }
            _ => {}
        }
    }

    result
}
