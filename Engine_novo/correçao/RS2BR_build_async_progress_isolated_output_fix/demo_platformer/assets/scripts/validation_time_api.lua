local shown_start = false
local shown_one_second = false
local last_label = nil

local function safe_call(fn)
    if type(fn) == "function" then
        local ok, value = pcall(fn)
        if ok then
            return value
        end
    end
    return nil
end

local function dt_now()
    local v = safe_call(game.delta_time)
    if type(v) == "number" then
        return v
    end
    v = safe_call(game.get_delta_time)
    if type(v) == "number" then
        return v
    end
    if type(game.delta_time_value) == "number" then
        return game.delta_time_value
    end
    return -1
end

local function elapsed_now()
    local v = safe_call(game.elapsed_time)
    if type(v) == "number" then
        return v
    end
    v = safe_call(game.get_elapsed_time)
    if type(v) == "number" then
        return v
    end
    if type(game.elapsed_time_value) == "number" then
        return game.elapsed_time_value
    end
    return -1
end

local function rebuild_text()
    local text = "[TIME] dt=" .. tostring(dt_now()) .. " | elapsed=" .. tostring(elapsed_now())
    return text
end

function on_start()
    local text = rebuild_text()
    entity.set_text(text)
    last_label = text

    if not shown_start then
        shown_start = true
        game.log("[TIME] validação iniciada")
        game.log("[TIME] delta_time() = " .. tostring(dt_now()))
        game.log("[TIME] elapsed_time() = " .. tostring(elapsed_now()))
    end
end

function on_update(dt)
    local text = rebuild_text()
    if text ~= last_label then
        last_label = text
        entity.set_text(text)
    end

    if not shown_one_second and elapsed_now() >= 1.0 then
        shown_one_second = true
        game.log("[TIME] após 1 segundo | dt=" .. tostring(dt_now()) .. " | elapsed=" .. tostring(elapsed_now()))
    end
end
