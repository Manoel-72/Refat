function on_start()
    entity.add_tag("player")
    game.log("Lua V0.9.3 pronto")
end

function on_update(dt)
    if input.key_pressed("P") then
        game.spawn_prefab("enemy_basic", entity.x + 96, entity.y)
        game.spawn_particle(entity.x + 32, entity.y + 16, 0, 36, 0.7, 1.0, 0.85, 0.2, 1.2)
    end

    if input.key_pressed("O") then
        game.spawn_entity("pickup", entity.x + 48, entity.y + 24)
    end
end
