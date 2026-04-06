// ============================================================
//  runtime/lua_runtime.rs  —  V0.9
//  Execução de scripts Lua (.lua) por entidade.
//
//  API exposta ao Lua:
//    entity.x, entity.y, entity.rotation, entity.scale_x, entity.scale_y
//    entity.vx, entity.vy, entity.grounded, entity.visible, entity.name
//    entity.set_position(x, y)
//    entity.set_velocity(vx, vy)
//    entity.set_rotation(r)
//    entity.set_visible(bool)
//    entity.play_anim(clip_name)
//    input.key_held(name), input.key_pressed(name), input.mouse_pos()
//    game.delta_time, game.elapsed_time, game.log(msg)
//    game.change_scene(path)
//    game.get_collisions() → lista de nomes das entidades em contato
//    game.collision_enter(name) → bool, true apenas no frame de entrada
//    game.raycast(ox,oy,dx,dy,dist) → {hit, x, y, dist, name} ou nil
//    entity.id, entity.apply_impulse(ix, iy)
//    save.set(key, value), save.get(key), save.has(key), save.remove(key)
// ============================================================

use std::collections::HashSet;

use mlua::{Lua, Table, Value as LuaValue};

use crate::{
    core::{component::Component, entity::Entity},
    runtime::{
        input::key_code::KeyCode,
        save::SaveData,
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
    pub play_anim: Option<String>,
    pub set_text: Option<String>,
    pub change_scene: Option<String>,
    pub apply_impulse: Option<(f32, f32)>,
    pub save_ops: Vec<SaveOp>,
}

#[derive(Debug)]
pub enum SaveOp {
    Set(String, crate::runtime::save::SaveValue),
    Remove(String),
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
    delta_time: f32,
    elapsed_time: f32,
    started: bool,
    collision_names: &[String],
) -> Result<LuaScriptResult, String> {
    let lua = Lua::new();
    run_lua_script_with_vm(&lua, lua_source, entity, input, save_data, delta_time, elapsed_time, started, collision_names, &[])
}

/// Versão interna que recebe uma VM Lua já existente (reutilizada do cache).
pub fn run_lua_script_with_vm(
    lua: &Lua,
    lua_source: &str,
    entity: &Entity,
    input: &RuntimeInput,
    save_data: &SaveData,
    delta_time: f32,
    elapsed_time: f32,
    started: bool,
    collision_names: &[String],
    colliders: &[crate::runtime::systems::collision_system::RuntimeCollider],
) -> Result<LuaScriptResult, String> {
    let mut result = LuaScriptResult::default();

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
    entity_tbl.set("name", entity.name.clone()).ok();
    entity_tbl.set("id", entity.id.clone()).ok();

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
        let mouse_pos = lua.create_function(move |lua, ()| {
            let t = lua.create_table()?;
            t.set(1, mx)?;
            t.set(2, my)?;
            Ok(t)
        }).map_err(|e| e.to_string())?;
        input_tbl.set("mouse_pos", mouse_pos).ok();

        input_tbl.set("mouse_left",   input.mouse_left).ok();
        input_tbl.set("mouse_right",  input.mouse_right).ok();
        input_tbl.set("mouse_middle", input.mouse_middle).ok();

        lua.globals().set("input", input_tbl).map_err(|e| e.to_string())?;
    }

    // ── tabela `game` ────────────────────────────────────────
    {
        let game_tbl = lua.create_table().map_err(|e| e.to_string())?;
        game_tbl.set("delta_time", delta_time).ok();
        game_tbl.set("elapsed_time", elapsed_time).ok();

        // game.log(msg)
        let log_fn = lua.create_function(|_, msg: String| {
            println!("[LuaScript] {}", msg);
            Ok(())
        }).map_err(|e| e.to_string())?;
        game_tbl.set("log", log_fn).ok();

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

        // game.collision_enter(name) — true apenas no primeiro frame em que o nome aparece
        // Nota: usa um set armazenado em _cmds["_prev_cols"] como memória entre frames.
        // Por simplicidade de MVP, é implementado como contains (sem estado entre frames),
        // pois a VM é recriada por frame. Para lógica de "enter" real use grounded + flag no Lua.
        let col_enter: Vec<String> = collision_names.to_vec();
        let collision_enter = lua.create_function(move |_, name: String| {
            Ok(col_enter.iter().any(|n| n.eq_ignore_ascii_case(&name)))
        }).map_err(|e| e.to_string())?;
        game_tbl.set("collision_enter", collision_enter).ok();

        // game.raycast(ox, oy, dx, dy, max_dist) → {hit=true, x, y, dist, name} | {hit=false}
        // Snapshot sem raw ptr — apenas dados geométricos + nome, seguro para closure.
        let ray_snap: Vec<(f32, f32, f32, f32, u8, String)> = colliders.iter()
            .map(|c| (c.center_x, c.center_y, c.width, c.height, c.layer, c.entity_name.clone()))
            .collect();
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
            let mut best: Option<(f32, f32, f32, String)> = None; // (hit_x, hit_y, dist, name)
            let mut d = 0.0_f32;
            while d <= max_dist {
                let px = ox + ndx * d;
                let py = oy + ndy * d;
                for (cx, cy, w, h, _layer, name) in &ray_snap {
                    if (px - cx).abs() <= w * 0.5 && (py - cy).abs() <= h * 0.5 {
                        if best.is_none() {
                            best = Some((px, py, d, name.clone()));
                        }
                    }
                }
                if best.is_some() { break; }
                d += step;
            }
            match best {
                None => { t.set("hit", false)?; }
                Some((hx, hy, hd, hname)) => {
                    t.set("hit", true)?;
                    t.set("x", hx)?;
                    t.set("y", hy)?;
                    t.set("dist", hd)?;
                    t.set("name", hname)?;
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

    // ── carrega e executa o script ───────────────────────────
    lua.load(lua_source).exec().map_err(|e| format!("Erro ao carregar script: {e}"))?;

    // on_start (apenas na primeira vez)
    if !started {
        if let Ok(f) = lua.globals().get::<mlua::Function>("on_start") {
            f.call::<()>(()).map_err(|e| format!("on_start: {e}"))?;
        }
    }

    // on_update(delta_time)
    if let Ok(f) = lua.globals().get::<mlua::Function>("on_update") {
        f.call::<()>(delta_time).map_err(|e| format!("on_update: {e}"))?;
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
            if let Ok(clip) = cmds.get::<String>("anim") {
                result.play_anim = Some(clip);
            }
            if let Ok(text) = cmds.get::<String>("text") {
                result.set_text = Some(text);
            }
            if let (Ok(ix), Ok(iy)) = (cmds.get::<f32>("impulse_x"), cmds.get::<f32>("impulse_y")) {
                result.apply_impulse = Some((ix, iy));
            }
        }
    }

    // ── coleta resultados de game._cmds ──────────────────────
    if let Ok(game_tbl) = lua.globals().get::<Table>("game") {
        if let Ok(cmds) = game_tbl.get::<Table>("_cmds") {
            if let Ok(path) = cmds.get::<String>("change_scene") {
                result.change_scene = Some(path);
            }
        }
    }

    // ── coleta resultados de save._cmds ──────────────────────
    if let Ok(save_tbl) = lua.globals().get::<Table>("save") {
        if let Ok(cmds) = save_tbl.get::<Table>("_cmds") {
            for pair in cmds.pairs::<String, LuaValue>() {
                let Ok((key, val)) = pair else { continue };
                if let Some(real_key) = key.strip_prefix("rm:") {
                    result.save_ops.push(SaveOp::Remove(real_key.to_string()));
                } else if let Some(real_key) = key.strip_prefix("b:") {
                    if let LuaValue::Boolean(b) = val {
                        result.save_ops.push(SaveOp::Set(real_key.to_string(), b.into()));
                    }
                } else if let Some(real_key) = key.strip_prefix("i:") {
                    if let LuaValue::Integer(i) = val {
                        result.save_ops.push(SaveOp::Set(real_key.to_string(), i.into()));
                    }
                } else if let Some(real_key) = key.strip_prefix("f:") {
                    if let LuaValue::Number(f) = val {
                        result.save_ops.push(SaveOp::Set(real_key.to_string(), f.into()));
                    }
                } else if let Some(real_key) = key.strip_prefix("s:") {
                    if let LuaValue::String(s) = val {
                        result.save_ops.push(SaveOp::Set(
                            real_key.to_string(),
                            s.to_string_lossy().to_string().into(),
                        ));
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
        let r = run_lua_script(src, &entity, &input, &save, 0.016, 0.0, true, &[]).unwrap();
        assert_eq!(r.set_velocity, Some((100.0, 0.0)));
    }

    #[test]
    fn on_start_called_once() {
        let entity = dummy_entity();
        let input  = RuntimeInput::default();
        let save   = SaveData::new();
        let src    = "function on_start() entity.set_position(10, 20) end \
                      function on_update(dt) end";
        let r = run_lua_script(src, &entity, &input, &save, 0.016, 0.0, false, &[]).unwrap();
        assert_eq!(r.set_position, Some((10.0, 20.0)));

        // segunda chamada com started=true → on_start não roda
        let r2 = run_lua_script(src, &entity, &input, &save, 0.016, 0.016, true, &[]).unwrap();
        assert_eq!(r2.set_position, None);
    }

    #[test]
    fn save_set_via_lua() {
        let entity = dummy_entity();
        let input  = RuntimeInput::default();
        let mut save = SaveData::new();
        let src    = r#"function on_update(dt) save.set("score", 99) end"#;
        let r = run_lua_script(src, &entity, &input, &save, 0.016, 0.0, true, &[]).unwrap();
        apply_save_ops(&mut save, &r.save_ops);
        assert_eq!(save.get_int("score"), Some(99));
    }
}
