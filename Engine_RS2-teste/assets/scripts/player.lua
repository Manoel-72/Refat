-- Player verde: move com WASD/setas e atira com Espaço.
local speed = 220.0
local fire_cooldown = 0.0
local last_dx, last_dy = 0.0, 1.0
local max_hp = 5

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
    for i = 1, 12 do
        session.set("bullet_" .. i .. "_active", false)
    end
end

local function shoot()
    for i = 1, 12 do
        if not session.get("bullet_" .. i .. "_active", false) then
            session.set("bullet_" .. i .. "_active", true)
            session.set("bullet_" .. i .. "_x", entity.x + last_dx * 26.0)
            session.set("bullet_" .. i .. "_y", entity.y + last_dy * 26.0)
            session.set("bullet_" .. i .. "_vx", last_dx * 520.0)
            session.set("bullet_" .. i .. "_vy", last_dy * 520.0)
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
        last_dx, last_dy = dx, dy
    end

    fire_cooldown = math.max(0.0, fire_cooldown - dt)
    if input.key_pressed("Space") and fire_cooldown <= 0.0 then
        shoot()
        fire_cooldown = 0.18
    end

    local nx = entity.x + dx * speed * dt
    local ny = entity.y + dy * speed * dt
    nx = math.max(-360.0, math.min(360.0, nx))
    ny = math.max(-230.0, math.min(230.0, ny))
    entity.set_position(nx, ny)
    session.set("player_x", nx)
    session.set("player_y", ny)
end
