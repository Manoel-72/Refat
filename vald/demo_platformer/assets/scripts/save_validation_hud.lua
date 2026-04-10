-- =============================================================
-- save_validation_hud.lua — HUD oficial de validação da separação
--
-- Prova no runtime:
--   - session.score  -> score da rodada atual
--   - session.pickups -> pickups da rodada atual
--   - save.best_score -> melhor score persistente
--
-- Observação:
--   save.* atualiza o save em memória. Para persistir em disco,
--   use a ação de salvar do runtime/editor e depois Continue.
-- =============================================================

local last_text = nil

local function update_best_score(round_score)
    local current_best = save.get("best_score")
    if current_best == nil then
        current_best = 0
    end

    if round_score > current_best then
        current_best = round_score
        save.set("best_score", current_best)
    end

    return current_best
end

local function rebuild_label()
    local round_score = session.get("score", 0)
    local round_pickups = session.get("pickups", 0)
    local best_score = update_best_score(round_score)

    return "Sessão(score=" .. tostring(round_score)
        .. ", pickups=" .. tostring(round_pickups)
        .. ") | Save(best=" .. tostring(best_score)
        .. ")"
end

function on_start()
    local text = rebuild_label()
    last_text = text
    entity.set_text(text)
end

function on_update(dt)
    local text = rebuild_label()
    if text ~= last_text then
        last_text = text
        entity.set_text(text)
    end
end
