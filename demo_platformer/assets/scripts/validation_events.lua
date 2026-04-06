function on_start()
    state.set("event_registered", false)
    state.set("event_emit_count", 0)
    state.set("event_receive_count", 0)
    state.set("event_last_emit", "none")
    state.set("event_last_received", "none")
    state.set("event_boot_sent", false)
    state.set("event_probe_a_sent", false)
    state.set("event_probe_b_sent", false)

    if event ~= nil and type(event.listen) == "function" then
        event.listen("validation_ping", function(data)
            state.set("event_receive_count", state.get("event_receive_count", 0) + 1)
            state.set("event_last_received", "validation_ping:" .. tostring(data))
            game.log("[EVENT] validation_ping recebido: " .. tostring(data))
        end)

        event.listen("validation_collision", function(data)
            state.set("event_receive_count", state.get("event_receive_count", 0) + 1)
            state.set("event_last_received", "validation_collision:" .. tostring(data))
            game.log("[EVENT] validation_collision recebido: " .. tostring(data))
        end)

        event.listen("validation_timer", function(data)
            state.set("event_receive_count", state.get("event_receive_count", 0) + 1)
            state.set("event_last_received", "validation_timer:" .. tostring(data))
            game.log("[EVENT] validation_timer recebido: " .. tostring(data))
        end)

        state.set("event_registered", true)
        game.log("[EVENT] listeners registrados")
    else
        state.set("event_last_received", "event_api_missing")
        game.log("[EVENT] API event.* não disponível nesta build")
    end
end

local function emit_event(name, data)
    if event ~= nil and type(event.emit) == "function" then
        event.emit(name, data)
        state.set("event_emit_count", state.get("event_emit_count", 0) + 1)
        state.set("event_last_emit", name .. ":" .. tostring(data))
        return true
    end
    return false
end

function on_update(dt)
    if state.get("event_registered", false) and not state.get("event_boot_sent", false) and type(game.elapsed_time) == "function" and game.elapsed_time() > 0.25 then
        state.set("event_boot_sent", true)
        emit_event("validation_ping", "boot_ok")
    end

    if state.get("event_registered", false) and type(game.get_current_collision_ids) == "function" then
        local ids = game.get_current_collision_ids()
        if ids ~= nil then
            for _, id in ipairs(ids) do
                if id == "ent-reg-trigger-a" and not state.get("event_probe_a_sent", false) then
                    state.set("event_probe_a_sent", true)
                    emit_event("validation_collision", "trigger_a")
                elseif id == "ent-reg-trigger-b" and not state.get("event_probe_b_sent", false) then
                    state.set("event_probe_b_sent", true)
                    emit_event("validation_collision", "trigger_b")
                end
            end
        end
    end

    entity.set_text(
        "[EVENT] reg=" .. tostring(state.get("event_registered", false)) ..
        " | emit=" .. tostring(state.get("event_emit_count", 0)) ..
        " | recv=" .. tostring(state.get("event_receive_count", 0)) ..
        " | last=" .. tostring(state.get("event_last_received", "none"))
    )
end
