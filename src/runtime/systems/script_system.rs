use std::{collections::HashSet, path::Path};

use mlua;

use crate::{
    core::{component::{Component, Animator}, entity::Entity},
    runtime::{
        script::{
            load_script_behavior_checked, parse_event_instruction, ScriptAction, ScriptEventInstruction,
        },
        systems::audio_system,
    },
};

fn lua_stage_log(
    scene_label: Option<&str>,
    entity: &Entity,
    script_path: &str,
    stage: &str,
    kind: &str,
    message: impl std::fmt::Display,
) {
    let scene = scene_label.unwrap_or("<sem_cena>");
    eprintln!(
        "[Lua][scene={}][entity={}#{}][script={}][stage={}][kind={}] {}",
        scene,
        entity.name,
        entity.id,
        script_path,
        stage,
        kind,
        message,
    );
}

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
    pub extra_velocity: Option<(f32, f32)>,
    /// Troca de cena pedida por LuaScript (preenchido por run_lua_scripts_for_entity).
    pub lua_change_scene: Option<String>,
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
                            println!("[RS2][scene=<sem_cena>][entity={}#{}][script={}] {}", entity.name, entity.id, script.file_path, message);
                        } else {
                            println!("[RS2][scene=<sem_cena>][entity={}#{}][script={}] iniciado", entity.name, entity.id, script.file_path);
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
            // LuaScript é tratado separadamente em run_lua_scripts_for_entity,
            // que recebe input + save e é chamado depois de scan_script_behavior.
            Component::LuaScript(_) => {}
            _ => {}
        }
    }

    result
}

/// Executa todos os LuaScripts de uma entidade para um frame.
/// Aplica as mutações diretamente na entidade e retorna uma troca de cena se pedida.
/// Usa `lua_vms` como cache — uma VM por (entity_id + script_path), recriada só quando o arquivo muda.
pub fn run_lua_scripts_for_entity(
    entity: &mut Entity,
    project_root: &Path,
    scene_label: Option<&str>,
    started_scripts: &mut HashSet<String>,
    input: &crate::runtime::state::RuntimeInput,
    save_data: &mut crate::runtime::save::SaveData,
    session_state: &mut std::collections::HashMap<String, crate::runtime::save::SaveValue>,
    script_state: &mut crate::runtime::state::ScriptState,
    delta_time: f32,
    elapsed_time: f32,
    lua_vms: &mut std::collections::HashMap<String, (mlua::Lua, std::time::SystemTime)>,
    collision_entries: &[crate::runtime::systems::CollisionEntry],
    collision_names: &[String],
    collision_ids: &[String],
    previous_collision_names: &[String],
    previous_collision_ids: &[String],
    collision_enter_names: &[String],
    collision_enter_ids: &[String],
    collision_stay_names: &[String],
    collision_stay_ids: &[String],
    collision_exit_names: &[String],
    collision_exit_ids: &[String],
    colliders: &[crate::runtime::systems::collision_system::RuntimeCollider],
    tag_index: &std::collections::HashMap<String, Vec<(String, String)>>,
    pending_destroys: &mut Vec<crate::runtime::state::PendingDestroyRequest>,
    pending_spawns: &mut Vec<crate::runtime::state::PendingSpawnRequest>,
    pending_particles: &mut Vec<crate::runtime::state::RuntimeParticle>,
    emitters: &mut Vec<crate::runtime::state::RuntimeParticleEmitter>,
    current_runtime_events: &[crate::runtime::state::RuntimeEvent],
    pending_runtime_events: &mut Vec<crate::runtime::state::RuntimeEvent>,
    camera_shake: &mut Option<(f32, f32)>,
    camera_zoom: &mut Option<f32>,
    audio_runtime: &mut audio_system::AudioRuntime,
) -> Option<String> {
    use crate::runtime::lua_runtime;

    let lua_scripts: Vec<String> = entity.components.iter()
        .filter_map(|c| if let Component::LuaScript(s) = c { Some(s.file_path.clone()) } else { None })
        .collect();

    for file_path in lua_scripts {
        let full_path = match crate::runtime::script::resolve_script_path(project_root, &file_path) {
            Some(p) => p,
            None => {
                lua_stage_log(scene_label, entity, &file_path, "resolve", "file_not_found", "Arquivo não encontrado");
                continue;
            }
        };

        // Lê mtime para detectar se o arquivo mudou
        let current_mtime = std::fs::metadata(&full_path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        let vm_key = format!("vm::{}::{}", entity.id, file_path);

        // Verifica se precisa recriar a VM (arquivo mudou ou primeira vez)
        let needs_reload = lua_vms.get(&vm_key)
            .map(|(_, mtime)| *mtime != current_mtime)
            .unwrap_or(true);

        if needs_reload {
            let scope_key = crate::runtime::state::make_script_state_scope_key(&entity.id, &file_path);
            script_state.remove(&scope_key);
            started_scripts.remove(&format!("lua::{}::{}", entity.id, file_path));
            let source = match std::fs::read_to_string(&full_path) {
                Ok(s) => s,
                Err(e) => {
                    lua_stage_log(scene_label, entity, &file_path, "read", "io_error", e);
                    continue;
                }
            };
            // Cria nova VM e pré-carrega o chunk para validar sintaxe
            let lua = mlua::Lua::new();
            if let Err(e) = lua.load(&source).into_function() {
                lua_stage_log(scene_label, entity, &file_path, "syntax", "syntax_error", e);
            }
            // Armazena a fonte como global _src para reutilizar sem ler disco todo frame
            lua.globals().set("_src", source).ok();
            lua_vms.insert(vm_key.clone(), (lua, current_mtime));
        }

        // Recupera a VM do cache e a fonte armazenada nela
        let (lua, _) = match lua_vms.get(&vm_key) {
            Some(entry) => entry,
            None => continue,
        };

        let source: String = match lua.globals().get("_src") {
            Ok(s) => s,
            Err(_) => {
                lua_stage_log(scene_label, entity, &file_path, "cache", "source_missing", "Fonte não encontrada no cache");
                continue;
            }
        };

        let script_key = format!("lua::{}::{}", entity.id, file_path);
        let already_started = !started_scripts.insert(script_key);

        match lua_runtime::run_lua_script_with_vm(
            lua,
            &source,
            entity,
            input,
            save_data,
            session_state,
            script_state.get(&crate::runtime::state::make_script_state_scope_key(&entity.id, &file_path)),
            delta_time,
            elapsed_time,
            already_started,
            collision_entries,
            collision_names,
            collision_ids,
            previous_collision_names,
            previous_collision_ids,
            collision_enter_names,
            collision_enter_ids,
            collision_stay_names,
            collision_stay_ids,
            collision_exit_names,
            collision_exit_ids,
            colliders,
            tag_index,
            current_runtime_events,
            scene_label,
            Some(&file_path),
        ) {
            Ok(result) => {
                let change_scene = result.change_scene.clone();
                lua_runtime::apply_lua_result(entity, &result);
                lua_runtime::apply_save_ops(save_data, &result.save_ops);
                lua_runtime::apply_session_ops(session_state, &result.session_ops);
                let scope_key = crate::runtime::state::make_script_state_scope_key(&entity.id, &file_path);
                lua_runtime::apply_state_ops(script_state, &scope_key, &result.state_ops);
                lua_runtime::apply_event_ops(pending_runtime_events, &result.event_ops);
                for audio_op in &result.audio_ops {
                    match audio_op {
                        lua_runtime::AudioOp::Play { key, path, looped, volume } => {
                            if let Some(audio_path) = audio_system::resolve_audio_path(project_root, path) {
                                if let Err(error) = audio_runtime.play_once(key, &audio_path, *looped, *volume) {
                                    lua_stage_log(scene_label, entity, &file_path, "audio_play", "runtime_error", error);
                                }
                            } else {
                                lua_stage_log(scene_label, entity, &file_path, "audio_play", "file_not_found", path);
                            }
                        }
                        lua_runtime::AudioOp::Stop { key } => {
                            let _ = audio_runtime.stop_by_name(key);
                        }
                        lua_runtime::AudioOp::SetVolume { key, volume } => {
                            let _ = audio_runtime.set_volume_by_name(key, *volume);
                        }
                    }
                }
                for spawn in &result.spawn_entities {
                    pending_spawns.push(crate::runtime::state::PendingSpawnRequest {
                        kind: crate::runtime::state::PendingSpawnKind::Template(spawn.name.clone()),
                        x: spawn.x,
                        y: spawn.y,
                        velocity: spawn.init.velocity,
                        tags: spawn.init.tags.clone(),
                        hp: spawn.init.hp,
                        anim: spawn.init.anim.clone(),
                        request_seq: Some(spawn.request_seq),
                    });
                }
                for spawn in &result.spawn_prefabs {
                    pending_spawns.push(crate::runtime::state::PendingSpawnRequest {
                        kind: crate::runtime::state::PendingSpawnKind::PrefabPath(spawn.path.clone()),
                        x: spawn.x,
                        y: spawn.y,
                        velocity: spawn.init.velocity,
                        tags: spawn.init.tags.clone(),
                        hp: spawn.init.hp,
                        anim: spawn.init.anim.clone(),
                        request_seq: Some(spawn.request_seq),
                    });
                }
                for (x, y, vx, vy, life, r, g, b, scale) in &result.spawn_particles {
                    pending_particles.push(crate::runtime::state::RuntimeParticle {
                        x: *x, y: *y, vx: *vx, vy: *vy, life: *life, max_life: *life, color: [*r, *g, *b, 1.0], scale: *scale,
                    });
                }
                for emitter in &result.spawn_emitters {
                    emitters.push(crate::runtime::state::RuntimeParticleEmitter {
                        x: emitter.x,
                        y: emitter.y,
                        rate: emitter.rate,
                        particle_life: emitter.particle_life,
                        speed_min: emitter.speed_min,
                        speed_max: emitter.speed_max,
                        color: [emitter.color.0, emitter.color.1, emitter.color.2, 1.0],
                        scale: emitter.scale,
                        duration: emitter.duration,
                        accumulator: 0.0,
                    });
                }
                if let Some((intensity, duration)) = result.camera_shake {
                    *camera_shake = Some((intensity, duration));
                }
                if let Some(zoom) = result.camera_zoom {
                    *camera_zoom = Some(zoom);
                }
                if result.destroy_entity {
                    pending_destroys.push(crate::runtime::state::PendingDestroyRequest {
                        entity_id: entity.id.clone(),
                    });
                }
                if change_scene.is_some() {
                    return change_scene;
                }
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }

    None
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
            println!("[RS2][{}][entity={}][script={}] {}", stage, entity_name, script_path, text);
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

    // Reseta o timer quando o clip muda (fix do bug V0.8)
    if animator.prev_clip != current_clip_name {
        animator.timer = 0.0;
        animator.prev_clip = current_clip_name.to_string();
    }

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
