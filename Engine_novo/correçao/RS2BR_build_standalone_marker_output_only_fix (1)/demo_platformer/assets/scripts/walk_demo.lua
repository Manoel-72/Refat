local SPEED = 220.0
local JUMP_FORCE = 300.0
function on_start()
    entity.set_anim_state("walk")
    entity.set_anim_playing(false)
end

function on_update(dt)
    local vx = 0.0
    if input.key_held("A") or input.key_held("Left") then
        vx = -SPEED
    end
    if input.key_held("D") or input.key_held("Right") then
        vx = SPEED
    end

    -- Pulo
    if (input.key_pressed("Space") or input.key_pressed("W") or input.key_pressed("Up"))
       and entity.grounded then
        entity.apply_impulse(0.0, JUMP_FORCE)
        game.spawn_particle(entity.x, entity.y - 18.0, 0.0, -18.0, 0.35, 0.85, 0.90, 1.0, 0.7)
    end

    -- Animação
    if entity.grounded then
        if math.abs(vx) > 0.1 then entity.set_anim("run") else entity.set_anim("idle") end
    else
        entity.set_anim("jump")
    end


    entity.set_velocity(vx, entity.vy)

    if vx < -0.1 then
        entity.flip_x(true)
        entity.set_anim_state("walk")
        entity.set_anim_playing(true)
    elseif vx > 0.1 then
        entity.flip_x(false)
        entity.set_anim_state("walk")
        entity.set_anim_playing(true)
    else
        entity.set_anim_state("walk")
        entity.set_anim_playing(false)
    end
end
