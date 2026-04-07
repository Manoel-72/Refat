// ============================================================
//  runtime/lua_runtime.rs  —  V0.9
//  Execução de scripts Lua (.lua) por entidade.
//
//  API exposta ao Lua:
//    entity.x, entity.y, entity.rotation, entity.scale_x, entity.scale_y
//    entity.vx, entity.vy, entity.grounded, entity.visible, entity.name, entity.id
//    entity.set_position(x, y)
//    entity.set_velocity(vx, vy)
//    entity.set_rotation(r)
//    entity.set_visible(bool), entity.is_visible()
//    entity.collision_enabled, entity.set_collision_enabled(bool), entity.is_collision_enabled()
//    entity.play_anim(clip_name)
//    input.key_held(name), input.key_pressed(name), input.mouse_pos()
//    game.delta_time(), game.elapsed_time(), game.log(msg)
//    game.change_scene(path)
//    game.get_collisions() → lista de nomes das entidades em contato
//    game.collision_enter(name) → bool, true apenas no frame de entrada
//    game.collision_stay(name)  → bool, true enquanto continuar em contato
//    game.collision_exit(name)  → bool, true apenas no frame de saída
//    game.collision_enter_id(id), game.collision_stay_id(id), game.collision_exit_id(id)
//    game.get_current_collision_ids(), game.get_current_collision_info()
//    game.raycast(ox,oy,dx,dy,dist) → {hit, x, y, dist, name} ou nil
//    entity.id, entity.apply_impulse(ix, iy), entity.destroy()
//    save.set(key, value), save.get(key), save.has(key), save.remove(key)
// ============================================================

use std::collections::{HashMap, HashSet};

use mlua::{Function, Lua, Table, Value as LuaValue, Variadic};

use crate::{
    core::{component::Component, entity::Entity},
    runtime::{
        input::key_code::KeyCode,
        save::{SaveData, SaveValue},
        state::RuntimeInput,
    },
};

// ── contexto que o script pode modificar ─────────────────────

/// Resultado de executar um script Lua num frame.
#[derive(Debug, Default)]
pub struct LuaScriptResult {
    /// Posição alvo (se o script chamou entity.set_position).
    pub set_position: Option<(f32, f32)>,
    pub set_velocity: Option<(f32, f32)>,
    pub set_rotation: Option<f32>,
    pub set_visible: Option<bool>,
    pub set_collision_enabled: Option<bool>,
    pub play_anim: Option<String>,
    pub set_text: Option<String>,
    pub set_color: Option<(f32, f32, f32, f32)>,
    pub set_hp: Option<f32>,
    pub damage: Option<f32>,
    pub heal: Option<f32>,
    pub set_flip_x: Option<bool>,
    pub add_tags: Vec<String>,
    pub spawn_entities: Vec<(String, f32, f32)>,
    pub spawn_prefabs: Vec<(String, f32, f32)>,
    pub spawn_particles: Vec<(f32, f32, f32, f32, f32, f32, f32, f32, f32)>,
    pub camera_shake: Option<(f32, f32)>,
    pub camera_zoom: Option<f32>,
    pub change_scene: Option<String>,
    pub apply_impulse: Option<(f32, f32)>,
    pub destroy_entity: bool,
    pub save_ops: Vec<SaveOp>,
    pub session_ops: Vec<SessionOp>,
    pub state_ops: Vec<StateOp>,
    pub event_ops: Vec<EventOp>,
}

#[derive(Debug)]
pub enum SaveOp {
    Set(String, crate::runtime::save::SaveValue),
    Remove(String),
}

#[derive(Debug)]
pub enum SessionOp {
    Set(String, crate::runtime::save::SaveValue),
    Remove(String),
    Clear,
}

#[derive(Debug)]
pub enum StateOp {
    Set(String, crate::runtime::save::SaveValue),
    Remove(String),
    Clear,
}

#[derive(Debug)]
pub enum EventOp {
    Emit {
        name: String,
        data: Option<crate::runtime::save::SaveValue>,
    },
}

// ── execução ─────────────────────────────────────────────────

/// Executa o `on_update(dt)` de um script Lua para a entidade dada.
/// Retorna as mutações que o script quer aplicar.
///
/// `started` controla se `on_start()` já foi chamado para este script.
/// `collision_names` lista as entidades atualmente colidindo com esta.
pub fn run_lua_script(
    lua_source: &str,
    entity: &Entity,
    input: &RuntimeInput,
    save_data: &SaveData,
    session_data: &std::collections::HashMap<String, SaveValue>,
    script_data: Option<&std::collections::HashMap<String, SaveValue>>,
    delta_time: f32,
    elapsed_time: f32,
    started: bool,
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
    current_runtime_events: &[crate::runtime::state::RuntimeEvent],
    scene_label: Option<&str>,
    script_path: Option<&str>,
) -> Result<LuaScriptResult, String> {
    let lua = Lua::new();
    run_lua_script_with_vm(
        &lua,
        lua_source,
        entity,
        input,
        save_data,
        session_data,
        script_data,
        delta_time,
        elapsed_time,
        started,
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
        &[],
        &HashMap::new(),
        current_runtime_events,
        scene_label,
        script_path,
    )
}

/// Versão interna que recebe uma VM Lua já existente (reutilizada do cache).
pub fn run_lua_script_with_vm(
    lua: &Lua,
    lua_source: &str,
    entity: &Entity,
    input: &RuntimeInput,
    save_data: &SaveData,
    session_data: &std::collections::HashMap<String, SaveValue>,
    script_data: Option<&std::collections::HashMap<String, SaveValue>>,
    delta_time: f32,
    elapsed_time: f32,
    started: bool,
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
    tag_index: &HashMap<String, Vec<(String, String)>>,
    current_runtime_events: &[crate::runtime::state::RuntimeEvent],
    scene_label: Option<&str>,
    script_path: Option<&str>,
) -> Result<LuaScriptResult, String> {
    let mut result = LuaScriptResult::default();

    fn format_lua_context(entity: &Entity, scene_label: Option<&str>, script_path: Option<&str>) -> String {
        let scene = scene_label.unwrap_or("<sem_cena>");
        let script = script_path.unwrap_or("<script_lua>");
        format!("[Lua][scene={}][entity={}#{}][script={}]", scene, entity.name, entity.id, script)
    }

    fn stage_error(stage: &str, err: impl std::fmt::Display, entity: &Entity, scene_label: Option<&str>, script_path: Option<&str>) -> String {
        format!("{}[stage={}][kind=runtime_error] {}", format_lua_context(entity, scene_label, script_path), stage, err)
    }

    fn lua_value_to_log_string(value: &LuaValue) -> String {
        match value {
            LuaValue::Nil => "nil".to_string(),
            LuaValue::Boolean(v) => v.to_string(),
            LuaValue::Integer(v) => v.to_string(),
            LuaValue::Number(v) => v.to_string(),
            LuaValue::String(v) => v.to_string_lossy().to_string(),
            LuaValue::Table(_) => "<table>".to_string(),
            LuaValue::Function(_) => "<function>".to_string(),
            LuaValue::Thread(_) => "<thread>".to_string(),
            LuaValue::UserData(_) => "<userdata>".to_string(),
            LuaValue::LightUserData(_) => "<light_userdata>".to_string(),
            LuaValue::Error(err) => format!("<error:{}>", err),
            LuaValue::Other(_) => "<other>".to_string(),
        }
    }

    fn save_value_to_lua(lua: &Lua, value: &SaveValue) -> Result<LuaValue, String> {
        match value {
            SaveValue::Bool(b) => Ok(LuaValue::Boolean(*b)),
            SaveValue::Int(i) => Ok(LuaValue::Integer(*i)),
            SaveValue::Float(f) => Ok(LuaValue::Number(*f)),
            SaveValue::Text(s) => lua.create_string(s).map(LuaValue::String).map_err(|e| e.to_string()),
        }
    }


    fn lua_value_to_save_value(value: LuaValue) -> Result<Option<SaveValue>, mlua::Error> {
        match value {
            LuaValue::Nil => Ok(None),
            LuaValue::Boolean(v) => Ok(Some(SaveValue::Bool(v))),
            LuaValue::Integer(v) => Ok(Some(SaveValue::Int(v))),
            LuaValue::Number(v) => Ok(Some(SaveValue::Float(v))),
            LuaValue::String(v) => Ok(Some(SaveValue::Text(v.to_string_lossy().to_string()))),
            _ => Err(mlua::Error::runtime("Apenas nil, bool, integer, number e string são suportados")),
        }
    }

    fn set_scalar_cmd(cmds: &Table, prefix: &str, key: &str, value: LuaValue) -> Result<(), mlua::Error> {
        match lua_value_to_save_value(value)? {
            None => Ok(()),
            Some(SaveValue::Bool(v)) => cmds.set(format!("b:{}:{}", prefix, key), v),
            Some(SaveValue::Int(v)) => cmds.set(format!("i:{}:{}", prefix, key), v),
            Some(SaveValue::Float(v)) => cmds.set(format!("f:{}:{}", prefix, key), v),
            Some(SaveValue::Text(v)) => cmds.set(format!("s:{}:{}", prefix, key), v),
        }
    }

    fn dispatch_runtime_events(
        lua: &Lua,
        entity: &Entity,
        scene_label: Option<&str>,
        script_path: Option<&str>,
        current_runtime_events: &[crate::runtime::state::RuntimeEvent],
    ) -> Result<(), String> {
        let globals = lua.globals();
        let listeners: Table = match globals.get("__rs2_event_listeners") {
            Ok(table) => table,
            Err(_) => return Ok(()),
        };

        for event in current_runtime_events {
            let maybe_listeners: LuaValue = listeners.get(event.name.as_str()).map_err(|e| e.to_string())?;
            let payload = match &event.data {
                Some(value) => save_value_to_lua(lua, value).map_err(|e| stage_error("event_dispatch", e, entity, scene_label, script_path))?,
                None => LuaValue::Nil,
            };
            match maybe_listeners {
                LuaValue::Function(listener) => {
                    if let Err(err) = listener.call::<()>(payload.clone()) {
                        eprintln!("{}", stage_error("event_dispatch", err, entity, scene_label, script_path));
                    }
                }
                LuaValue::Table(entries) => {
                    for pair in entries.sequence_values::<Function>() {
                        let Ok(listener) = pair else { continue };
                        if let Err(err) = listener.call::<()>(payload.clone()) {
                            eprintln!("{}", stage_error("event_dispatch", err, entity, scene_label, script_path));
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn advance_timers(
        lua: &Lua,
        entity: &Entity,
        scene_label: Option<&str>,
        script_path: Option<&str>,
        delta_time: f32,
    ) -> Result<(), String> {
        let globals = lua.globals();
        let timers: Table = match globals.get("__rs2_timers") {
            Ok(table) => table,
            Err(_) => return Ok(()),
        };

        let mut due_callbacks: Vec<(bool, f32, Function)> = Vec::new();
        let mut to_remove: Vec<i64> = Vec::new();

        for pair in timers.pairs::<i64, Table>() {
            let Ok((timer_id, timer)) = pair else { continue };
            let interval = timer.get::<f32>("interval").unwrap_or(0.0).max(0.0);
            let repeating = timer.get::<bool>("repeating").unwrap_or(false);
            let remaining = timer.get::<f32>("remaining").unwrap_or(interval) - delta_time;
            if remaining <= 0.0 {
                if let Ok(callback) = timer.get::<Function>("callback") {
                    due_callbacks.push((repeating, interval, callback));
                }
                if repeating {
                    timer.set("remaining", (remaining + interval).max(0.0)).map_err(|e| e.to_string())?;
                } else {
                    to_remove.push(timer_id);
                }
            } else {
                timer.set("remaining", remaining).map_err(|e| e.to_string())?;
            }
        }

        for timer_id in to_remove {
            timers.raw_remove(timer_id).map_err(|e| e.to_string())?;
        }

        for (_repeating, _interval, callback) in due_callbacks {
            if let Err(err) = callback.call::<()>(()) {
                eprintln!("{}", stage_error("timer_callback", err, entity, scene_label, script_path));
            }
        }

        Ok(())
    }

    fn make_vec2_callable(lua: &Lua, x: f32, y: f32) -> Result<Table, String> {
        let wrapper = lua.create_table().map_err(|e| e.to_string())?;
        wrapper.set(1, x).map_err(|e| e.to_string())?;
        wrapper.set(2, y).map_err(|e| e.to_string())?;
        wrapper.set("x", x).map_err(|e| e.to_string())?;
        wrapper.set("y", y).map_err(|e| e.to_string())?;

        let mt = lua.create_table().map_err(|e| e.to_string())?;
        let call_x = x;
        let call_y = y;
        let call_fn = lua.create_function(move |lua_ctx, ()| {
            let t = lua_ctx.create_table()?;
            t.set(1, call_x)?;
            t.set(2, call_y)?;
            t.set("x", call_x)?;
            t.set("y", call_y)?;
            Ok(t)
        }).map_err(|e| e.to_string())?;
        mt.set("__call", call_fn).map_err(|e| e.to_string())?;
        wrapper.set_metatable(Some(mt));
        Ok(wrapper)
    }

// ── tabela `entity` ──────────────────────────────────────
    let entity_tbl = lua.create_table().map_err(|e| e.to_string())?;

    // leitura de Transform
    if let Some(t) = entity.transform() {
        entity_tbl.set("x", t.x).ok();
        entity_tbl.set("y", t.y).ok();
        entity_tbl.set("rotation", t.rotation).ok();
        entity_tbl.set("scale_x", t.scale_x).ok();
        entity_tbl.set("scale_y", t.scale_y).ok();
    }

    // leitura de Velocity
    let (vx, vy) = entity.components.iter()
        .find_map(|c| if let Component::Velocity(v) = c { Some((v.x, v.y)) } else { None })
        .unwrap_or((0.0, 0.0));
    entity_tbl.set("vx", vx).ok();
    entity_tbl.set("vy", vy).ok();

    // grounded
    let grounded = entity.components.iter()
        .find_map(|c| if let Component::RigidBody2D(rb) = c { Some(rb.grounded) } else { None })
        .unwrap_or(false);
    entity_tbl.set("grounded", grounded).ok();
    entity_tbl.set("visible", entity.visible).ok();
    let collision_enabled = entity.components.iter()
        .find_map(|c| if let Component::BoxCollider(col) = c { Some(col.collision_enabled) } else { None })
        .unwrap_or(false);
    entity_tbl.set("collision_enabled", collision_enabled).ok();
    entity_tbl.set("name", entity.name.clone()).ok();
    entity_tbl.set("id", entity.id.clone()).ok();
    entity_tbl.set("hp", entity.hp()).ok();
    entity_tbl.set("max_hp", entity.max_hp()).ok();
    entity_tbl.set("is_dead", entity.is_dead).ok();

    {
        let entity_name = entity.name.clone();
        let name_fn = lua.create_function(move |lua_ctx, ()| {
            Ok(LuaValue::String(lua_ctx.create_string(&entity_name)?))
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("get_name", name_fn).ok();
    }
    {
        let entity_id = entity.id.clone();
        let id_fn = lua.create_function(move |lua_ctx, ()| {
            Ok(LuaValue::String(lua_ctx.create_string(&entity_id)?))
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("get_id", id_fn).ok();
    }
    {
        let visible_now = entity.visible;
        let is_visible_fn = lua.create_function(move |_, ()| Ok(visible_now))
            .map_err(|e| e.to_string())?;
        entity_tbl.set("is_visible", is_visible_fn).ok();
    }
    {
        let collision_enabled_now = collision_enabled;
        let is_collision_enabled_fn = lua.create_function(move |_, ()| Ok(collision_enabled_now))
            .map_err(|e| e.to_string())?;
        entity_tbl.set("is_collision_enabled", is_collision_enabled_fn).ok();
    }

    {
        let current_hp = entity.hp();
        let hp_fn = lua.create_function(move |_, ()| Ok(current_hp)).map_err(|e| e.to_string())?;
        entity_tbl.set("get_hp", hp_fn).ok();
    }
    {
        let current_alive = entity.is_alive();
        let alive_fn = lua.create_function(move |_, ()| Ok(current_alive)).map_err(|e| e.to_string())?;
        entity_tbl.set("is_alive", alive_fn).ok();
    }
    {
        let current_anim = entity.components.iter().find_map(|c| if let Component::Animator(anim) = c { Some(anim.current.clone()) } else { None }).unwrap_or_else(|| "idle".to_string());
        let anim_fn = lua.create_function(move |lua_ctx, ()| Ok(LuaValue::String(lua_ctx.create_string(&current_anim)?))).map_err(|e| e.to_string())?;
        entity_tbl.set("get_anim", anim_fn).ok();
    }
    {
        let anim_finished = entity.components.iter().find_map(|c| if let Component::Animator(anim) = c {
            let clip_name = if anim.current.trim().is_empty() { "idle" } else { anim.current.trim() };
            let clip = anim.clips.get(clip_name);
            Some(match clip {
                Some(clip) if !anim.looped && clip.fps > f32::EPSILON && !clip.frames.is_empty() => {
                    anim.timer >= (clip.frames.len() as f32 / clip.fps)
                }
                Some(_) => false,
                None => false,
            })
        } else { None }).unwrap_or(false);
        let anim_finished_fn = lua.create_function(move |_, ()| Ok(anim_finished)).map_err(|e| e.to_string())?;
        entity_tbl.set("is_anim_finished", anim_finished_fn).ok();
    }
    // comandos de escrita — armazenados numa tabela interna _cmds
    let cmds: Table = lua.create_table().map_err(|e| e.to_string())?;
    entity_tbl.set("_cmds", cmds).ok();

    // funções de escrita
    {
        let tbl = entity_tbl.clone();
        let set_pos = lua.create_function(move |_, (x, y): (f32, f32)| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("pos_x", x)?;
            cmds.set("pos_y", y)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("set_position", set_pos).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let set_vel = lua.create_function(move |_, (vx, vy): (f32, f32)| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("vel_x", vx)?;
            cmds.set("vel_y", vy)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("set_velocity", set_vel).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let set_rot = lua.create_function(move |_, r: f32| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("rotation", r)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("set_rotation", set_rot).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let set_vis = lua.create_function(move |_, v: bool| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("visible", v)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("set_visible", set_vis).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let set_col = lua.create_function(move |_, enabled: bool| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("collision_enabled", enabled)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("set_collision_enabled", set_col).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let play_anim = lua.create_function(move |_, clip: String| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("anim", clip)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("play_anim", play_anim).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let set_text = lua.create_function(move |_, text: String| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("text", text)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("set_text", set_text).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let set_color = lua.create_function(move |_, (r, g, b, a): (f32, f32, f32, f32)| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("color_r", r)?;
            cmds.set("color_g", g)?;
            cmds.set("color_b", b)?;
            cmds.set("color_a", a)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("set_color", set_color).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let set_hp = lua.create_function(move |_, value: f32| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("set_hp", value)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("set_hp", set_hp).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let damage_fn = lua.create_function(move |_, value: f32| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("damage", value)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("damage", damage_fn).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let heal_fn = lua.create_function(move |_, value: f32| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("heal", value)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("heal", heal_fn).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let set_anim = lua.create_function(move |_, clip_name: String| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("anim", clip_name)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("set_anim", set_anim).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let flip_x = lua.create_function(move |_, enabled: bool| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("flip_x", enabled)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("flip_x", flip_x).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let add_tag = lua.create_function(move |_, tag: String| {
            let normalized = tag.trim().to_string();
            if normalized.is_empty() {
                return Ok(false);
            }
            let cmds: Table = tbl.get("_cmds")?;
            let seq = cmds.get::<i64>("tag_seq").unwrap_or(0) + 1;
            cmds.set("tag_seq", seq)?;
            cmds.set(format!("tag:{}", seq), normalized)?;
            Ok(true)
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("add_tag", add_tag).ok();
    }
    {
        let tag_set: HashSet<String> = entity.tags.iter().map(|tag| tag.to_ascii_lowercase()).collect();
        let has_tag = lua.create_function(move |_, tag: String| {
            Ok(tag_set.contains(&tag.trim().to_ascii_lowercase()))
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("has_tag", has_tag).ok();
    }
    // entity.apply_impulse(ix, iy) — acumula impulso (somado à velocidade no apply)
    {
        let tbl = entity_tbl.clone();
        let apply_impulse = lua.create_function(move |_, (ix, iy): (f32, f32)| {
            let cmds: Table = tbl.get("_cmds")?;
            let prev_ix: f32 = cmds.get("impulse_x").unwrap_or(0.0);
            let prev_iy: f32 = cmds.get("impulse_y").unwrap_or(0.0);
            cmds.set("impulse_x", prev_ix + ix)?;
            cmds.set("impulse_y", prev_iy + iy)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("apply_impulse", apply_impulse).ok();
    }
    {
        let tbl = entity_tbl.clone();
        let destroy_fn = lua.create_function(move |_, ()| {
            let cmds: Table = tbl.get("_cmds")?;
            cmds.set("destroy", true)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        entity_tbl.set("destroy", destroy_fn).ok();
    }

    lua.globals().set("entity", entity_tbl.clone()).map_err(|e| e.to_string())?;

    // ── tabela `input` ───────────────────────────────────────
    {
        let input_snap = InputSnapshot::from(input);
        let input_tbl = lua.create_table().map_err(|e| e.to_string())?;

        let snap = input_snap.clone();
        let key_held = lua.create_function(move |_, name: String| {
            Ok(snap.key_held(&name))
        }).map_err(|e| e.to_string())?;
        input_tbl.set("key_held", key_held).ok();

        let snap = input_snap.clone();
        let key_pressed = lua.create_function(move |_, name: String| {
            Ok(snap.key_pressed(&name))
        }).map_err(|e| e.to_string())?;
        input_tbl.set("key_pressed", key_pressed).ok();

        let (mx, my) = input.mouse_pos;
        let mouse_pos = make_vec2_callable(&lua, mx, my)?;
        input_tbl.set("mouse_pos", mouse_pos).ok();

        let get_mouse_pos = lua.create_function(move |lua_ctx, ()| {
            let t = lua_ctx.create_table()?;
            t.set(1, mx)?;
            t.set(2, my)?;
            t.set("x", mx)?;
            t.set("y", my)?;
            Ok(t)
        }).map_err(|e| e.to_string())?;
        input_tbl.set("get_mouse_pos", get_mouse_pos).ok();

        input_tbl.set("mouse_left",   input.mouse_left).ok();
        input_tbl.set("mouse_right",  input.mouse_right).ok();
        input_tbl.set("mouse_middle", input.mouse_middle).ok();

        lua.globals().set("input", input_tbl).map_err(|e| e.to_string())?;
    }

    // ── tabela `game` ────────────────────────────────────────
    {
        let game_tbl = lua.create_table().map_err(|e| e.to_string())?;
        let delta_time_fn = lua.create_function(move |_, ()| Ok(delta_time))
            .map_err(|e| e.to_string())?;
        game_tbl.set("delta_time", delta_time_fn).ok();
        game_tbl.set("delta_time_value", delta_time).ok();
        let get_delta_time = lua.create_function(move |_, ()| Ok(delta_time))
            .map_err(|e| e.to_string())?;
        game_tbl.set("get_delta_time", get_delta_time).ok();

        let elapsed_time_fn = lua.create_function(move |_, ()| Ok(elapsed_time))
            .map_err(|e| e.to_string())?;
        game_tbl.set("elapsed_time", elapsed_time_fn).ok();
        game_tbl.set("elapsed_time_value", elapsed_time).ok();
        let get_elapsed_time = lua.create_function(move |_, ()| Ok(elapsed_time))
            .map_err(|e| e.to_string())?;
        game_tbl.set("get_elapsed_time", get_elapsed_time).ok();

        {
            let tag_snapshot = tag_index.clone();
            let find_by_tag = lua.create_function(move |lua_ctx, tag: String| {
                let table = lua_ctx.create_table()?;
                if let Some(entries) = tag_snapshot.get(&tag.trim().to_ascii_lowercase()) {
                    for (index, (id, name)) in entries.iter().enumerate() {
                        let entry = lua_ctx.create_table()?;
                        entry.set("id", id.clone())?;
                        entry.set("name", name.clone())?;
                        table.set(index + 1, entry)?;
                    }
                }
                Ok(table)
            }).map_err(|e| e.to_string())?;
            game_tbl.set("find_by_tag", find_by_tag).ok();
        }

        {
            let tag_snapshot = tag_index.clone();
            let find_one_by_tag = lua.create_function(move |lua_ctx, tag: String| {
                if let Some(entries) = tag_snapshot.get(&tag.trim().to_ascii_lowercase()) {
                    if let Some((id, name)) = entries.first() {
                        let entry = lua_ctx.create_table()?;
                        entry.set("id", id.clone())?;
                        entry.set("name", name.clone())?;
                        return Ok(Some(entry));
                    }
                }
                Ok(None::<Table>)
            }).map_err(|e| e.to_string())?;
            game_tbl.set("find_one_by_tag", find_one_by_tag).ok();
        }

        {
            let game_tbl_clone = game_tbl.clone();
            let spawn_entity = lua.create_function(move |_, (name, x, y): (String, f32, f32)| {
                let cmds: Table = game_tbl_clone.get("_cmds")?;
                let seq = cmds.get::<i64>("spawn_entity_seq").unwrap_or(0) + 1;
                cmds.set("spawn_entity_seq", seq)?;
                cmds.set(format!("spawn_entity_name:{}", seq), name)?;
                cmds.set(format!("spawn_entity_x:{}", seq), x)?;
                cmds.set(format!("spawn_entity_y:{}", seq), y)?;
                Ok(())
            }).map_err(|e| e.to_string())?;
            game_tbl.set("spawn_entity", spawn_entity).ok();
        }

        {
            let game_tbl_clone = game_tbl.clone();
            let spawn_prefab = lua.create_function(move |_, (path, x, y): (String, f32, f32)| {
                let cmds: Table = game_tbl_clone.get("_cmds")?;
                let seq = cmds.get::<i64>("spawn_prefab_seq").unwrap_or(0) + 1;
                cmds.set("spawn_prefab_seq", seq)?;
                cmds.set(format!("spawn_prefab_path:{}", seq), path)?;
                cmds.set(format!("spawn_prefab_x:{}", seq), x)?;
                cmds.set(format!("spawn_prefab_y:{}", seq), y)?;
                Ok(())
            }).map_err(|e| e.to_string())?;
            game_tbl.set("spawn_prefab", spawn_prefab).ok();
        }

        {
            let game_tbl_clone = game_tbl.clone();
            let spawn_particle = lua.create_function(move |_, args: Variadic<LuaValue>| {
                let mut values = [0.0_f32; 9];
                let defaults = [0.0_f32, 0.0, 0.0, 24.0, 1.0, 0.95, 0.85, 0.25, 1.0];
                for (i, default) in defaults.iter().enumerate() { values[i] = *default; }
                for (i, value) in args.iter().take(9).enumerate() {
                    values[i] = match value {
                        LuaValue::Integer(v) => *v as f32,
                        LuaValue::Number(v) => *v as f32,
                        _ => values[i],
                    };
                }
                let cmds: Table = game_tbl_clone.get("_cmds")?;
                let seq = cmds.get::<i64>("spawn_particle_seq").unwrap_or(0) + 1;
                cmds.set("spawn_particle_seq", seq)?;
                for (i, value) in values.iter().enumerate() { cmds.set(format!("spawn_particle_{}:{}", i, seq), *value)?; }
                Ok(())
            }).map_err(|e| e.to_string())?;
            game_tbl.set("spawn_particle", spawn_particle).ok();
        }
        {
            let game_tbl_clone = game_tbl.clone();
            let camera_shake = lua.create_function(move |_, (intensity, duration): (f32, f32)| {
                let cmds: Table = game_tbl_clone.get("_cmds")?;
                cmds.set("camera_shake_intensity", intensity.max(0.0))?;
                cmds.set("camera_shake_duration", duration.max(0.0))?;
                Ok(())
            }).map_err(|e| e.to_string())?;
            game_tbl.set("camera_shake", camera_shake).ok();
        }
        {
            let game_tbl_clone = game_tbl.clone();
            let camera_zoom = lua.create_function(move |_, zoom: f32| {
                let cmds: Table = game_tbl_clone.get("_cmds")?;
                cmds.set("camera_zoom", zoom)?;
                Ok(())
            }).map_err(|e| e.to_string())?;
            game_tbl.set("camera_zoom", camera_zoom).ok();
        }
        // game.log/msg + aliases de severidade
        let log_prefix = format_lua_context(entity, scene_label, script_path);
        let info_prefix = format!("{}[stage=log]", log_prefix);
        let log_fn = lua.create_function(move |_, msg: String| {
            println!("{} {}", info_prefix, msg);
            Ok(())
        }).map_err(|e| e.to_string())?;
        game_tbl.set("log", log_fn).ok();

        let warn_prefix = format!("{}[stage=log][level=warn]", log_prefix);
        let warn_fn = lua.create_function(move |_, msg: String| {
            eprintln!("{} {}", warn_prefix, msg);
            Ok(())
        }).map_err(|e| e.to_string())?;
        game_tbl.set("warn", warn_fn).ok();

        let error_prefix = format!("{}[stage=log][level=error]", log_prefix);
        let error_fn = lua.create_function(move |_, msg: String| {
            eprintln!("{} {}", error_prefix, msg);
            Ok(())
        }).map_err(|e| e.to_string())?;
        game_tbl.set("error", error_fn).ok();

        // game.change_scene(path) → gravado em _cmds do game
        let game_cmds: Table = lua.create_table().map_err(|e| e.to_string())?;
        let gc = game_cmds.clone();
        let change_scene = lua.create_function(move |_, path: String| {
            gc.set("change_scene", path)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        game_tbl.set("change_scene", change_scene).ok();
        game_tbl.set("_cmds", game_cmds).ok();

        // game.get_collisions() → lista de nomes das entidades em contato
        let col_names: Vec<String> = collision_names.to_vec();
        let get_collisions = lua.create_function(move |lua_ctx, ()| {
            let t = lua_ctx.create_table()?;
            for (i, name) in col_names.iter().enumerate() {
                t.set(i + 1, name.as_str())?;
            }
            Ok(t)
        }).map_err(|e| e.to_string())?;
        game_tbl.set("get_collisions", get_collisions).ok();

        let current_cols_snapshot: Vec<String> = collision_names.to_vec();
        let current_cols_for_names = current_cols_snapshot.clone();
        let current_col_ids_snapshot: Vec<String> = collision_ids.to_vec();
        let current_col_ids_alias = current_col_ids_snapshot.clone();
        let current_col_entries: Vec<(String, String)> = collision_entries.iter().map(|entry| (entry.id.clone(), entry.name.clone())).collect();
        let previous_cols_snapshot: Vec<String> = previous_collision_names.to_vec();
        let previous_col_ids_snapshot: Vec<String> = previous_collision_ids.to_vec();
        let enter_cols: Vec<String> = collision_enter_names.to_vec();
        let enter_col_ids: Vec<String> = collision_enter_ids.to_vec();
        let stay_cols: Vec<String> = collision_stay_names.to_vec();
        let stay_col_ids: Vec<String> = collision_stay_ids.to_vec();
        let exit_cols: Vec<String> = collision_exit_names.to_vec();
        let exit_col_ids: Vec<String> = collision_exit_ids.to_vec();

        let collision_enter = lua.create_function(move |_, name: String| {
            Ok(enter_cols.iter().any(|n| n.eq_ignore_ascii_case(&name)))
        }).map_err(|e| e.to_string())?;
        game_tbl.set("collision_enter", collision_enter).ok();

        let collision_stay = lua.create_function(move |_, name: String| {
            Ok(stay_cols.iter().any(|n| n.eq_ignore_ascii_case(&name)))
        }).map_err(|e| e.to_string())?;
        game_tbl.set("collision_stay", collision_stay).ok();

        let collision_exit = lua.create_function(move |_, name: String| {
            Ok(exit_cols.iter().any(|n| n.eq_ignore_ascii_case(&name)))
        }).map_err(|e| e.to_string())?;
        game_tbl.set("collision_exit", collision_exit).ok();

        let collision_enter_id = lua.create_function(move |_, id: String| {
            Ok(enter_col_ids.iter().any(|existing| existing == &id))
        }).map_err(|e| e.to_string())?;
        game_tbl.set("collision_enter_id", collision_enter_id).ok();

        let collision_stay_id = lua.create_function(move |_, id: String| {
            Ok(stay_col_ids.iter().any(|existing| existing == &id))
        }).map_err(|e| e.to_string())?;
        game_tbl.set("collision_stay_id", collision_stay_id).ok();

        let collision_exit_id = lua.create_function(move |_, id: String| {
            Ok(exit_col_ids.iter().any(|existing| existing == &id))
        }).map_err(|e| e.to_string())?;
        game_tbl.set("collision_exit_id", collision_exit_id).ok();

        // Mantém snapshots brutos disponíveis para compatibilidade e debug.
        let get_collisions_previous = lua.create_function(move |lua_ctx, ()| {
            let t = lua_ctx.create_table()?;
            for (i, name) in previous_cols_snapshot.iter().enumerate() {
                t.set(i + 1, name.as_str())?;
            }
            Ok(t)
        }).map_err(|e| e.to_string())?;
        game_tbl.set("get_previous_collisions", get_collisions_previous).ok();

        let get_collisions_current = lua.create_function(move |lua_ctx, ()| {
            let t = lua_ctx.create_table()?;
            for (i, name) in current_cols_snapshot.iter().enumerate() {
                t.set(i + 1, name.as_str())?;
            }
            Ok(t)
        }).map_err(|e| e.to_string())?;
        game_tbl.set("get_current_collisions", get_collisions_current).ok();

        let get_current_collision_ids = lua.create_function(move |lua_ctx, ()| {
            let t = lua_ctx.create_table()?;
            for (i, id) in current_col_ids_snapshot.iter().enumerate() {
                t.set(i + 1, id.as_str())?;
            }
            Ok(t)
        }).map_err(|e| e.to_string())?;
        game_tbl.set("get_current_collision_ids", get_current_collision_ids).ok();

        let get_previous_collision_ids = lua.create_function(move |lua_ctx, ()| {
            let t = lua_ctx.create_table()?;
            for (i, id) in previous_col_ids_snapshot.iter().enumerate() {
                t.set(i + 1, id.as_str())?;
            }
            Ok(t)
        }).map_err(|e| e.to_string())?;
        game_tbl.set("get_previous_collision_ids", get_previous_collision_ids).ok();

        let get_collision_ids = lua.create_function(move |lua_ctx, ()| {
            let t = lua_ctx.create_table()?;
            for (i, id) in current_col_ids_alias.iter().enumerate() {
                t.set(i + 1, id.as_str())?;
            }
            Ok(t)
        }).map_err(|e| e.to_string())?;
        game_tbl.set("get_collision_ids", get_collision_ids).ok();

        let get_collision_names = lua.create_function(move |lua_ctx, ()| {
            let t = lua_ctx.create_table()?;
            for (i, name) in current_cols_for_names.iter().enumerate() {
                t.set(i + 1, name.as_str())?;
            }
            Ok(t)
        }).map_err(|e| e.to_string())?;
        game_tbl.set("get_collision_names", get_collision_names).ok();

        let get_current_collision_info = lua.create_function(move |lua_ctx, ()| {
            let t = lua_ctx.create_table()?;
            for (i, (id, name)) in current_col_entries.iter().enumerate() {
                let entry = lua_ctx.create_table()?;
                entry.set("id", id.as_str())?;
                entry.set("name", name.as_str())?;
                t.set(i + 1, entry)?;
            }
            Ok(t)
        }).map_err(|e| e.to_string())?;
        game_tbl.set("get_current_collision_info", get_current_collision_info).ok();

        // game.raycast(ox, oy, dx, dy, max_dist) → {hit=true, x, y, dist, name, id} | {hit=false}
        // Snapshot sem raw ptr — apenas dados geométricos + nome, seguro para closure.
        let ray_snap: Vec<(f32, f32, f32, f32, u8, String, String)> = colliders.iter()
            .map(|c| (c.center_x, c.center_y, c.width, c.height, c.layer, c.entity_name.clone(), c.entity_id.clone()))
            .collect();
        let timers: Table = lua.globals().get("__rs2_timers").unwrap_or_else(|_| lua.create_table().unwrap());
        lua.globals().set("__rs2_timers", timers.clone()).ok();
        let timer_counter: i64 = lua.globals().get("__rs2_timer_counter").unwrap_or(0);
        lua.globals().set("__rs2_timer_counter", timer_counter).ok();

        let timers_after = timers.clone();
        let after_fn = lua.create_function(move |lua_ctx, (seconds, callback): (f32, Function)| {
            let seconds = seconds.max(0.0);
            let current_id: i64 = lua_ctx.globals().get("__rs2_timer_counter").unwrap_or(0);
            let next_id = current_id + 1;
            lua_ctx.globals().set("__rs2_timer_counter", next_id)?;
            let timer = lua_ctx.create_table()?;
            timer.set("remaining", seconds)?;
            timer.set("interval", seconds)?;
            timer.set("repeating", false)?;
            timer.set("callback", callback)?;
            timers_after.set(next_id, timer)?;
            Ok(next_id)
        }).map_err(|e| e.to_string())?;
        game_tbl.set("after", after_fn).ok();

        let timers_every = timers.clone();
        let every_fn = lua.create_function(move |lua_ctx, (seconds, callback): (f32, Function)| {
            let seconds = seconds.max(0.0);
            let current_id: i64 = lua_ctx.globals().get("__rs2_timer_counter").unwrap_or(0);
            let next_id = current_id + 1;
            lua_ctx.globals().set("__rs2_timer_counter", next_id)?;
            let timer = lua_ctx.create_table()?;
            timer.set("remaining", seconds)?;
            timer.set("interval", seconds)?;
            timer.set("repeating", true)?;
            timer.set("callback", callback)?;
            timers_every.set(next_id, timer)?;
            Ok(next_id)
        }).map_err(|e| e.to_string())?;
        game_tbl.set("every", every_fn).ok();

        let raycast_fn = lua.create_function(move |lua_ctx, (ox, oy, dx, dy, max_dist): (f32,f32,f32,f32,f32)| {
            let t = lua_ctx.create_table()?;
            // Raycast AABB manual sobre o snapshot de dados seguros
            let len = (dx * dx + dy * dy).sqrt();
            if len < f32::EPSILON {
                t.set("hit", false)?;
                return Ok(t);
            }
            let (ndx, ndy) = (dx / len, dy / len);
            let min_dim = ray_snap.iter().map(|c| c.2.min(c.3)).fold(f32::MAX, f32::min);
            let step = (min_dim * 0.5).max(2.0).min(16.0);
            let mut best: Option<(f32, f32, f32, String, String)> = None; // (hit_x, hit_y, dist, name, id)
            let mut d = 0.0_f32;
            while d <= max_dist {
                let px = ox + ndx * d;
                let py = oy + ndy * d;
                for (cx, cy, w, h, _layer, name, id) in &ray_snap {
                    if (px - cx).abs() <= w * 0.5 && (py - cy).abs() <= h * 0.5 {
                        if best.is_none() {
                            best = Some((px, py, d, name.clone(), id.clone()));
                        }
                    }
                }
                if best.is_some() { break; }
                d += step;
            }
            match best {
                None => { t.set("hit", false)?; }
                Some((hx, hy, hd, hname, hid)) => {
                    t.set("hit", true)?;
                    t.set("x", hx)?;
                    t.set("y", hy)?;
                    t.set("dist", hd)?;
                    t.set("name", hname)?;
                    t.set("id", hid)?;
                }
            }
            Ok(t)
        }).map_err(|e| e.to_string())?;
        game_tbl.set("raycast", raycast_fn).ok();

        lua.globals().set("game", game_tbl).map_err(|e| e.to_string())?;
    }

    // ── tabela `save` ────────────────────────────────────────
    {
        let save_tbl = lua.create_table().map_err(|e| e.to_string())?;
        let save_cmds: Table = lua.create_table().map_err(|e| e.to_string())?;

        // save.get(key) → number | string | bool | nil
        let snap: Vec<(String, crate::runtime::save::SaveValue)> = save_data.entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let get_fn = lua.create_function(move |lua_ctx, key: String| {
            let val = snap.iter().find(|(k, _)| k == &key).map(|(_, v)| v.clone());
            match val {
                None => Ok(LuaValue::Nil),
                Some(crate::runtime::save::SaveValue::Bool(b))  => Ok(LuaValue::Boolean(b)),
                Some(crate::runtime::save::SaveValue::Int(i))   => Ok(LuaValue::Integer(i)),
                Some(crate::runtime::save::SaveValue::Float(f)) => Ok(LuaValue::Number(f)),
                Some(crate::runtime::save::SaveValue::Text(s))  => {
                    Ok(LuaValue::String(lua_ctx.create_string(&s)?))
                }
            }
        }).map_err(|e| e.to_string())?;
        save_tbl.set("get", get_fn).ok();

        // save.has(key)
        let keys: HashSet<String> = save_data.entries.keys().cloned().collect();
        let has_fn = lua.create_function(move |_, key: String| Ok(keys.contains(&key)))
            .map_err(|e| e.to_string())?;
        save_tbl.set("has", has_fn).ok();

        // save.set(key, value) — apenas gravado em _cmds para aplicar depois
        let sc = save_cmds.clone();
        let set_fn = lua.create_function(move |_, (key, val): (String, LuaValue)| {
            match val {
                LuaValue::Boolean(b) => sc.set(format!("b:{}", key), b)?,
                LuaValue::Integer(i) => sc.set(format!("i:{}", key), i)?,
                LuaValue::Number(f)  => sc.set(format!("f:{}", key), f)?,
                LuaValue::String(s)  => sc.set(format!("s:{}", key), s.to_string_lossy().to_string())?,
                _ => {}
            }
            Ok(())
        }).map_err(|e| e.to_string())?;
        save_tbl.set("set", set_fn).ok();

        // save.remove(key)
        let sc = save_cmds.clone();
        let rm_fn = lua.create_function(move |_, key: String| {
            sc.set(format!("rm:{}", key), true)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        save_tbl.set("remove", rm_fn).ok();
        save_tbl.set("_cmds", save_cmds).ok();

        lua.globals().set("save", save_tbl).map_err(|e| e.to_string())?;
    }

    // ── tabela `session` ─────────────────────────────────────
    {
        let session_tbl = lua.create_table().map_err(|e| e.to_string())?;
        let session_cmds: Table = lua.create_table().map_err(|e| e.to_string())?;

        let snap: Vec<(String, SaveValue)> = session_data
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let get_fn = lua.create_function(move |lua_ctx, (key, default): (String, LuaValue)| {
            let val = snap.iter().find(|(k, _)| k == &key).map(|(_, v)| v.clone());
            match val {
                None => Ok(default),
                Some(v) => save_value_to_lua(lua_ctx, &v).map_err(mlua::Error::runtime),
            }
        }).map_err(|e| e.to_string())?;
        session_tbl.set("get", get_fn).ok();

        let keys: HashSet<String> = session_data.keys().cloned().collect();
        let has_fn = lua.create_function(move |_, key: String| Ok(keys.contains(&key)))
            .map_err(|e| e.to_string())?;
        session_tbl.set("has", has_fn).ok();

        let sc = session_cmds.clone();
        let set_fn = lua.create_function(move |_, (key, val): (String, LuaValue)| {
            match val {
                LuaValue::Boolean(b) => sc.set(format!("b:{}", key), b)?,
                LuaValue::Integer(i) => sc.set(format!("i:{}", key), i)?,
                LuaValue::Number(f)  => sc.set(format!("f:{}", key), f)?,
                LuaValue::String(s)  => sc.set(format!("s:{}", key), s.to_string_lossy().to_string())?,
                LuaValue::Nil => {},
                _ => return Err(mlua::Error::runtime("session.set suporta apenas bool, integer, number e string")),
            }
            Ok(())
        }).map_err(|e| e.to_string())?;
        session_tbl.set("set", set_fn).ok();

        let sc = session_cmds.clone();
        let rm_fn = lua.create_function(move |_, key: String| {
            sc.set(format!("rm:{}", key), true)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        session_tbl.set("remove", rm_fn).ok();

        let sc = session_cmds.clone();
        let clear_fn = lua.create_function(move |_, ()| {
            sc.set("clear", true)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        session_tbl.set("clear", clear_fn).ok();

        session_tbl.set("_cmds", session_cmds).ok();
        lua.globals().set("session", session_tbl).map_err(|e| e.to_string())?;
    }

    // ── tabela `state` ───────────────────────────────────────
    {
        let state_tbl = lua.create_table().map_err(|e| e.to_string())?;
        let state_cmds: Table = lua.create_table().map_err(|e| e.to_string())?;

        let snap: Vec<(String, SaveValue)> = script_data
            .into_iter()
            .flat_map(|data| data.iter().map(|(k, v)| (k.clone(), v.clone())))
            .collect();
        let get_fn = lua.create_function(move |lua_ctx, (key, default): (String, LuaValue)| {
            let val = snap.iter().find(|(k, _)| k == &key).map(|(_, v)| v.clone());
            match val {
                None => Ok(default),
                Some(v) => save_value_to_lua(lua_ctx, &v).map_err(mlua::Error::runtime),
            }
        }).map_err(|e| e.to_string())?;
        state_tbl.set("get", get_fn).ok();

        let keys: HashSet<String> = script_data
            .map(|data| data.keys().cloned().collect())
            .unwrap_or_default();
        let has_fn = lua.create_function(move |_, key: String| Ok(keys.contains(&key)))
            .map_err(|e| e.to_string())?;
        state_tbl.set("has", has_fn).ok();

        let sc = state_cmds.clone();
        let set_fn = lua.create_function(move |_, (key, val): (String, LuaValue)| {
            match val {
                LuaValue::Boolean(b) => sc.set(format!("b:{}", key), b)?,
                LuaValue::Integer(i) => sc.set(format!("i:{}", key), i)?,
                LuaValue::Number(f)  => sc.set(format!("f:{}", key), f)?,
                LuaValue::String(s)  => sc.set(format!("s:{}", key), s.to_string_lossy().to_string())?,
                LuaValue::Nil => {},
                _ => return Err(mlua::Error::runtime("state.set suporta apenas bool, integer, number e string")),
            }
            Ok(())
        }).map_err(|e| e.to_string())?;
        state_tbl.set("set", set_fn).ok();

        let sc = state_cmds.clone();
        let rm_fn = lua.create_function(move |_, key: String| {
            sc.set(format!("rm:{}", key), true)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        state_tbl.set("remove", rm_fn).ok();

        let sc = state_cmds.clone();
        let clear_fn = lua.create_function(move |_, ()| {
            sc.set("clear", true)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        state_tbl.set("clear", clear_fn).ok();

        state_tbl.set("_cmds", state_cmds).ok();
        lua.globals().set("state", state_tbl).map_err(|e| e.to_string())?;
    }

    // ── tabela `event` ───────────────────────────────────────
    {
        let event_tbl = lua.create_table().map_err(|e| e.to_string())?;
        let event_cmds: Table = lua.create_table().map_err(|e| e.to_string())?;
        let listeners: Table = lua.globals().get("__rs2_event_listeners").unwrap_or_else(|_| lua.create_table().unwrap());
        lua.globals().set("__rs2_event_listeners", listeners.clone()).ok();

        let listeners_for_set = listeners.clone();
        let listen_fn = lua.create_function(move |lua_ctx, (name, callback): (String, Function)| {
            let existing: Table = match listeners_for_set.get::<LuaValue>(name.as_str())? {
                LuaValue::Table(t) => t,
                _ => lua_ctx.create_table()?,
            };
            let next_index = existing.raw_len() + 1;
            existing.set(next_index, callback)?;
            listeners_for_set.set(name, existing)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        event_tbl.set("listen", listen_fn).ok();

        let emit_cmds = event_cmds.clone();
        let emit_fn = lua.create_function(move |_lua_ctx, (name, data): (String, LuaValue)| {
            let seq: i64 = emit_cmds.get("seq").unwrap_or(0) + 1;
            emit_cmds.set("seq", seq)?;
            emit_cmds.set(format!("name:{}", seq), name.clone())?;
            set_scalar_cmd(&emit_cmds, &seq.to_string(), "data", data)?;
            Ok(())
        }).map_err(|e| e.to_string())?;
        event_tbl.set("emit", emit_fn).ok();
        event_tbl.set("_cmds", event_cmds).ok();
        lua.globals().set("event", event_tbl).map_err(|e| e.to_string())?;
    }

    // print(...) global com prefixo contextual para logs Lua
    {
        let print_prefix = format!("{}[stage=print]", format_lua_context(entity, scene_label, script_path));
        let print_fn = lua.create_function(move |_, values: Variadic<LuaValue>| {
            let joined = values.iter()
                .map(lua_value_to_log_string)
                .collect::<Vec<_>>()
                .join("	");
            println!("{} {}", print_prefix, joined);
            Ok(())
        }).map_err(|e| e.to_string())?;
        lua.globals().set("print", print_fn).map_err(|e| e.to_string())?;
    }

    // ── carrega e executa o script uma única vez por VM ─────
    let chunk_loaded = lua.globals().get::<bool>("__rs2_chunk_loaded").unwrap_or(false);
    if !chunk_loaded {
        lua.load(lua_source).exec().map_err(|e| stage_error("load", format!("Erro ao carregar script: {e}"), entity, scene_label, script_path))?;
        lua.globals().set("__rs2_chunk_loaded", true).map_err(|e| stage_error("load", e, entity, scene_label, script_path))?;
    }

    // on_start (apenas na primeira vez)
    if !started {
        if let Ok(f) = lua.globals().get::<mlua::Function>("on_start") {
            f.call::<()>(()).map_err(|e| stage_error("on_start", e, entity, scene_label, script_path))?;
        }
    }

    dispatch_runtime_events(lua, entity, scene_label, script_path, current_runtime_events)?;
    advance_timers(lua, entity, scene_label, script_path, delta_time)?;

    // on_update(delta_time)
    if let Ok(f) = lua.globals().get::<mlua::Function>("on_update") {
        f.call::<()>(delta_time).map_err(|e| stage_error("on_update", e, entity, scene_label, script_path))?;
    }

    // ── coleta resultados de entity._cmds ────────────────────
    if let Ok(entity_tbl) = lua.globals().get::<Table>("entity") {
        if let Ok(cmds) = entity_tbl.get::<Table>("_cmds") {
            if let (Ok(x), Ok(y)) = (cmds.get::<f32>("pos_x"), cmds.get::<f32>("pos_y")) {
                result.set_position = Some((x, y));
            }
            if let (Ok(vx), Ok(vy)) = (cmds.get::<f32>("vel_x"), cmds.get::<f32>("vel_y")) {
                result.set_velocity = Some((vx, vy));
            }
            if let Ok(r) = cmds.get::<f32>("rotation") {
                result.set_rotation = Some(r);
            }
            // IMPORTANTE: usar LuaValue em vez de bool direto.
            // mlua converte nil → false para bool, fazendo entidades
            // ficarem invisíveis mesmo sem o script chamar set_visible.
            if let Ok(LuaValue::Boolean(v)) = cmds.get::<LuaValue>("visible") {
                result.set_visible = Some(v);
            }
            if let Ok(LuaValue::Boolean(v)) = cmds.get::<LuaValue>("collision_enabled") {
                result.set_collision_enabled = Some(v);
            }
            if let Ok(LuaValue::Boolean(true)) = cmds.get::<LuaValue>("destroy") {
                result.destroy_entity = true;
            }
            if let Ok(clip) = cmds.get::<String>("anim") {
                result.play_anim = Some(clip);
            }
            if let Ok(text) = cmds.get::<String>("text") {
                result.set_text = Some(text);
            }
            if let Ok(value) = cmds.get::<f32>("set_hp") {
                result.set_hp = Some(value);
            }
            if let Ok(value) = cmds.get::<f32>("damage") {
                result.damage = Some(value);
            }
            if let Ok(value) = cmds.get::<f32>("heal") {
                result.heal = Some(value);
            }
            if let Ok(value) = cmds.get::<bool>("flip_x") {
                result.set_flip_x = Some(value);
            }
            if let (Ok(r), Ok(g), Ok(b), Ok(a)) = (
                cmds.get::<f32>("color_r"),
                cmds.get::<f32>("color_g"),
                cmds.get::<f32>("color_b"),
                cmds.get::<f32>("color_a"),
            ) {
                result.set_color = Some((r, g, b, a));
            }
            if let (Ok(ix), Ok(iy)) = (cmds.get::<f32>("impulse_x"), cmds.get::<f32>("impulse_y")) {
                result.apply_impulse = Some((ix, iy));
            }
        }
    }

    if let Ok(event_tbl) = lua.globals().get::<Table>("event") {
        if let Ok(cmds) = event_tbl.get::<Table>("_cmds") {
            let max_seq = cmds.get::<i64>("seq").unwrap_or(0);
            for seq in 1..=max_seq {
                let Ok(name) = cmds.get::<String>(format!("name:{}", seq)) else { continue };
                let data = if let Ok(LuaValue::Boolean(v)) = cmds.get::<LuaValue>(format!("b:{}:data", seq)) {
                    Some(SaveValue::Bool(v))
                } else if let Ok(LuaValue::Integer(v)) = cmds.get::<LuaValue>(format!("i:{}:data", seq)) {
                    Some(SaveValue::Int(v))
                } else if let Ok(LuaValue::Number(v)) = cmds.get::<LuaValue>(format!("f:{}:data", seq)) {
                    Some(SaveValue::Float(v))
                } else if let Ok(LuaValue::String(v)) = cmds.get::<LuaValue>(format!("s:{}:data", seq)) {
                    Some(SaveValue::Text(v.to_string_lossy().to_string()))
                } else {
                    None
                };
                result.event_ops.push(EventOp::Emit { name, data });
                let _ = cmds.raw_remove(format!("name:{}", seq));
                let _ = cmds.raw_remove(format!("b:{}:data", seq));
                let _ = cmds.raw_remove(format!("i:{}:data", seq));
                let _ = cmds.raw_remove(format!("f:{}:data", seq));
                let _ = cmds.raw_remove(format!("s:{}:data", seq));
            }
            let _ = cmds.set("seq", 0);
        }
    }

    if let Ok(game_tbl) = lua.globals().get::<Table>("game") {
        if let Ok(gcmds) = game_tbl.get::<Table>("_cmds") {
            if let Ok(path) = gcmds.get::<String>("change_scene") {
                result.change_scene = Some(path);
            }
            if let (Ok(intensity), Ok(duration)) = (
                gcmds.get::<f32>("camera_shake_intensity"),
                gcmds.get::<f32>("camera_shake_duration"),
            ) {
                result.camera_shake = Some((intensity, duration));
            }
            if let Ok(zoom) = gcmds.get::<f32>("camera_zoom") {
                result.camera_zoom = Some(zoom);
            }
        }
    }

    // ── coleta session._cmds ──────────────────────────────────
    if let Ok(session_tbl) = lua.globals().get::<Table>("session") {
        if let Ok(cmds) = session_tbl.get::<Table>("_cmds") {
            if let Ok(LuaValue::Boolean(true)) = cmds.get::<LuaValue>("clear") {
                result.session_ops.push(SessionOp::Clear);
            }
            for pair in cmds.clone().pairs::<String, LuaValue>() {
                let Ok((raw_key, val)) = pair else { continue };
                if raw_key == "clear" { continue }
                if let Some(key) = raw_key.strip_prefix("rm:") {
                    result.session_ops.push(SessionOp::Remove(key.to_string()));
                } else if let Some(key) = raw_key.strip_prefix("b:") {
                    if let LuaValue::Boolean(b) = val {
                        result.session_ops.push(SessionOp::Set(key.to_string(), SaveValue::Bool(b)));
                    }
                } else if let Some(key) = raw_key.strip_prefix("i:") {
                    if let LuaValue::Integer(i) = val {
                        result.session_ops.push(SessionOp::Set(key.to_string(), SaveValue::Int(i)));
                    }
                } else if let Some(key) = raw_key.strip_prefix("f:") {
                    if let LuaValue::Number(f) = val {
                        result.session_ops.push(SessionOp::Set(key.to_string(), SaveValue::Float(f)));
                    }
                } else if let Some(key) = raw_key.strip_prefix("s:") {
                    if let LuaValue::String(s) = val {
                        result.session_ops.push(SessionOp::Set(key.to_string(), SaveValue::Text(s.to_string_lossy().to_string())));
                    }
                }
            }
        }
    }

    // ── coleta state._cmds ────────────────────────────────────
    if let Ok(state_tbl) = lua.globals().get::<Table>("state") {
        if let Ok(cmds) = state_tbl.get::<Table>("_cmds") {
            if let Ok(LuaValue::Boolean(true)) = cmds.get::<LuaValue>("clear") {
                result.state_ops.push(StateOp::Clear);
            }
            for pair in cmds.clone().pairs::<String, LuaValue>() {
                let Ok((raw_key, val)) = pair else { continue };
                if raw_key == "clear" { continue }
                if let Some(key) = raw_key.strip_prefix("rm:") {
                    result.state_ops.push(StateOp::Remove(key.to_string()));
                } else if let Some(key) = raw_key.strip_prefix("b:") {
                    if let LuaValue::Boolean(b) = val {
                        result.state_ops.push(StateOp::Set(key.to_string(), SaveValue::Bool(b)));
                    }
                } else if let Some(key) = raw_key.strip_prefix("i:") {
                    if let LuaValue::Integer(i) = val {
                        result.state_ops.push(StateOp::Set(key.to_string(), SaveValue::Int(i)));
                    }
                } else if let Some(key) = raw_key.strip_prefix("f:") {
                    if let LuaValue::Number(f) = val {
                        result.state_ops.push(StateOp::Set(key.to_string(), SaveValue::Float(f)));
                    }
                } else if let Some(key) = raw_key.strip_prefix("s:") {
                    if let LuaValue::String(s) = val {
                        result.state_ops.push(StateOp::Set(key.to_string(), SaveValue::Text(s.to_string_lossy().to_string())));
                    }
                }
            }
        }
    }

    // ── coleta save._cmds ─────────────────────────────────────
    if let Ok(save_tbl) = lua.globals().get::<Table>("save") {
        if let Ok(cmds) = save_tbl.get::<Table>("_cmds") {
            for pair in cmds.clone().pairs::<String, LuaValue>() {
                let Ok((raw_key, val)) = pair else { continue };
                if let Some(key) = raw_key.strip_prefix("rm:") {
                    result.save_ops.push(SaveOp::Remove(key.to_string()));
                } else if let Some(key) = raw_key.strip_prefix("b:") {
                    if let LuaValue::Boolean(b) = val {
                        result.save_ops.push(SaveOp::Set(key.to_string(), SaveValue::Bool(b)));
                    }
                } else if let Some(key) = raw_key.strip_prefix("i:") {
                    if let LuaValue::Integer(i) = val {
                        result.save_ops.push(SaveOp::Set(key.to_string(), SaveValue::Int(i)));
                    }
                } else if let Some(key) = raw_key.strip_prefix("f:") {
                    if let LuaValue::Number(f) = val {
                        result.save_ops.push(SaveOp::Set(key.to_string(), SaveValue::Float(f)));
                    }
                } else if let Some(key) = raw_key.strip_prefix("s:") {
                    if let LuaValue::String(s) = val {
                        result.save_ops.push(SaveOp::Set(key.to_string(), SaveValue::Text(s.to_string_lossy().to_string())));
                    }
                }
            }
        }
    }

    Ok(result)
}

/// Aplica os resultados de um `LuaScriptResult` na entidade.
pub fn apply_lua_result(entity: &mut Entity, r: &LuaScriptResult) {
    if let Some((x, y)) = r.set_position {
        if let Some(t) = entity.transform_mut() {
            t.x = x;
            t.y = y;
        }
    }
    if let Some((vx, vy)) = r.set_velocity {
        for c in &mut entity.components {
            if let Component::Velocity(v) = c {
                v.x = vx;
                v.y = vy;
                break;
            }
        }
    }
    if let Some(r_val) = r.set_rotation {
        if let Some(t) = entity.transform_mut() {
            t.rotation = r_val;
        }
    }
    if let Some(vis) = r.set_visible {
        entity.visible = vis;
    }
    if let Some(enabled) = r.set_collision_enabled {
        entity.set_collision_enabled(enabled);
    }
    if let Some(clip) = &r.play_anim {
        for c in &mut entity.components {
            if let Component::Animator(anim) = c {
                if anim.clips.contains_key(clip.as_str()) {
                    anim.current = clip.clone();
                }
                break;
            }
        }
    }
    if let Some(text) = &r.set_text {
        for c in &mut entity.components {
            if let Component::TextLabel(label) = c {
                label.text = text.clone();
                break;
            }
        }
    }
    if let Some(set_hp) = r.set_hp {
        entity.set_hp(set_hp);
    }

    if let Some(damage) = r.damage {
        entity.damage(damage);
    }

    if let Some(heal) = r.heal {
        entity.heal(heal);
    }

    if let Some(flip_x) = r.set_flip_x {
        if let Some(transform) = entity.transform_mut() {
            let sign = if flip_x { -1.0 } else { 1.0 };
            transform.scale_x = transform.scale_x.abs().max(0.0001) * sign;
        }
    }

    if let Some((r_val, g_val, b_val, a_val)) = r.set_color {
        for c in &mut entity.components {
            match c {
                Component::Sprite(sprite) => { sprite.color_r = r_val; sprite.color_g = g_val; sprite.color_b = b_val; sprite.color_a = a_val; }
                Component::TextLabel(label) => { label.color_r = r_val; label.color_g = g_val; label.color_b = b_val; label.color_a = a_val; }
                Component::UIButton(button) => { button.color_r = r_val; button.color_g = g_val; button.color_b = b_val; button.color_a = a_val; }
                _ => {}
            }
        }
    }
    for tag in &r.add_tags {
        let _ = entity.add_tag(tag.clone());
    }
    if let Some((ix, iy)) = r.apply_impulse {
        for c in &mut entity.components {
            if let Component::Velocity(v) = c {
                v.x += ix;
                v.y += iy;
                break;
            }
        }
    }
}

/// Aplica as operações de save coletadas pelo script.
pub fn apply_save_ops(save_data: &mut SaveData, ops: &[SaveOp]) {
    for op in ops {
        match op {
            SaveOp::Set(key, val) => save_data.set(key.clone(), val.clone()),
            SaveOp::Remove(key)   => { save_data.remove(key); }
        }
    }
}

// ── snapshot de input (Send + Clone para closures) ───────────

#[derive(Clone, Default)]
struct InputSnapshot {
    held:    Vec<String>,
    pressed: Vec<String>,
}

impl InputSnapshot {
    fn key_held(&self, name: &str)    -> bool { self.held.iter().any(|k| k.eq_ignore_ascii_case(name)) }
    fn key_pressed(&self, name: &str) -> bool { self.pressed.iter().any(|k| k.eq_ignore_ascii_case(name)) }
}

impl From<&RuntimeInput> for InputSnapshot {
    fn from(input: &RuntimeInput) -> Self {
        let all_keys = [
            ("W",      KeyCode::W),     ("A", KeyCode::A),
            ("S",      KeyCode::S),     ("D", KeyCode::D),
            ("Up",     KeyCode::Up),    ("Down",  KeyCode::Down),
            ("Left",   KeyCode::Left),  ("Right", KeyCode::Right),
            ("Space",  KeyCode::Space), ("Enter", KeyCode::Enter),
            ("Escape", KeyCode::Escape),
        ];
        let mut held    = Vec::new();
        let mut pressed = Vec::new();
        for (name, code) in &all_keys {
            if input.is_key_held(*code)    { held.push(name.to_string()); }
            if input.is_key_pressed(*code) { pressed.push(name.to_string()); }
        }
        Self { held, pressed }
    }
}

// ── testes ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::entity::Entity;

    fn dummy_entity() -> Entity {
        let mut e = Entity::new("TestEntity");
        e.add_component(Component::Velocity(
            crate::core::component::Velocity { x: 0.0, y: 0.0 }
        ));
        e
    }

    #[test]
    fn on_update_move() {
        let entity = dummy_entity();
        let input  = RuntimeInput::default();
        let save   = SaveData::new();
        let src    = "function on_update(dt) entity.set_velocity(100, 0) end";
        let r = run_lua_script(src, &entity, &input, &save, &std::collections::HashMap::new(), None, 0.016, 0.0, true, &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], None, None).unwrap();
        assert_eq!(r.set_velocity, Some((100.0, 0.0)));
    }

    #[test]
    fn on_start_called_once() {
        let entity = dummy_entity();
        let input  = RuntimeInput::default();
        let save   = SaveData::new();
        let src    = "function on_start() entity.set_position(10, 20) end \
                      function on_update(dt) end";
        let r = run_lua_script(src, &entity, &input, &save, &std::collections::HashMap::new(), None, 0.016, 0.0, false, &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], None, None).unwrap();
        assert_eq!(r.set_position, Some((10.0, 20.0)));

        // segunda chamada com started=true → on_start não roda
        let r2 = run_lua_script(src, &entity, &input, &save, &std::collections::HashMap::new(), None, 0.016, 0.016, true, &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], None, None).unwrap();
        assert_eq!(r2.set_position, None);
    }

    #[test]
    fn save_set_via_lua() {
        let entity = dummy_entity();
        let input  = RuntimeInput::default();
        let mut save = SaveData::new();
        let src    = r#"function on_update(dt) save.set("score", 99) end"#;
        let r = run_lua_script(src, &entity, &input, &save, &std::collections::HashMap::new(), None, 0.016, 0.0, true, &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], &[], None, None).unwrap();
        apply_save_ops(&mut save, &r.save_ops);
        assert_eq!(save.get_int("score"), Some(99));
    }
}
pub fn apply_event_ops(
    pending_runtime_events: &mut Vec<crate::runtime::state::RuntimeEvent>,
    ops: &[EventOp],
) {
    for op in ops {
        match op {
            EventOp::Emit { name, data } => pending_runtime_events.push(crate::runtime::state::RuntimeEvent {
                name: name.clone(),
                data: data.clone(),
            }),
        }
    }
}

pub fn apply_session_ops(session_data: &mut std::collections::HashMap<String, SaveValue>, ops: &[SessionOp]) {
    for op in ops {
        match op {
            SessionOp::Set(key, value) => { session_data.insert(key.clone(), value.clone()); }
            SessionOp::Remove(key) => { session_data.remove(key); }
            SessionOp::Clear => session_data.clear(),
        }
    }
}




pub fn apply_state_ops(
    script_state: &mut crate::runtime::state::ScriptState,
    script_scope_key: &str,
    ops: &[StateOp],
) {
    for op in ops {
        match op {
            StateOp::Set(key, value) => {
                script_state
                    .entry(script_scope_key.to_string())
                    .or_default()
                    .insert(key.clone(), value.clone());
            }
            StateOp::Remove(key) => {
                let should_remove_entity = if let Some(entity_state) = script_state.get_mut(script_scope_key) {
                    entity_state.remove(key);
                    entity_state.is_empty()
                } else {
                    false
                };
                if should_remove_entity {
                    script_state.remove(script_scope_key);
                }
            }
            StateOp::Clear => {
                script_state.remove(script_scope_key);
            }
        }
    }
}
