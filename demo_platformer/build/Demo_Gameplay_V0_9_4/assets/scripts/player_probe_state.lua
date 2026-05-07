-- player_probe_state.lua
-- Script secundário de validação para a mesma entidade Player.
-- Exercita state.* por entidade + script sem alterar o gameplay principal.

function on_start()
    state.set("probe_ticks", 0)
    game.log("Probe secundário iniciado")
end

function on_update(dt)
    local ticks = state.get("probe_ticks", 0)
    state.set("probe_ticks", ticks + 1)
end
