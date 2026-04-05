--[[
Script Lua base — RS2BR-Engine V0.8.7

Este arquivo já nasce com uma estrutura segura para começar.
Use on_start() para inicialização e on_update(dt) para lógica por frame.

APIs disponíveis no runtime:
- entity:get_pos() / entity:set_pos(x, y)
- entity:get_vel() / entity:set_vel(vx, vy)
- entity:set_rotation(rot)
- entity:set_visible(visivel)
- entity:play_anim("nome_do_clip")
- input:key_held("A")
- input:key_pressed("Space")
- input:mouse_pos()
- game:change_scene("assets/scenes/main.scene.json")
- game:log("mensagem")
- save:get("chave") / save:set("chave", valor) / save:has("chave") / save:remove("chave")

Dica:
Comece pequeno. Primeiro mova a entidade ou troque uma animação.
Depois adicione mudança de cena, save e outras regras.
]]

local SPEED = 140.0

function on_start()
    game:log("Lua iniciado: entidade pronta")
end

function on_update(dt)
    local vx, vy = 0.0, 0.0

    if input:key_held("A") then vx = vx - SPEED end
    if input:key_held("D") then vx = vx + SPEED end
    if input:key_held("W") then vy = vy + SPEED end
    if input:key_held("S") then vy = vy - SPEED end

    entity:set_vel(vx, vy)

    if input:key_pressed("Space") then
        game:log("Space pressionado")
    end

    -- Exemplo opcional de animação:
    -- if vx ~= 0.0 or vy ~= 0.0 then
    --     entity:play_anim("run")
    -- else
    --     entity:play_anim("idle")
    -- end
end
