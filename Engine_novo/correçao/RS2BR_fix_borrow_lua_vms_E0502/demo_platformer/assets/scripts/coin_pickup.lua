-- Variável local persiste entre frames na VM cacheada
local collected = false
local base_y    = nil
local t         = 0.0
local BOB_AMP   = 6.0   -- pixels de amplitude
local BOB_SPEED = 2.5   -- ciclos por segundo

function on_start()
    -- base_y capturado no primeiro update
end

function on_update(dt)
    if collected then return end

    -- Animação de bobbing (sobe/desce)
    if base_y == nil then base_y = entity.y end
    t = t + dt * BOB_SPEED
    local offset_y = math.sin(t) * BOB_AMP
    entity.set_position(entity.x, base_y + offset_y)

    -- Coleta
    if game.collision_enter("Player") or game.collision_stay("Player") then
        collected = true
        local new_score = session.get("score", 0) + 1
        session.set("score", new_score)
        game.log("MOEDA! score = " .. tostring(new_score))
        for i = 1, 7 do
            local vx = (i - 4) * 16.0
            game.spawn_particle(entity.x, entity.y, vx, -18.0 - i * 1.5,
                0.40, 1.0, 0.88, 0.25, 0.50)
        end
        entity.destroy()
    end
end
