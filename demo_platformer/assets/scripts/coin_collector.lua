-- =============================================================
--  coin_collector.lua  —  RS2BR-Engine Exemplo Oficial #3
--  Moeda coletável: pulsa visualmente e some ao ser tocada.
--
--  Demo V0.9.1:
--  - score vale só para a partida atual via session.*
--  - estado local da moeda vive em state.*
-- =============================================================

local BOB_SPEED = 2.5

function on_start()
    state.set("collected", false)
    state.set("base_y", entity.y)
    entity.set_visible(true)
    game.log("Moeda '" .. entity.name .. "' pronta em y=" .. entity.y)
end

function on_update(dt)
    if state.get("collected", false) then
        return
    end

    local base_y = state.get("base_y", entity.y)
    local t = type(game.elapsed_time) == "function" and game.elapsed_time() or game.elapsed_time
    entity.set_position(entity.x, base_y + math.sin(t * BOB_SPEED) * 6)

    local cols = game.get_collisions()
    for _, name in ipairs(cols) do
        if name == "Player" then
            state.set("collected", true)
            entity.set_visible(false)

            local current = session.get("score", 0)
            local next_score = current + 1
            session.set("score", next_score)

            game.log("Moeda coletada! score agora = " .. next_score)
            return
        end
    end
end
