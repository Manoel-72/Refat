function on_start()
    state.set("timer_started", true)
    state.set("timer_every_count", state.get("timer_every_count", 0))
    state.set("timer_after_done", state.get("timer_after_done", false))
    state.set("timer_last_message", state.get("timer_last_message", "armed"))

    if type(game.after) == "function" then
        game.after(1.0, function()
            state.set("timer_after_done", true)
            state.set("timer_last_message", "after_1s")
            game.log("[TIMER] after(1.0) executou")
            if event ~= nil and type(event.emit) == "function" then
                event.emit("validation_timer", "after_1s")
            end
        end)
    else
        state.set("timer_last_message", "after_missing")
        game.log("[TIMER] game.after não disponível nesta build")
    end

    if type(game.every) == "function" then
        game.every(0.75, function()
            local count = state.get("timer_every_count", 0) + 1
            state.set("timer_every_count", count)
            state.set("timer_last_message", "every_tick_" .. tostring(count))
            game.log("[TIMER] every(0.75) tick=" .. tostring(count))
            if event ~= nil and type(event.emit) == "function" then
                event.emit("validation_timer", "every_tick_" .. tostring(count))
            end
        end)
    else
        state.set("timer_last_message", "every_missing")
        game.log("[TIMER] game.every não disponível nesta build")
    end
end

function on_update(dt)
    local count = state.get("timer_every_count", 0)
    local after_done = state.get("timer_after_done", false)
    local last_message = state.get("timer_last_message", "none")
    entity.set_text("[TIMER] count=" .. tostring(count) .. " | after=" .. tostring(after_done) .. " | last=" .. tostring(last_message))
end
