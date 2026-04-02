use std::{collections::HashSet, path::Path};

use crate::{core::{component::Component, entity::Entity}, runtime::script::{load_script_behavior, ScriptAction}};

pub fn scan_script_behavior(
    entity: &Entity,
    project_root: &Path,
    started_scripts: &mut HashSet<String>,
) -> (f32, f32, f32, bool, Vec<ScriptAction>, Option<f32>, bool, f32) {
    let mut script_move_x = 0.0_f32;
    let mut script_move_y = 0.0_f32;
    let mut script_rotate_speed = 0.0_f32;
    let mut should_follow_camera = false;
    let mut collision_actions = Vec::new();
    let mut gravity_scale = None;
    let mut is_static = false;
    let mut collider_half_height = 0.0_f32;

    for component in &entity.components {
        match component {
            Component::RigidBody2D(rb) => {
                gravity_scale = Some(rb.gravity_scale);
                is_static = rb.is_static;
            }
            Component::BoxCollider(collider) => {
                collider_half_height = collider.height.max(0.0) * 0.5;
            }
            Component::Script(script) => {
                if let Some(behavior) = load_script_behavior(project_root, &script.file_path) {
                    let script_key = format!("{}::{}", entity.id, script.file_path);
                    if started_scripts.insert(script_key) {
                        if let Some(message) = &behavior.start_message {
                            println!("▶ Script '{}' em '{}': {}", script.file_path, entity.name, message);
                        } else {
                            println!("▶ Script '{}' iniciado em '{}'", script.file_path, entity.name);
                        }
                    }
                    script_move_x += behavior.move_x;
                    script_move_y += behavior.move_y;
                    script_rotate_speed += behavior.rotate_speed;
                    should_follow_camera |= behavior.camera_follow;
                    collision_actions.extend(behavior.on_collision.into_iter());
                }
            }
            _ => {}
        }
    }

    (script_move_x, script_move_y, script_rotate_speed, should_follow_camera, collision_actions, gravity_scale, is_static, collider_half_height)
}
