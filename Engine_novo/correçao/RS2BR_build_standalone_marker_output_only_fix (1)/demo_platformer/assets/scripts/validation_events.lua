local emit_total = 0
local emit_ping = 0
local emit_timer = 0
local recv_total = 0
local recv_ping = 0
local recv_timer = 0
local recv_collision = 0
local timer_tick_count = 0
local after_done = false
local boot_sent = false
local listeners_registered = false
local last_emit = "none"
local last_recv = "none"

local function mark_emit(name, data)
    emit_total = emit_total + 1
    if name == "validation_ping" then
        emit_ping = emit_ping + 1
    elseif name == "validation_timer" then
        emit_timer = emit_timer + 1
    end
    last_emit = tostring(name) .. ":" .. tostring(data)
end

local function mark_recv(name, data)
    recv_total = recv_total + 1
    if name == "validation_ping" then
        recv_ping = recv_ping + 1
    elseif name == "validation_timer" then
        recv_timer = recv_timer + 1
    elseif name == "validation_collision" then
        recv_collision = recv_collision + 1
    end
    last_recv = tostring(name) .. ":" .. tostring(data)
end

local function safe_emit(name, data)
    if event ~= nil and type(event.emit) == "function" then
        mark_emit(name, data)
        event.emit(name, data)
        return true
    end
    return false
end

function on_start()
    if event ~= nil and type(event.listen) == "function" then
        event.listen("validation_ping", function(data)
            mark_recv("validation_ping", data)
            game.log("[EVENT] validation_ping recebido: " .. tostring(data))
        end)

        event.listen("validation_timer", function(data)
            mark_recv("validation_timer", data)
            game.log("[EVENT] validation_timer recebido: " .. tostring(data))
        end)

        event.listen("validation_collision", function(data)
            mark_recv("validation_collision", data)
            game.log("[EVENT] validation_collision recebido: " .. tostring(data))
        end)

        listeners_registered = true
        game.log("[EVENT] listeners registrados (hub local)")
    else
        game.log("[EVENT] API event.* não disponível nesta build")
    end

    if type(game.after) == "function" then
        game.after(1.0, function()
            after_done = true
            game.log("[EVENT] after(1.0) do hub executou")
            safe_emit("validation_timer", "after_1s")
        end)
    end

    if type(game.every) == "function" then
        game.every(1.0, function()
            timer_tick_count = timer_tick_count + 1
            safe_emit("validation_timer", "hub_tick_" .. tostring(timer_tick_count))
        end)
    end
end

function on_update(dt)
    if listeners_registered and not boot_sent and type(game.elapsed_time) == "function" and game.elapsed_time() > 0.25 then
        boot_sent = true
        safe_emit("validation_ping", "boot_ok")
    end

    entity.set_text(
        "[EVENT] reg=" .. tostring(listeners_registered) ..
        " | emit_local=" .. tostring(emit_total) ..
        " (ping=" .. tostring(emit_ping) .. ", timer=" .. tostring(emit_timer) .. ")" ..
        " | recv_total=" .. tostring(recv_total) ..
        " (ping=" .. tostring(recv_ping) .. ", timer=" .. tostring(recv_timer) .. ", collision=" .. tostring(recv_collision) .. ")" ..
        "\n[EVENT-LAST] emit=" .. tostring(last_emit) .. " | recv=" .. tostring(last_recv) ..
        "\n[EVENT-TIMER] hub_ticks=" .. tostring(timer_tick_count) .. " | after=" .. tostring(after_done)
    )
end
