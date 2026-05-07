local registered = false
local boot_sent = false
local emit_total = 0
local recv_total = 0
local emit_ping = 0
local recv_ping = 0
local emit_pulse = 0
local recv_pulse = 0
local pulse_count = 0
local next_pulse_at = 1.0
local last_emit = "none"
local last_recv = "none"

local function elapsed_now()
    if type(game.elapsed_time) == "function" then
        return game.elapsed_time()
    end
    if type(game.get_elapsed_time) == "function" then
        return game.get_elapsed_time()
    end
    return 0.0
end

local function mark_emit(name, data)
    emit_total = emit_total + 1
    if name == "validation_ping" then
        emit_ping = emit_ping + 1
    elseif name == "validation_pulse" then
        emit_pulse = emit_pulse + 1
    end
    last_emit = tostring(name) .. ":" .. tostring(data)
end

local function mark_recv(name, data)
    recv_total = recv_total + 1
    if name == "validation_ping" then
        recv_ping = recv_ping + 1
    elseif name == "validation_pulse" then
        recv_pulse = recv_pulse + 1
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
        end)
        event.listen("validation_pulse", function(data)
            mark_recv("validation_pulse", data)
        end)
        registered = true
    end
end

function on_update(dt)
    local elapsed = elapsed_now()

    if registered and not boot_sent and elapsed >= 0.25 then
        boot_sent = true
        safe_emit("validation_ping", "boot_ok")
    end

    while registered and elapsed >= next_pulse_at do
        pulse_count = pulse_count + 1
        safe_emit("validation_pulse", "pulse_" .. tostring(pulse_count))
        next_pulse_at = next_pulse_at + 1.0
    end

    entity.set_text(
        "[EVENT] reg=" .. tostring(registered) ..
        " | emit=" .. tostring(emit_total) ..
        " | recv=" .. tostring(recv_total) ..
        "\n[EVENT] ping e/r=" .. tostring(emit_ping) .. "/" .. tostring(recv_ping) ..
        " | pulse e/r=" .. tostring(emit_pulse) .. "/" .. tostring(recv_pulse) ..
        "\n[EVENT] last_emit=" .. tostring(last_emit) ..
        " | last_recv=" .. tostring(last_recv)
    )
end
