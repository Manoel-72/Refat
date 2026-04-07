-- Variáveis locais persistem entre frames na VM cacheada
local base_x   = nil   -- definido no primeiro on_update
local base_y   = nil
local dir      = 1.0   -- 1 = direita, -1 = esquerda
local HALF     = 90.0  -- alcance em px de cada lado
local SPEED    = 50.0  -- px/s

function on_start()
    entity.add_tag("enemy")
    entity.set_hp(25)
end

function on_update(dt)
    -- captura posição inicial apenas uma vez
    if base_x == nil then
        base_x = entity.x
        base_y = entity.y
    end

    local new_x = entity.x + dir * SPEED * dt

    if new_x >= base_x + HALF then
        new_x = base_x + HALF
        dir = -1.0
        entity.flip_x(true)
    elseif new_x <= base_x - HALF then
        new_x = base_x - HALF
        dir = 1.0
        entity.flip_x(false)
    end

    entity.set_position(new_x, base_y)
    entity.set_velocity(0.0, 0.0)
end
