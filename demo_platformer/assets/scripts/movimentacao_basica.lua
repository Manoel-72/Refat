-- Variáveis locais persistem entre frames na VM cacheada
local hit_cooldown   = 0.0
local spawn_cooldown = 0.0

local SPEED      = 190.0
local JUMP_FORCE = 420.0

local function spawn_enemy()
    local sx = entity.x + 96.0
    local sy = entity.y
    game.spawn_prefab("assets/prefabs/enemy_basic.prefab.json", sx, sy)
    game.spawn_particle(sx,       sy - 8.0,  0.0,  -10.0, 0.45, 1.0, 0.55, 0.30, 0.9)
    game.spawn_particle(sx+10.0,  sy - 4.0,  8.0,  -13.0, 0.30, 1.0, 0.80, 0.35, 0.5)
end

function on_start()
    entity.set_hp(100)
    session.set("player_hp", 100)
    if not session.has("score") then
        session.set("score", 0)
    end
end

function on_update(dt)
    -- Cooldowns
    if hit_cooldown   > 0.0 then hit_cooldown   = hit_cooldown   - dt end
    if spawn_cooldown > 0.0 then spawn_cooldown  = spawn_cooldown - dt end

    -- Movimento
    local vx = 0.0
    if input.key_held("A") or input.key_held("Left")  then vx = -SPEED end
    if input.key_held("D") or input.key_held("Right") then vx =  SPEED end
    entity.set_velocity(vx, entity.vy)

    if     vx < -0.1 then entity.flip_x(true)
    elseif vx >  0.1 then entity.flip_x(false) end

    -- Pulo
    if (input.key_pressed("Space") or input.key_pressed("W") or input.key_pressed("Up"))
       and entity.grounded then
        entity.apply_impulse(0.0, JUMP_FORCE)
        game.spawn_particle(entity.x, entity.y - 18.0,
            0.0, -18.0, 0.35, 0.85, 0.90, 1.0, 0.7)
    end

    -- Animação
    if entity.grounded then
        if math.abs(vx) > 0.1 then entity.set_anim("run")
        else entity.set_anim("idle") end
    else
        entity.set_anim("jump")
    end

    -- Spawn inimigo (só Enter — "E" não está mapeado no engine)
    if spawn_cooldown <= 0.0 and input.key_pressed("Enter") then
        spawn_enemy()
        spawn_cooldown = 0.5
    end

    -- Dano ao tocar inimigo
    if hit_cooldown <= 0.0 then
        if game.collision_enter("Enemy") or game.collision_stay("Enemy") then
            entity.damage(10)
            -- lê HP pelo campo direto (snapshot deste frame, será 1 frame atrasado no HUD — aceitável)
            game.camera_shake(2.0, 0.18)
            for i = 1, 4 do
                game.spawn_particle(entity.x, entity.y - 8.0,
                    (i - 2.5) * 14.0, -10.0 - i * 2.0,
                    0.25, 1.0, 0.35, 0.35, 0.45)
            end
            hit_cooldown = 0.75
            -- Atualiza sessão com hp atual (snapshot pré-damage; o próximo frame reflete)
            session.set("player_hp", entity.hp - 10)
        end
    else
        -- fora de cooldown: sincroniza HP normalmente
        session.set("player_hp", entity.hp)
    end
end
