-- =============================================================
--  coin_collector.lua  —  RS2BR-Engine Exemplo Oficial #3
--  Moeda coletável: pulsa visualmente e some ao ser tocada.
--  Demonstra: trigger, save, visible, elapsed_time.
--
--  API usada:
--    entity.x, entity.y, entity.visible, entity.name
--    entity.set_visible(bool)
--    game.get_collisions(), game.elapsed_time, game.log(msg)
--    save.get(key), save.set(key, value)
-- =============================================================

local collected = false
local bob_speed = 2.5       -- velocidade do efeito de flutuação
local base_y    = 0

function on_start()
    base_y = entity.y
    game.log("Moeda '" .. entity.name .. "' pronta em y=" .. base_y)
end

function on_update(dt)
    -- Já coletada — não faz nada
    if collected then return end

    -- ── efeito de flutuação (bob) ─────────────────────
    local t = game.elapsed_time
    entity.set_position(entity.x, base_y + math.sin(t * bob_speed) * 6)

    -- ── detecta colisão com o player via trigger ───────
    local cols = game.get_collisions()
    for _, name in ipairs(cols) do
        if name == "Player" then
            collected = true
            entity.set_visible(false)

            -- Incrementa score global no save
            local current = save.get("score") or 0
            save.set("score", current + 1)

            game.log("Moeda coletada! score agora = " .. (current + 1))
            return
        end
    end
end
