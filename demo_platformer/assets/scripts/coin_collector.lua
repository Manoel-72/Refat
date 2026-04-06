-- =============================================================
--  coin_collector.lua  —  RS2BR-Engine Exemplo Oficial #3
--  Moeda coletável: pulsa visualmente e some ao ser tocada.
--
--  Demo V0.9.1:
--  - score vale só para a partida atual
--  - moeda não persiste entre entradas no Game
-- =============================================================

local collected = false
local bob_speed = 2.5
local base_y    = 0

function on_start()
    collected = false
    base_y = entity.y
    entity.set_visible(true)
    game.log("Moeda '" .. entity.name .. "' pronta em y=" .. base_y)
end

function on_update(dt)
    if collected then return end

    local t = game.elapsed_time
    entity.set_position(entity.x, base_y + math.sin(t * bob_speed) * 6)

    local cols = game.get_collisions()
    for _, name in ipairs(cols) do
        if name == "Player" then
            collected = true
            entity.set_visible(false)

            local current = save.get("score") or 0
            save.set("score", current + 1)

            game.log("Moeda coletada! score agora = " .. (current + 1))
            return
        end
    end
end
