#[path = "systems/audio_system.rs"]
pub mod audio_system;
#[path = "systems/collision_system.rs"]
pub mod collision_system;
#[path = "systems/input_system.rs"]
pub mod input_system;
#[path = "systems/movement_system.rs"]
pub mod movement_system;
#[path = "systems/physics_system.rs"]
pub mod physics_system;
#[path = "systems/render_system.rs"]
pub mod render_system;
#[path = "systems/script_system.rs"]
pub mod script_system;
#[path = "systems/ui_system.rs"]
pub mod ui_system;

use std::{collections::HashSet, path::Path};

use crate::core::{component::Component, entity::Entity};
use crate::runtime::script::ScriptAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeCommand {
    ChangeScene(String),
    ReloadScene,
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
    collision_system::collect_colliders(entities, &mut colliders);

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
        let (script_move_x, script_move_y, script_rotate_speed, should_follow_camera, collision_actions, gravity_scale, is_static, collider_half_height) =
            script_system::scan_script_behavior(entity, project_root, started_scripts);

        let my_collider_data = entity.components.iter().find_map(|component| match component {
            Component::BoxCollider(c) => Some((c.offset_x, c.offset_y, c.width, c.height)),
            _ => None,
        });
        let entity_ptr = entity as *const Entity;

        let mut collided = false;
        let old_pos = entity.transform().map(|t| (t.x, t.y)).unwrap_or((0.0, 0.0));

        movement_system::apply_script_movement(entity, delta_time, script_move_x, script_move_y, script_rotate_speed);
        if !is_static {
            physics_system::apply_gravity(entity, delta_time, gravity_scale);
        }

        if let (Some(transform), Some((off_x, off_y, width, height))) = (entity.transform_mut(), my_collider_data) {
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
                if collision_system::aabb_collision(my_rect, other_rect) {
                    collided = true;
                    transform.x = old_pos.0;
                    transform.y = old_pos.1;
                    break;
                }
            }

            physics_system::clamp_to_ground(entity, ground_y, collider_half_height);

            if should_follow_camera {
                *camera_follow_target = Some((transform.x, transform.y));
            }
        }

        if collided {
            for action in collision_actions {
                match action {
                    ScriptAction::ChangeScene(path) => return Some(RuntimeCommand::ChangeScene(path)),
                    ScriptAction::ReloadScene => return Some(RuntimeCommand::ReloadScene),
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
                if let Some(behavior) = crate::runtime::script::load_script_behavior(project_root, &script.file_path) {
                    controller_speed = controller_speed.max(behavior.player_controller_speed.abs());
                }
            }
        }

        if controller_speed > 0.0 {
            movement_system::apply_player_controller(entity, delta_time, input, controller_speed);
        }

        apply_player_controller_input(&mut entity.children, project_root, delta_time, input);
    }
}
