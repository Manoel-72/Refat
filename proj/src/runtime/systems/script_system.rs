use std::{collections::HashSet, path::Path};

use crate::{
    core::{component::{Component, Animator}, entity::Entity},
    runtime::script::{
        load_script_behavior_checked, parse_event_instruction, ScriptAction, ScriptEventInstruction,
    },
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
                    let is_first_start = started_scripts.insert(script_key);

                    if is_first_start {
                        if let Some(message) = &behavior.start_message {
                            println!("▶ Script '{}' em '{}': {}", script.file_path, entity.name, message);
                        } else {
                            println!("▶ Script '{}' iniciado em '{}'", script.file_path, entity.name);
                        }

                        for event in &behavior.on_start {
                            execute_event_instruction(event, &entity.name, &script.file_path, &mut result, true);
                        }
                    }

                    for event in &behavior.on_update {
                        execute_event_instruction(event, &entity.name, &script.file_path, &mut result, false);
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

fn execute_event_instruction(
    event: &str,
    entity_name: &str,
    script_path: &str,
    result: &mut ScriptScanResult,
    is_start: bool,
) {
    let Some(instruction) = parse_event_instruction(event) else {
        return;
    };

    match instruction {
        ScriptEventInstruction::Print(text) => {
            let stage = if is_start { "on_start" } else { "on_update" };
            println!("▶ {} '{}' em '{}': {}", stage, script_path, entity_name, text);
        }
        ScriptEventInstruction::MoveX(value) => {
            result.move_x += value;
        }
        ScriptEventInstruction::MoveY(value) => {
            result.move_y += value;
        }
    }
}


pub fn advance_animator(entity: &mut Entity, delta_time: f32) {
    let next_frame = next_animator_frame(entity, delta_time);
    let Some(next_frame) = next_frame else {
        return;
    };

    for component in &mut entity.components {
        if let Component::Sprite(sprite) = component {
            sprite.texture_path = next_frame.clone();
            break;
        }
    }
}

fn next_animator_frame(entity: &mut Entity, delta_time: f32) -> Option<String> {
    for component in &mut entity.components {
        if let Component::Animator(animator) = component {
            return compute_next_animation_frame(animator, delta_time);
        }
    }
    None
}

fn compute_next_animation_frame(animator: &mut Animator, delta_time: f32) -> Option<String> {
    if !animator.playing {
        return None;
    }

    let current_clip_name = if animator.current.trim().is_empty() { "idle" } else { animator.current.trim() };
    let clip = animator.clips.get(current_clip_name)?;

    if clip.frames.is_empty() || clip.fps <= f32::EPSILON {
        return None;
    }

    animator.timer += delta_time.max(0.0);
    let frame_count = clip.frames.len();
    let mut frame_index = (animator.timer * clip.fps).floor() as usize;

    if animator.looped {
        frame_index %= frame_count;
    } else {
        frame_index = frame_index.min(frame_count.saturating_sub(1));
    }

    clip.frames.get(frame_index).cloned()
}
