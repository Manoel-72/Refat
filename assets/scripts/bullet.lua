-- Bala amarela: pool de projéteis ativados pelo player.
-- Ajustada para sensação mais rápida: vida menor, velocidade maior vem do player,
-- e a bala avança um pequeno passo já no primeiro frame em que é ativada.

local idx = 1
local life = 0.0
local just_activated = false

local function get_index()
    local n = string.match(entity.name, "Bullet_(%d+)")
    return tonumber(n) or 1
end

function on_start()
    idx = get_index()
    entity.set_visible(false)
    entity.set_collision_enabled(false)
    life = 0.0
    just_activated = false
end

local function deactivate()
    session.set("bullet_" .. idx .. "_active", false)
    session.set("bullet_" .. idx .. "_life", 0.0)
    entity.set_visible(false)
    entity.set_collision_enabled(false)
    life = 0.0
    just_activated = false
end

function on_update(dt)
    if session.get("game_over", false) or session.get("victory", false) then
        deactivate()
        return
    end

    local active = session.get("bullet_" .. idx .. "_active", false)
    if not active then
        entity.set_visible(false)
        entity.set_collision_enabled(false)
        life = 0.0
        just_activated = false
        return
    end

    local x = session.get("bullet_" .. idx .. "_x", entity.x)
    local y = session.get("bullet_" .. idx .. "_y", entity.y)
    local vx = session.get("bullet_" .. idx .. "_vx", 0.0)
    local vy = session.get("bullet_" .. idx .. "_vy", 0.0)

    local stored_life = session.get("bullet_" .. idx .. "_life", 0.75)
    if life <= 0.0 then
        life = stored_life
        just_activated = true
    end

    -- Remove sensação de atraso: no frame de ativação, já anda um mínimo visual.
    local step_dt = dt
    if just_activated and step_dt < 0.016 then
        step_dt = 0.016
    end
    just_activated = false

    life = life - dt
    x = x + vx * step_dt
    y = y + vy * step_dt

    entity.set_visible(true)
    entity.set_collision_enabled(true)
    entity.set_position(x, y)
    session.set("bullet_" .. idx .. "_x", x)
    session.set("bullet_" .. idx .. "_y", y)
    session.set("bullet_" .. idx .. "_life", life)

    for i = 1, 4 do
        if session.get("enemy_" .. i .. "_alive", false) then
            local ex = session.get("enemy_" .. i .. "_x", 99999.0)
            local ey = session.get("enemy_" .. i .. "_y", 99999.0)
            local dx = ex - x
            local dy = ey - y
            if math.sqrt(dx * dx + dy * dy) < 30.0 then
                session.set("enemy_" .. i .. "_hit", true)
                deactivate()
                return
            end
        end
    end

    if life <= 0.0 or x < -460.0 or x > 460.0 or y < -330.0 or y > 330.0 then
        deactivate()
    end
end
