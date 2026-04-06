-- =============================================================
--  player_platformer.lua  —  RS2BR-Engine Exemplo Oficial #1
--  Controle de personagem plataformer completo.
--
--  Demo V0.9.1:
--  - ao iniciar a fase, o score da partida vive em session.*
--  - novo jogo limpa a sessão no runtime
-- =============================================================

local SPEED       = 180     -- pixels/s horizontal
local JUMP_FORCE  = 380     -- impulso de pulo (pixels/s)

function on_start()
    if not session.has("score") then
        session.set("score", 0)
    end
    game.log("Player iniciado | sessão atual com score = " .. session.get("score", 0))
end

function on_update(dt)
    local vx = 0
    if input.key_held("A") or input.key_held("Left") then
        vx = -SPEED
    end
    if input.key_held("D") or input.key_held("Right") then
        vx = SPEED
    end

    entity.set_velocity(vx, entity.vy)

    local wants_jump = input.key_pressed("Space")
                    or input.key_pressed("W")
                    or input.key_pressed("Up")
    if wants_jump and entity.grounded then
        entity.apply_impulse(0, JUMP_FORCE)
        game.log("Pulo! pos=(" .. entity.x .. "," .. entity.y .. ")")
    end

    if math.abs(vx) > 0 then
        entity.play_anim("run")
    elseif not entity.grounded then
        entity.play_anim("jump")
    else
        entity.play_anim("idle")
    end
end
