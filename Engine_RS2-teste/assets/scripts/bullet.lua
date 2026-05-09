-- Bala amarela: fica invisível até o player ativar pelo session.
local idx = 1
local life = 0.0

local function get_index()
    local n = string.match(entity.name, "Bullet_(%d+)")
    return tonumber(n) or 1
end

function on_start()
    idx = get_index()
    entity.set_visible(false)
    entity.set_collision_enabled(false)
end

local function deactivate()
    session.set("bullet_" .. idx .. "_active", false)
    entity.set_visible(false)
    entity.set_collision_enabled(false)
    life = 0.0
end

function on_update(dt)
    if session.get("game_over", false) or session.get("victory", false) then
        deactivate()
        return
    end

    local active = session.get("bullet_" .. idx .. "_active", false)
    if not active then
        entity.set_visible(false)
        return
    end

    local x = session.get("bullet_" .. idx .. "_x", entity.x)
    local y = session.get("bullet_" .. idx .. "_y", entity.y)
    local vx = session.get("bullet_" .. idx .. "_vx", 0.0)
    local vy = session.get("bullet_" .. idx .. "_vy", 0.0)

    if life <= 0.0 then life = 1.3 end
    life = life - dt
    x = x + vx * dt
    y = y + vy * dt

    entity.set_visible(true)
    entity.set_collision_enabled(true)
    entity.set_position(x, y)
    session.set("bullet_" .. idx .. "_x", x)
    session.set("bullet_" .. idx .. "_y", y)

    for i = 1, 4 do
        if session.get("enemy_" .. i .. "_alive", false) then
            local ex = session.get("enemy_" .. i .. "_x", 99999.0)
            local ey = session.get("enemy_" .. i .. "_y", 99999.0)
            local dx = ex - x
            local dy = ey - y
            if math.sqrt(dx * dx + dy * dy) < 28.0 then
                session.set("enemy_" .. i .. "_hit", true)
                deactivate()
                return
            end
        end
    end

    if life <= 0.0 or x < -430.0 or x > 430.0 or y < -300.0 or y > 300.0 then
        deactivate()
    end
end
