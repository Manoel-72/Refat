function on_update(dt)
    local hp = session.get("player_hp", 5)
    local dead = session.get("enemies_dead", 0)
    if entity.name == "HUD_Status" then
        entity.set_text("Vida: " .. hp .. "   Inimigos: " .. (4 - dead) .. "/4   WASD/Setas mover | Espaço atirar")
    elseif entity.name == "HUD_End" then
        if session.get("victory", false) then
            entity.set_text("VITÓRIA! Você eliminou todos os inimigos.")
        elseif session.get("game_over", false) then
            entity.set_text("GAME OVER! Você morreu.")
        else
            entity.set_text("")
        end
    end
end
