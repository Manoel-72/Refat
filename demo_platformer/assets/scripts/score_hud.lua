-- =============================================================
--  score_hud.lua  —  HUD simples de pontuação para o demo oficial.
--  Lê o score salvo e atualiza o TextLabel da própria entidade.
-- =============================================================

local last_score = nil

function on_start()
    local score = save.get("score") or 0
    last_score = score
    entity.set_text("Score: " .. tostring(score))
end

function on_update(dt)
    local score = save.get("score") or 0
    if score ~= last_score then
        last_score = score
        entity.set_text("Score: " .. tostring(score))
    end
end
