-- =============================================================
--  enemy_patrol.lua  —  RS2BR-Engine Exemplo Oficial #2
--  Inimigo que patrulha entre dois pontos e muda de direção
--  ao colidir com paredes ou ao chegar no limite configurado.
--
--  API usada:
--    entity.x, entity.y, entity.vx, entity.vy, entity.grounded
--    entity.set_velocity(vx, vy)
--    game.get_collisions(), game.collision_enter(name)
--    game.raycast(ox, oy, dx, dy, dist)  → {hit, x, y, dist, name}
--    game.log(msg), game.delta_time
-- =============================================================

local SPEED         = 90      -- pixels/s
local PATROL_DIST   = 140     -- distância máxima do ponto inicial
local dir           = 1       -- 1 = direita, -1 = esquerda
local origin_x      = 0
local ray_timer     = 0       -- acumulador para raycast periódico
local reversal_cd   = 0       -- cooldown de reversão (evita tremor)

function on_start()
    origin_x = entity.x
    game.log("Inimigo: patrulhando a partir de x=" .. origin_x)
end

function on_update(dt)
    ray_timer   = ray_timer   + dt
    reversal_cd = reversal_cd - dt

    -- ── reversão por limite de patrol ──────────────────
    local dist_from_origin = entity.x - origin_x
    if dist_from_origin > PATROL_DIST and dir == 1 then
        dir = -1
        reversal_cd = 0.3
    elseif dist_from_origin < -PATROL_DIST and dir == -1 then
        dir = 1
        reversal_cd = 0.3
    end

    -- ── reversão por colisão lateral (raycast a cada 0.15s) ──
    -- Só reverte se o cooldown zerou (evita oscilação ao ficar preso)
    if ray_timer >= 0.15 and reversal_cd <= 0 then
        ray_timer = 0
        -- Lança raio 28px à frente na direção atual
        local result = game.raycast(entity.x, entity.y, dir, 0, 28)
        if result.hit and result.name ~= entity.name then
            dir = -dir
            reversal_cd = 0.4   -- espera 0.4s antes de poder reverter de novo
            game.log("Inimigo: bateu em '" .. result.name .. "' — revertendo")
        end
    end

    -- ── aplica velocidade ─────────────────────────────
    entity.set_velocity(SPEED * dir, entity.vy)

    -- ── detecção do player ────────────────────────────
    local cols = game.get_collisions()
    for _, name in ipairs(cols) do
        if name == "Player" then
            game.log("Inimigo tocou o Player!")
        end
    end
end
