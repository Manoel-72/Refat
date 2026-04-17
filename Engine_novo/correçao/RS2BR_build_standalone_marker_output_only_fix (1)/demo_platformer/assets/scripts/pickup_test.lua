local function time_now()
    if type(game.get_elapsed_time) == "function" then
        return game.get_elapsed_time()
    end
    if type(game.elapsed_time) == "function" then
        return game.elapsed_time()
    end
    if type(game.elapsed_time) == "table" and type(game.elapsed_time.value) == "number" then
        return game.elapsed_time.value
    end
    if type(game.elapsed_time_value) == "number" then
        return game.elapsed_time_value
    end
    if type(game.elapsed_time) == "number" then
        return game.elapsed_time
    end
    return 0
end

-- =============================================================
-- pickup_test.lua — pickup que testa entity.destroy()
--
-- Valida:
--   - collision_enter real (frame exato de contato)
--   - entity.destroy() seguro (remoção no fim do frame)
--   - session.* para contagem de pickups
-- =============================================================

function on_start()
    entity.set_collision_enabled(true)
    entity.set_visible(true)
end

function on_update(dt)
    -- Pulsa levemente para indicar que está ativo
    local t = time_now()
    local scale = 1.0 + math.sin(t * 3.0) * 0.1
    -- (scale_x/y são somente leitura na V0.9.1, mas o efeito de "entrar/sair" é via log)

    if game.collision_enter("Player") then
        local n = session.get("pickups", 0)
        session.set("pickups", n + 1)
        game.log("Pickup coletado! Total: " .. (n + 1))
        entity.destroy()  -- remove do mundo no fim do frame
    end
end
