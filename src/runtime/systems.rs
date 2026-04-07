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
    scene_label: Option<&str>,
    started_scripts: &mut HashSet<String>,
    camera_follow_target: &mut Option<(f32, f32)>,
    ground_y: f32,
    save_data: &mut crate::runtime::save::SaveData,
    session_state: &mut std::collections::HashMap<String, crate::runtime::save::SaveValue>,
    script_state: &mut crate::runtime::state::ScriptState,
    input: &crate::runtime::state::RuntimeInput,
    lua_vms: &mut std::collections::HashMap<String, (mlua::Lua, std::time::SystemTime)>,
    collision_contacts: &mut std::collections::HashMap<String, Vec<String>>,
    collision_contact_ids: &mut std::collections::HashMap<String, Vec<String>>,
    previous_collision_contacts: &std::collections::HashMap<String, Vec<String>>,
    previous_collision_contact_ids: &std::collections::HashMap<String, Vec<String>>,
    pending_destroys: &mut Vec<crate::runtime::state::PendingDestroyRequest>,
    current_runtime_events: &[crate::runtime::state::RuntimeEvent],
    pending_runtime_events: &mut Vec<crate::runtime::state::RuntimeEvent>,
) -> Option<RuntimeCommand> {
    let mut colliders = Vec::new();
    collision_system::collect_colliders(entities, &mut colliders);

    update_entities_runtime_recursive(
        entities,
        delta_time,
        elapsed_time,
        project_root,
        scene_label,
        started_scripts,
        camera_follow_target,
        ground_y,
        &colliders,
        save_data,
        session_state,
        script_state,
        input,
        lua_vms,
        collision_contacts,
        collision_contact_ids,
        previous_collision_contacts,
        previous_collision_contact_ids,
        pending_destroys,
        current_runtime_events,
        pending_runtime_events,
    )
}

fn update_entities_runtime_recursive(
    entities: &mut [Entity],
    delta_time: f32,
    elapsed_time: f32,
    project_root: &Path,
    scene_label: Option<&str>,
    started_scripts: &mut HashSet<String>,
    camera_follow_target: &mut Option<(f32, f32)>,
    ground_y: f32,
    colliders: &[collision_system::RuntimeCollider],
    save_data: &mut crate::runtime::save::SaveData,
    session_state: &mut std::collections::HashMap<String, crate::runtime::save::SaveValue>,
    script_state: &mut crate::runtime::state::ScriptState,
    input: &crate::runtime::state::RuntimeInput,
    lua_vms: &mut std::collections::HashMap<String, (mlua::Lua, std::time::SystemTime)>,
    collision_contacts: &mut std::collections::HashMap<String, Vec<String>>,
    collision_contact_ids: &mut std::collections::HashMap<String, Vec<String>>,
    previous_collision_contacts: &std::collections::HashMap<String, Vec<String>>,
    previous_collision_contact_ids: &std::collections::HashMap<String, Vec<String>>,
    pending_destroys: &mut Vec<crate::runtime::state::PendingDestroyRequest>,
    current_runtime_events: &[crate::runtime::state::RuntimeEvent],
    pending_runtime_events: &mut Vec<crate::runtime::state::RuntimeEvent>,
) -> Option<RuntimeCommand> {
    for entity in entities {
        let script_data = script_system::scan_script_behavior(entity, project_root, started_scripts);
        let entity_ptr = entity as *const Entity;
        let my_collider_data = find_entity_collider(entity);
        let old_position = entity.transform().map(|t| (t.x, t.y)).unwrap_or((0.0, 0.0));

        // Salva grounded ANTES de apply_gravity, que o reseta para false.
        // Assim o Lua lê o valor correto do frame anterior (estava no chão = pode pular).
        let grounded_before_physics = entity.components.iter()
            .find_map(|c| if let crate::core::component::Component::RigidBody2D(rb) = c { Some(rb.grounded) } else { None })
            .unwrap_or(false);

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
        let collision_entries = collect_collision_entries(entity, entity_ptr, &my_collider_data, colliders);
        let collision_names: Vec<String> = collision_entries.iter().map(|entry| entry.name.clone()).collect();
        let collision_ids: Vec<String> = collision_entries.iter().map(|entry| entry.id.clone()).collect();
        let previous_collision_names = previous_collision_contacts.get(&entity.id).cloned().unwrap_or_default();
        let previous_collision_ids = previous_collision_contact_ids.get(&entity.id).cloned().unwrap_or_default();
        collision_contacts.insert(entity.id.clone(), collision_names.clone());
        collision_contact_ids.insert(entity.id.clone(), collision_ids.clone());

        let current_collision_set: std::collections::HashSet<String> = collision_names.iter().cloned().collect();
        let previous_collision_set: std::collections::HashSet<String> = previous_collision_names.iter().cloned().collect();
        let current_collision_id_set: std::collections::HashSet<String> = collision_ids.iter().cloned().collect();
        let previous_collision_id_set: std::collections::HashSet<String> = previous_collision_ids.iter().cloned().collect();

        let mut collision_enter_names: Vec<String> = current_collision_set.difference(&previous_collision_set).cloned().collect();
        let mut collision_stay_names: Vec<String> = current_collision_set.intersection(&previous_collision_set).cloned().collect();
        let mut collision_exit_names: Vec<String> = previous_collision_set.difference(&current_collision_set).cloned().collect();
        let mut collision_enter_ids: Vec<String> = current_collision_id_set.difference(&previous_collision_id_set).cloned().collect();
        let mut collision_stay_ids: Vec<String> = current_collision_id_set.intersection(&previous_collision_id_set).cloned().collect();
        let mut collision_exit_ids: Vec<String> = previous_collision_id_set.difference(&current_collision_id_set).cloned().collect();

        collision_enter_names.sort();
        collision_stay_names.sort();
        collision_exit_names.sort();
        collision_enter_ids.sort();
        collision_stay_ids.sort();
        collision_exit_ids.sort();

        // Restaura grounded para o valor correto antes de rodar o Lua.
        // apply_gravity() zera grounded como efeito colateral — mas o Lua
        // precisa saber se a entidade estava no chão no frame anterior para
        // permitir o pulo (Space). Sem isso, entity.grounded é sempre false.
        physics_system::set_grounded(entity, grounded_before_physics);

        if let Some(scene_path) = script_system::run_lua_scripts_for_entity(
            entity,
            project_root,
            scene_label,
            started_scripts,
            input,
            save_data,
            session_state,
            script_state,
            delta_time,
            elapsed_time,
            lua_vms,
            &collision_entries,
            &collision_names,
            &collision_ids,
            &previous_collision_names,
            &previous_collision_ids,
            &collision_enter_names,
            &collision_enter_ids,
            &collision_stay_names,
            &collision_stay_ids,
            &collision_exit_names,
            &collision_exit_ids,
            colliders,
            pending_destroys,
            current_runtime_events,
            pending_runtime_events,
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
            scene_label,
            started_scripts,
            camera_follow_target,
            ground_y,
            colliders,
            save_data,
            session_state,
            script_state,
            input,
            lua_vms,
            collision_contacts,
            collision_contact_ids,
            previous_collision_contacts,
            previous_collision_contact_ids,
            pending_destroys,
            current_runtime_events,
            pending_runtime_events,
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
    my_collider_data: Option<(f32, f32, f32, f32, bool, bool, u8, u8)>,
    _old_position: (f32, f32),
    colliders: &[collision_system::RuntimeCollider],
) -> CollisionState {
    const GROUND_EPSILON: f32 = 0.001;

    let Some((off_x, off_y, width, height, is_my_trigger, my_collision_enabled, my_layer, my_mask)) = my_collider_data else {
        return CollisionState::default();
    };
    if !my_collision_enabled {
        return CollisionState::default();
    }

    let Some(pos) = entity.transform().map(|t| (t.x, t.y)) else {
        return CollisionState::default();
    };

    let falling_or_idle = entity
        .velocity()
        .map(|v| v.y <= 0.0)
        .unwrap_or(true);

    let my_rect = (
        pos.0 + off_x - width * 0.5,
        pos.1 + off_y - height * 0.5,
        width,
        height,
    );

    let my_col = collision_system::RuntimeCollider {
        center_x: pos.0 + off_x,
        center_y: pos.1 + off_y,
        width,
        height,
        is_trigger: is_my_trigger,
        collision_enabled: my_collision_enabled,
        layer: my_layer,
        mask: my_mask,
        entity_ptr,
        entity_id: entity.id.clone(),
        entity_name: entity.name.clone(),
    };

    let mut state = CollisionState::default();
    let mut total_mtv_x = 0.0_f32;
    let mut total_mtv_y = 0.0_f32;
    let mut touched_ground = false;
    let mut hit_ceiling = false;

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

        if mtv.y > GROUND_EPSILON && falling_or_idle {
            touched_ground = true;
        } else if mtv.y < -GROUND_EPSILON {
            hit_ceiling = true;
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

        if touched_ground {
            if let Some(vel) = entity.velocity_mut() {
                if vel.y < 0.0 { vel.y = 0.0; }
            }
            physics_system::set_grounded(entity, true);
        } else {
            physics_system::set_grounded(entity, false);

            if hit_ceiling {
                if let Some(vel) = entity.velocity_mut() {
                    if vel.y > 0.0 { vel.y = 0.0; }
                }
            }
        }

        if total_mtv_x.abs() > GROUND_EPSILON {
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

fn find_entity_collider(entity: &Entity) -> Option<(f32, f32, f32, f32, bool, bool, u8, u8)> {
    entity.components.iter().find_map(|component| match component {
        Component::BoxCollider(c) if c.collision_enabled => Some((c.offset_x, c.offset_y, c.width, c.height, c.is_trigger, c.collision_enabled, c.layer, c.mask)),
        _ => None,
    })
}

#[derive(Debug, Clone)]
pub struct CollisionEntry {
    pub id: String,
    pub name: String,
}

fn collect_collision_entries(
    entity: &Entity,
    entity_ptr: *const Entity,
    my_collider_data: &Option<(f32, f32, f32, f32, bool, bool, u8, u8)>,
    colliders: &[collision_system::RuntimeCollider],
) -> Vec<CollisionEntry> {
    let Some((off_x, off_y, width, height, is_my_trigger, my_collision_enabled, my_layer, my_mask)) = *my_collider_data else {
        return Vec::new();
    };
    let Some((x, y)) = entity.transform().map(|t| (t.x, t.y)) else {
        return Vec::new();
    };
    let center_x = x + off_x;
    let center_y = y + off_y;
    let my_rect = (center_x - width * 0.5, center_y - height * 0.5, width, height);
    let my_col = collision_system::RuntimeCollider {
        center_x,
        center_y,
        width,
        height,
        is_trigger: is_my_trigger,
        collision_enabled: my_collision_enabled,
        layer: my_layer,
        mask: my_mask,
        entity_ptr,
        entity_id: entity.id.clone(),
        entity_name: entity.name.clone(),
    };
    let mut entries = Vec::new();
    for other in colliders {
        if std::ptr::eq(entity_ptr, other.entity_ptr) { continue; }
        if !collision_system::layers_interact(&my_col, other) { continue; }
        let other_rect = (other.center_x - other.width * 0.5, other.center_y - other.height * 0.5, other.width, other.height);
        if !collision_system::aabb_mtv(my_rect, other_rect).is_zero() {
            entries.push(CollisionEntry {
                id: other.entity_id.clone(),
                name: if other.entity_name.trim().is_empty() { other.entity_id.clone() } else { other.entity_name.clone() },
            });
        }
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.name.cmp(&b.name)));
    entries.dedup_by(|a, b| a.id == b.id);
    entries
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
