local registered = false
local sent_boot = false
local sent_probe_a = false
local sent_probe_b = false

local function emit_event(name, data)
    if event ~= nil and type(event.emit) == "function" then
        event.emit(name, data)
        return true
    end
    return false
end

function on_start()
    if event ~= nil and type(event.listen) == "function" then
        event.listen("validation_ping", function(data)
            game.log("[EVENT] validation_ping recebido: " .. tostring(data))
        end)

        event.listen("validation_collision", function(data)
            game.log("[EVENT] validation_collision recebido: " .. tostring(data))
        end)

        event.listen("validation_timer", function(data)
            game.log("[EVENT] validation_timer recebido: " .. tostring(data))
        end)

        registered = true
        game.log("[EVENT] listeners registrados")
    else
        game.log("[EVENT] API event.* não disponível nesta build")
    end
end

function on_update(dt)
    if registered and not sent_boot and type(game.elapsed_time) == "function" and game.elapsed_time() > 0.25 then
        sent_boot = true
        emit_event("validation_ping", "boot_ok")
    end

    if registered and type(game.get_current_collision_ids) == "function" then
        local ids = game.get_current_collision_ids()
        if ids ~= nil then
            for _, id in ipairs(ids) do
                if id == "ent-reg-trigger-a" and not sent_probe_a then
                    sent_probe_a = true
                    emit_event("validation_collision", "trigger_a")
                elseif id == "ent-reg-trigger-b" and not sent_probe_b then
                    sent_probe_b = true
                    emit_event("validation_collision", "trigger_b")
                end
            end
        end
    end
end
