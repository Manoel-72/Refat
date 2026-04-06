-- =============================================================
-- enemy_patrol.lua — patrulha horizontal estável para a demo V0.9.1
--
-- Demo oficial:
-- - estado temporário do inimigo vive em state.*
-- - não usa save.* para direção/origem
-- =============================================================

local SPEED       = 90.0
local PATROL_DIST = 140.0
local TURN_MARGIN = 8.0

function on_start()
    state.set("origin_x", entity.x)
    state.set("dir", 1.0)
    game.log("Inimigo: patrulha inicializada em x=" .. tostring(entity.x))
end

function on_update(dt)
    local origin_x = state.get("origin_x", entity.x)
    local dir = state.get("dir", 1.0)

    if dir >= 0 then
        dir = 1.0
    else
        dir = -1.0
    end

    local min_x = origin_x - PATROL_DIST
    local max_x = origin_x + PATROL_DIST
    local next_x = entity.x + (SPEED * dir * dt)

    if dir > 0 and next_x >= (max_x - TURN_MARGIN) then
        next_x = max_x - TURN_MARGIN
        dir = -1.0
    elseif dir < 0 and next_x <= (min_x + TURN_MARGIN) then
        next_x = min_x + TURN_MARGIN
        dir = 1.0
    end

    entity.set_position(next_x, entity.y)
    entity.set_velocity(0.0, entity.vy)
    state.set("dir", dir)

    local cols = game.get_collisions()
    for _, name in ipairs(cols) do
        if name == "Player" then
            game.log("Inimigo tocou o Player!")
            break
        end
    end
end
