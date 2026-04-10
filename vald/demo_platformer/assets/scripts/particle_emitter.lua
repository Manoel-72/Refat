-- Prefab simples para particulas automáticas.
-- Duplique este script ou o prefab ParticleSourceBasic para criar outro emissor.
local RATE = 0.10

function on_start()
    state.set("cooldown", 0.0)
end

function on_update(dt)
    local cooldown = state.get("cooldown", 0.0) - dt
    if cooldown <= 0.0 then
        cooldown = RATE
        game.spawn_particle(entity.x, entity.y, 0.0, -12.0, 0.45, 0.30, 0.85, 1.0, 0.55)
        game.spawn_particle(entity.x + 5.0, entity.y + 1.0, 7.0, -9.0, 0.30, 0.85, 0.95, 1.0, 0.28)
    end
    state.set("cooldown", cooldown)
end
