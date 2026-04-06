-- =============================================================
--  score_hud.lua  —  HUD simples de pontuação para o demo oficial.
--  Lê o score da sessão atual e atualiza o TextLabel da entidade.
-- =============================================================

local last_score = nil

function on_start()
    local score = session.get("score", 0)
    last_score = score
    entity.set_text("Score: " .. tostring(score))
end

function on_update(dt)
    local score = session.get("score", 0)
    if score ~= last_score then
        last_score = score
        entity.set_text("Score: " .. tostring(score))
    end
end
