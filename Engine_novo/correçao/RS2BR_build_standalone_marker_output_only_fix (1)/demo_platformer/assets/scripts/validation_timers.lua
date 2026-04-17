local every_count = 0
local after_done = false
local last_message = "armed"

function on_start()
    if type(game.after) == "function" then
        game.after(1.0, function()
            after_done = true
            last_message = "after_1s"
            game.log("[TIMER] after(1.0) executou")
        end)
    else
        last_message = "after_missing"
        game.log("[TIMER] game.after não disponível nesta build")
    end

    if type(game.every) == "function" then
        game.every(0.75, function()
            every_count = every_count + 1
            last_message = "every_tick_" .. tostring(every_count)
            game.log("[TIMER] every(0.75) tick=" .. tostring(every_count))
        end)
    else
        last_message = "every_missing"
        game.log("[TIMER] game.every não disponível nesta build")
    end
end

function on_update(dt)
    entity.set_text(
        "[TIMER] local_every=" .. tostring(every_count) ..
        " | after=" .. tostring(after_done) ..
        " | last=" .. tostring(last_message)
    )
end
