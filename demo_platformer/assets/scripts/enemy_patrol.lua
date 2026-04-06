-- =============================================================
-- enemy_patrol.lua — patrulha horizontal estável para a demo V0.9.1
--
-- Demo oficial:
-- - reinicia o estado ao entrar no Game
-- - mantém estado entre frames via save.*
-- - não reaproveita direção/origem da sessão anterior
-- =============================================================

local SPEED       = 90.0
local PATROL_DIST = 140.0
local TURN_MARGIN = 8.0

local function key_origin()
    return "enemy_patrol.origin_x." .. entity.id
end

local function key_dir()
    return "enemy_patrol.dir." .. entity.id
end

local function load_origin_x()
    local value = save.get(key_origin())
    if value == nil then
        return entity.x
    end
    return value
end

local function load_dir()
    local value = save.get(key_dir())
    if value == nil then
        return 1.0
    end
    if value >= 0 then
        return 1.0
    end
    return -1.0
end

function on_start()
    -- Demo: sempre começa patrulha nova ao entrar em Game.
    save.set(key_origin(), entity.x)
    save.set(key_dir(), 1.0)
    game.log("Inimigo: patrulha inicializada em x=" .. tostring(entity.x))
end

function on_update(dt)
    local origin_x = load_origin_x()
    local dir = load_dir()

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
    save.set(key_dir(), dir)

    local cols = game.get_collisions()
    for _, name in ipairs(cols) do
        if name == "Player" then
            game.log("Inimigo tocou o Player!")
            break
        end
    end
end
