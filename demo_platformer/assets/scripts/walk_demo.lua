local SPEED = 220.0

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
