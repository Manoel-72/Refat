function on_start()
    entity.set_color(0.98, 0.98, 0.98, 1.0)
end

function on_update(dt)
    local hp    = session.get("player_hp", 100)
    local score = session.get("score", 0)
    local text = "HP: " .. tostring(hp) .. "  |  Score: " .. tostring(score)
    text = text .. "\nMover: A/D  Pular: Espaco  Spawn inimigo: Enter"
    entity.set_text(text)
end
