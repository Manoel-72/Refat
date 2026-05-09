-- Player verde: move com WASD/setas e atira com o botão esquerdo do mouse.
-- Mudança principal desta versão:
-- 1) mira usa a posição do mouse no mundo;
-- 2) tiro sai no primeiro frame do clique/segurando mouse;
-- 3) velocidade da bala maior e cooldown menor para não parecer travado.

local speed = 220.0
local fire_cooldown = 0.0
local last_dx, last_dy = 0.0, 1.0
local max_hp = 5
local bullet_count = 12
local fire_rate = 0.09
local bullet_speed = 760.0
local bullet_spawn_offset = 28.0

local function is_down(a, b)
    return input.key_held(a) or input.key_held(b)
end

function on_start()
    session.set("player_id", entity.id)
    session.set("player_hp", max_hp)
    session.set("player_x", entity.x)
    session.set("player_y", entity.y)
    session.set("game_over", false)
    session.set("victory", false)
    session.set("enemies_dead", 0)
    for i = 1, bullet_count do
        session.set("bullet_" .. i .. "_active", false)
        session.set("bullet_" .. i .. "_life", 0.0)
    end
end

local function get_mouse_world()
    -- A engine expõe input.get_mouse_world_pos() como tabela {x, y}.
    -- Fallback para input.mouse_world_pos caso esteja rodando em versão antiga.
    if input.get_mouse_world_pos ~= nil then
        return input.get_mouse_world_pos()
    end
    if input.mouse_world_pos ~= nil then
        return input.mouse_world_pos()
    end
    return { x = entity.x + last_dx * 100.0, y = entity.y + last_dy * 100.0 }
end

local function shoot_to_mouse()
    local mouse = get_mouse_world()
    local dx = (mouse.x or mouse[1] or entity.x) - entity.x
    local dy = (mouse.y or mouse[2] or entity.y) - entity.y
    local len = math.sqrt(dx * dx + dy * dy)

    -- Evita tiro parado se o mouse estiver exatamente em cima do player.
    if len < 0.001 then
        dx, dy = last_dx, last_dy
        len = math.sqrt(dx * dx + dy * dy)
    end

    dx = dx / len
    dy = dy / len
    last_dx, last_dy = dx, dy

    for i = 1, bullet_count do
        if not session.get("bullet_" .. i .. "_active", false) then
            session.set("bullet_" .. i .. "_active", true)
            session.set("bullet_" .. i .. "_x", entity.x + dx * bullet_spawn_offset)
            session.set("bullet_" .. i .. "_y", entity.y + dy * bullet_spawn_offset)
            session.set("bullet_" .. i .. "_vx", dx * bullet_speed)
            session.set("bullet_" .. i .. "_vy", dy * bullet_speed)
            session.set("bullet_" .. i .. "_life", 0.75)
            break
        end
    end
end

function on_update(dt)
    if session.get("game_over", false) or session.get("victory", false) then
        entity.set_velocity(0.0, 0.0)
        return
    end

    local hp = session.get("player_hp", max_hp)
    if session.get("player_hit", false) then
        hp = hp - 1
        session.set("player_hp", hp)
        session.set("player_hit", false)
        game.flash_screen(1.0, 0.1, 0.1, 0.12)
    end

    if hp <= 0 then
        session.set("game_over", true)
        entity.set_velocity(0.0, 0.0)
        return
    end

    local dx, dy = 0.0, 0.0
    if is_down("A", "Left") then dx = dx - 1.0 end
    if is_down("D", "Right") then dx = dx + 1.0 end
    if is_down("W", "Up") then dy = dy + 1.0 end
    if is_down("S", "Down") then dy = dy - 1.0 end

    local len = math.sqrt(dx * dx + dy * dy)
    if len > 0.0 then
        dx = dx / len
        dy = dy / len
    end

    -- Tiro pelo mouse. Como usa mouse_left segurado, o primeiro tiro sai sem esperar soltar/clicar de novo.
    fire_cooldown = math.max(0.0, fire_cooldown - dt)
    if input.mouse_left and fire_cooldown <= 0.0 then
        shoot_to_mouse()
        fire_cooldown = fire_rate
    end

    local nx = entity.x + dx * speed * dt
    local ny = entity.y + dy * speed * dt
    nx = math.max(-360.0, math.min(360.0, nx))
    ny = math.max(-230.0, math.min(230.0, ny))
    entity.set_position(nx, ny)
    session.set("player_x", nx)
    session.set("player_y", ny)
end
