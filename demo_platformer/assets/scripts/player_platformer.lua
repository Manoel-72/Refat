-- =============================================================
--  player_platformer.lua  —  RS2BR-Engine Exemplo Oficial #1
--  Controle de personagem plataformer completo.
--
--  Controles:
--    A / Seta Esquerda  → mover esquerda
--    D / Seta Direita   → mover direita
--    Espaço / W / Seta Cima → pular (só se no chão)
--
--  API usada:
--    entity.x, entity.y, entity.vx, entity.vy, entity.grounded
--    entity.set_velocity(vx, vy)
--    entity.apply_impulse(ix, iy)
--    entity.play_anim(clip)
--    input.key_held(name), input.key_pressed(name)
--    game.delta_time, game.log(msg)
--    save.get(key), save.set(key, value)
-- =============================================================

local SPEED       = 180     -- pixels/s horizontal
local JUMP_FORCE  = 380     -- impulso de pulo (pixels/s)
local score       = 0

function on_start()
    -- Recupera pontuação salva de sessões anteriores
    score = save.get("score") or 0
    game.log("Player iniciado | score salvo: " .. tostring(score))
end

function on_update(dt)
    -- ── movimento horizontal ─────────────────────────
    local vx = 0
    if input.key_held("A") or input.key_held("Left") then
        vx = -SPEED
    end
    if input.key_held("D") or input.key_held("Right") then
        vx = SPEED
    end

    -- Mantém velocidade vertical atual (gravidade já aplicada pelo engine)
    entity.set_velocity(vx, entity.vy)

    -- ── pulo ─────────────────────────────────────────
    local wants_jump = input.key_pressed("Space")
                    or input.key_pressed("W")
                    or input.key_pressed("Up")
    if wants_jump and entity.grounded then
        -- apply_impulse soma à velocidade atual instantaneamente
        entity.apply_impulse(0, JUMP_FORCE)
        game.log("Pulo! pos=(" .. entity.x .. "," .. entity.y .. ")")
    end

    -- ── coleta moedas por trigger ─────────────────────
    local cols = game.get_collisions()
    for _, name in ipairs(cols) do
        if name == "Moeda" then
            score = score + 1
            save.set("score", score)
            game.log("Moeda coletada! Total: " .. score)
        end
    end

    -- ── animação básica ──────────────────────────────
    if math.abs(vx) > 0 then
        entity.play_anim("run")
    elseif not entity.grounded then
        entity.play_anim("jump")
    else
        entity.play_anim("idle")
    end
end
