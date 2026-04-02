use std::{collections::HashSet, path::Path};

use crate::engine::{component::Component, entity::Entity};
use crate::runtime::script::{load_script_behavior, ScriptAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeCommand {
    ChangeScene(String),
    ReloadScene,
}

fn aabb_collision(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

pub fn update_entities_runtime(
    entities: &mut [Entity],
    delta_time: f32,
    project_root: &Path,
    started_scripts: &mut HashSet<String>,
    camera_follow_target: &mut Option<(f32, f32)>,
    ground_y: f32,
) -> Option<RuntimeCommand> {
    let mut colliders = Vec::new();
    collect_colliders(entities, &mut colliders);

    update_entities_runtime_recursive(
        entities,
        delta_time,
        project_root,
        started_scripts,
        camera_follow_target,
        ground_y,
        &colliders,
    )
}

fn update_entities_runtime_recursive(
    entities: &mut [Entity],
    delta_time: f32,
    project_root: &Path,
    started_scripts: &mut HashSet<String>,
    camera_follow_target: &mut Option<(f32, f32)>,
    ground_y: f32,
    colliders: &[(f32, f32, f32, f32, *const Entity)],
) -> Option<RuntimeCommand> {
    for entity in entities {
        let mut gravity_scale = None;
        let mut is_static = false;
        let mut collider_half_height = 0.0_f32;
        let mut script_move_x = 0.0_f32;
        let mut script_move_y = 0.0_f32;
        let mut script_rotate_speed = 0.0_f32;
        let mut should_follow_camera = false;
        let mut collision_actions = Vec::new();

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

        let my_collider_data = entity.components.iter().find_map(|component| match component {
            Component::BoxCollider(c) => Some((c.offset_x, c.offset_y, c.width, c.height)),
            _ => None,
        });
        let entity_ptr = entity as *const Entity;

        if let Some(transform) = entity.transform_mut() {
            let old_x = transform.x;
            let old_y = transform.y;
            let mut collided = false;

            transform.x += script_move_x * delta_time;
            transform.y += script_move_y * delta_time;
            transform.rotation += script_rotate_speed * delta_time;

            if !is_static {
                if let Some(gravity) = gravity_scale {
                    transform.y -= 180.0 * gravity.max(0.0) * delta_time;
                }
            }

            if let Some((off_x, off_y, width, height)) = my_collider_data {
                let my_rect = (
                    transform.x + off_x - width * 0.5,
                    transform.y + off_y - height * 0.5,
                    width,
                    height,
                );
                for (ox, oy, ow, oh, other_ptr) in colliders {
                    if std::ptr::eq(entity_ptr, *other_ptr) {
                        continue;
                    }
                    let other_rect = (*ox - *ow * 0.5, *oy - *oh * 0.5, *ow, *oh);
                    if aabb_collision(my_rect, other_rect) {
                        collided = true;
                        transform.x = old_x;
                        transform.y = old_y;
                        break;
                    }
                }
            }

            let floor_y = ground_y + collider_half_height.max(16.0);
            if transform.y < floor_y {
                transform.y = floor_y;
            }

            if should_follow_camera {
                *camera_follow_target = Some((transform.x, transform.y));
            }

            if collided {
                for action in collision_actions {
                    match action {
                        ScriptAction::ChangeScene(path) => return Some(RuntimeCommand::ChangeScene(path)),
                        ScriptAction::ReloadScene => return Some(RuntimeCommand::ReloadScene),
                    }
                }
            }
        }

        if let Some(command) = update_entities_runtime_recursive(
            &mut entity.children,
            delta_time,
            project_root,
            started_scripts,
            camera_follow_target,
            ground_y,
            colliders,
        ) {
            return Some(command);
        }
    }

    None
}

pub fn apply_player_controller_input(
    entities: &mut [Entity],
    project_root: &Path,
    delta_time: f32,
    input: eframe::egui::Vec2,
) {
    for entity in entities {
        let mut controller_speed = 0.0_f32;

        for component in &entity.components {
            if let Component::Script(script) = component {
                if let Some(behavior) = load_script_behavior(project_root, &script.file_path) {
                    controller_speed = controller_speed.max(behavior.player_controller_speed.abs());
                }
            }
        }

        if controller_speed > 0.0 {
            if let Some(transform) = entity.transform_mut() {
                transform.x += input.x * controller_speed * delta_time;
                transform.y += input.y * controller_speed * delta_time;
            }
        }

        apply_player_controller_input(&mut entity.children, project_root, delta_time, input);
    }
}

fn collect_colliders<'a>(entities: &'a [Entity], out: &mut Vec<(f32, f32, f32, f32, *const Entity)>) {
    for entity in entities {
        let mut transform = None;
        let mut collider = None;

        for component in &entity.components {
            match component {
                Component::Transform(t) => transform = Some(t),
                Component::BoxCollider(c) => collider = Some(c),
                _ => {}
            }
        }

        if let (Some(t), Some(c)) = (transform, collider) {
            out.push((
                t.x + c.offset_x,
                t.y + c.offset_y,
                c.width,
                c.height,
                entity as *const Entity,
            ));
        }

        collect_colliders(&entity.children, out);
    }
}
