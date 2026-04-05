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
    elapsed_time: f32,
    project_root: &Path,
    started_scripts: &mut HashSet<String>,
    camera_follow_target: &mut Option<(f32, f32)>,
    ground_y: f32,
    save_data: &mut crate::runtime::save::SaveData,
    input: &crate::runtime::state::RuntimeInput,
) -> Option<RuntimeCommand> {
    let mut colliders = Vec::new();
    collision_system::collect_colliders(entities, &mut colliders);

    update_entities_runtime_recursive(
        entities,
        delta_time,
        elapsed_time,
        project_root,
        started_scripts,
        camera_follow_target,
        ground_y,
        &colliders,
        save_data,
        input,
    )
}

fn update_entities_runtime_recursive(
    entities: &mut [Entity],
    delta_time: f32,
    elapsed_time: f32,
    project_root: &Path,
    started_scripts: &mut HashSet<String>,
    camera_follow_target: &mut Option<(f32, f32)>,
    ground_y: f32,
    colliders: &[collision_system::RuntimeCollider],
    save_data: &mut crate::runtime::save::SaveData,
    input: &crate::runtime::state::RuntimeInput,
) -> Option<RuntimeCommand> {
    for entity in entities {
        let script_data = script_system::scan_script_behavior(entity, project_root, started_scripts);
        let entity_ptr = entity as *const Entity;
        let my_collider_data = find_entity_collider(entity);
        let old_position = entity.transform().map(|t| (t.x, t.y)).unwrap_or((0.0, 0.0));

        script_system::advance_animator(entity, delta_time);

        let (extra_x, extra_y) = script_data.extra_velocity.unwrap_or((0.0, 0.0));
        movement_system::apply_script_movement(
            entity,
            delta_time,
            script_data.move_x + extra_x,
            script_data.move_y + extra_y,
            script_data.rotate_speed,
        );

        if !script_data.is_static {
            physics_system::apply_gravity(entity, delta_time, script_data.gravity_scale);
        }

        movement_system::apply_velocity(entity, delta_time);

        // Lua scripts — executam após movimento, antes de colisão
        // (podem ajustar velocidade/posição reativamente)
        if let Some(scene_path) = script_system::run_lua_scripts_for_entity(
            entity,
            project_root,
            started_scripts,
            input,
            save_data,
            delta_time,
            elapsed_time,
        ) {
            return Some(RuntimeCommand::ChangeScene(scene_path));
        }

        let collision_state =
            handle_entity_collisions(entity, entity_ptr, my_collider_data, old_position, colliders);
        physics_system::clamp_to_ground(entity, ground_y, script_data.collider_half_height);
        apply_camera_follow(entity, script_data.should_follow_camera, camera_follow_target);

        if collision_state.collided {
            for action in script_data.collision_actions {
                match action {
                    ScriptAction::ChangeScene(path) => return Some(RuntimeCommand::ChangeScene(path)),
                    ScriptAction::ReloadScene => return Some(RuntimeCommand::ReloadScene),
                }
            }
        }

        if collision_state.triggered {
            for action in script_data.trigger_actions {
                match action {
                    ScriptAction::ChangeScene(path) => return Some(RuntimeCommand::ChangeScene(path)),
                    ScriptAction::ReloadScene => return Some(RuntimeCommand::ReloadScene),
                }
            }
        }

        if let Some(command) = update_entities_runtime_recursive(
            &mut entity.children,
            delta_time,
            elapsed_time,
            project_root,
            started_scripts,
            camera_follow_target,
            ground_y,
            colliders,
            save_data,
            input,
        ) {
            return Some(command);
        }
    }

    None
}

#[derive(Debug, Default)]
struct CollisionState {
    collided: bool,
    triggered: bool,
}

fn handle_entity_collisions(
    entity: &mut Entity,
    entity_ptr: *const Entity,
    my_collider_data: Option<(f32, f32, f32, f32, bool, u8, u8)>,
    _old_position: (f32, f32),
    colliders: &[collision_system::RuntimeCollider],
) -> CollisionState {
    let Some((off_x, off_y, width, height, is_my_trigger, my_layer, my_mask)) = my_collider_data else {
        return CollisionState::default();
    };

    let Some(pos) = entity.transform().map(|t| (t.x, t.y)) else {
        return CollisionState::default();
    };

    let my_rect = (
        pos.0 + off_x - width * 0.5,
        pos.1 + off_y - height * 0.5,
        width,
        height,
    );

    let my_col = collision_system::RuntimeCollider {
        center_x: pos.0 + off_x,
        center_y: pos.1 + off_y,
        width, height,
        is_trigger: is_my_trigger,
        layer: my_layer,
        mask: my_mask,
        entity_ptr,
    };

    let mut state = CollisionState::default();
    let mut total_mtv_x = 0.0_f32;
    let mut total_mtv_y = 0.0_f32;

    for other in colliders {
        if std::ptr::eq(entity_ptr, other.entity_ptr) {
            continue;
        }

        if !collision_system::layers_interact(&my_col, other) {
            continue;
        }

        let other_rect = (
            other.center_x - other.width * 0.5,
            other.center_y - other.height * 0.5,
            other.width,
            other.height,
        );

        let mtv = collision_system::aabb_mtv(my_rect, other_rect);
        if mtv.is_zero() {
            continue;
        }

        if is_my_trigger || other.is_trigger {
            state.triggered = true;
            continue;
        }

        total_mtv_x += mtv.x;
        total_mtv_y += mtv.y;
        state.collided = true;
    }

    if state.collided {
        if let Some(transform) = entity.transform_mut() {
            transform.x += total_mtv_x;
            transform.y += total_mtv_y;
        }

        if total_mtv_y > 0.0 {
            if let Some(vel) = entity.velocity_mut() {
                if vel.y < 0.0 { vel.y = 0.0; }
            }
            physics_system::set_grounded(entity, true);
        } else if total_mtv_y < 0.0 {
            if let Some(vel) = entity.velocity_mut() {
                if vel.y > 0.0 { vel.y = 0.0; }
            }
        }

        if total_mtv_x != 0.0 {
            if let Some(vel) = entity.velocity_mut() {
                vel.x = 0.0;
            }
        }
    }

    state
}

fn apply_camera_follow(
    entity: &Entity,
    should_follow_camera: bool,
    camera_follow_target: &mut Option<(f32, f32)>,
) {
    if !should_follow_camera {
        return;
    }

    if let Some((x, y)) = entity.transform().map(|t| (t.x, t.y)) {
        *camera_follow_target = Some((x, y));
    }
}

fn find_entity_collider(entity: &Entity) -> Option<(f32, f32, f32, f32, bool, u8, u8)> {
    entity.components.iter().find_map(|component| match component {
        Component::BoxCollider(c) => Some((c.offset_x, c.offset_y, c.width, c.height, c.is_trigger, c.layer, c.mask)),
        _ => None,
    })
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
                if let Ok(behavior) =
                    crate::runtime::script::load_script_behavior_checked(project_root, &script.file_path)
                {
                    controller_speed = controller_speed.max(behavior.player_controller_speed.abs());
                }
            }
        }

        if controller_speed > 0.0 {
            movement_system::apply_player_controller(entity, input, controller_speed);
        }

        apply_player_controller_input(&mut entity.children, project_root, delta_time, input);
    }
}

pub fn apply_audio_autoplay(
    entities: &[Entity],
    project_root: &Path,
    started_audio: &mut HashSet<String>,
    audio_runtime: &mut audio_system::AudioRuntime,
) {
    for entity in entities {
        for component in &entity.components {
            if let Component::Audio(audio) = component {
                if !audio.play_on_start {
                    continue;
                }

                let key = format!("{}::{}", entity.id, audio.file_path);
                if !started_audio.insert(key.clone()) {
                    continue;
                }

                if let Some(audio_path) = audio_system::resolve_audio_path(project_root, &audio.file_path) {
                    if let Err(error) = audio_runtime.play_once(&key, &audio_path, audio.looped, audio.volume) {
                        println!("Audio error: {}", error);
                    }
                } else {
                    println!("Audio error: file not found {}", audio.file_path);
                }
            }
        }

        apply_audio_autoplay(&entity.children, project_root, started_audio, audio_runtime);
    }
}
