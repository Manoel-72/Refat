-- =============================================================
-- collision_probe.lua — entidade de diagnóstico de colisão
--
-- Valida os três eventos em separado e loga cada transição.
-- Útil como prova de regressão de collision_enter/stay/exit.
-- =============================================================

function on_start()
    state.set("phase", "idle")
    game.log("Probe pronto.")
end

function on_update(dt)
    local phase = state.get("phase", "idle")

    if game.collision_enter("Player") then
        state.set("phase", "contact")
        game.log("[Probe] ENTER — Player entrou")
    end

    if game.collision_stay("Player") then
        -- Loga só na mudança de fase para não inundar o console
        if phase ~= "contact" then
            state.set("phase", "contact")
        end
    end

    if game.collision_exit("Player") then
        state.set("phase", "idle")
        game.log("[Probe] EXIT — Player saiu")
    end
end
