local every_count = 0
local started = false

local function emit_timer_event(message)
    if event ~= nil and type(event.emit) == "function" then
        event.emit("validation_timer", message)
    end
end

function on_start()
    if type(game.after) == "function" then
        game.after(1.0, function()
            game.log("[TIMER] after(1.0) executou")
            emit_timer_event("after_1s")
        end)
    else
        game.log("[TIMER] game.after não disponível nesta build")
    end

    if type(game.every) == "function" then
        game.every(0.75, function()
            every_count = every_count + 1
            game.log("[TIMER] every(0.75) tick=" .. tostring(every_count))
            emit_timer_event("every_tick_" .. tostring(every_count))
        end)
    else
        game.log("[TIMER] game.every não disponível nesta build")
    end

    started = true
end

function on_update(dt)
    if started then
        entity.set_text("[TIMER] every_count=" .. tostring(every_count))
    end
end
