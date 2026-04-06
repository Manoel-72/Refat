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
    local t = type(game.elapsed_time) == "function" and game.elapsed_time() or game.elapsed_time
    local scale = 1.0 + math.sin(t * 3.0) * 0.1
    -- (scale_x/y são somente leitura na V0.9.1, mas o efeito de "entrar/sair" é via log)

    if game.collision_enter("Player") then
        local n = session.get("pickups", 0)
        session.set("pickups", n + 1)
        game.log("Pickup coletado! Total: " .. (n + 1))
        entity.destroy()  -- remove do mundo no fim do frame
    end
end
