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

local SPEED       = 90      -- pixels/s
local PATROL_DIST = 140     -- distância máxima do ponto inicial
local dir         = 1       -- 1 = direita, -1 = esquerda
local origin_x    = 0
local ray_timer   = 0       -- acumulador para raycast periódico

function on_start()
    origin_x = entity.x
    game.log("Inimigo: patrulhando a partir de x=" .. origin_x)
end

function on_update(dt)
    ray_timer = ray_timer + dt

    -- ── reversão por limite de patrol ──────────────────
    local dist_from_origin = entity.x - origin_x
    if dist_from_origin > PATROL_DIST then
        dir = -1
    elseif dist_from_origin < -PATROL_DIST then
        dir = 1
    end

    -- ── reversão por colisão lateral (raycast a cada 0.1s) ──
    if ray_timer >= 0.10 then
        ray_timer = 0
        -- Lança raio 24px à frente na direção atual
        local result = game.raycast(entity.x, entity.y, dir, 0, 24)
        if result.hit and result.name ~= entity.name then
            dir = -dir
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
