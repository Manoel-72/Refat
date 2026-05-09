-- Inimigo vermelho: segue o player, toma dano de tiros e morre.
local idx = 1
local hp = 3
local contact_cooldown = 0.0
local speed = 82.0

local function get_index()
    local n = string.match(entity.name, "Enemy_(%d+)")
    return tonumber(n) or 1
end

function on_start()
    idx = get_index()
    hp = 3
    session.set("enemy_" .. idx .. "_alive", true)
    session.set("enemy_" .. idx .. "_x", entity.x)
    session.set("enemy_" .. idx .. "_y", entity.y)
end

function on_update(dt)
    if session.get("game_over", false) or session.get("victory", false) then
        return
    end

    if session.get("enemy_" .. idx .. "_hit", false) then
        hp = hp - 1
        session.set("enemy_" .. idx .. "_hit", false)
        game.flash_screen(1.0, 1.0, 1.0, 0.04)
    end

    if hp <= 0 then
        session.set("enemy_" .. idx .. "_alive", false)
        local dead = session.get("enemies_dead", 0) + 1
        session.set("enemies_dead", dead)
        if dead >= 4 then
            session.set("victory", true)
        end
        entity.destroy()
        return
    end

    local px = session.get("player_x", 0.0)
    local py = session.get("player_y", 0.0)
    local dx = px - entity.x
    local dy = py - entity.y
    local dist = math.sqrt(dx * dx + dy * dy)

    if dist > 1.0 then
        dx = dx / dist
        dy = dy / dist
        entity.set_position(entity.x + dx * speed * dt, entity.y + dy * speed * dt)
    end

    contact_cooldown = math.max(0.0, contact_cooldown - dt)
    if dist < 34.0 and contact_cooldown <= 0.0 then
        session.set("player_hit", true)
        contact_cooldown = 0.75
    end

    session.set("enemy_" .. idx .. "_x", entity.x)
    session.set("enemy_" .. idx .. "_y", entity.y)
end
