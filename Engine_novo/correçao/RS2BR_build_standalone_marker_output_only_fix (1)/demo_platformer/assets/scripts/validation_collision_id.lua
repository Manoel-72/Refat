local enter_count = 0
local stay_count = 0
local exit_count = 0
local last_event = "none"
local last_ids = {}

local function has_id(list, wanted)
    if list == nil then
        return false
    end
    for _, value in ipairs(list) do
        if tostring(value) == tostring(wanted) then
            return true
        end
    end
    return false
end

local function append_ids(ids)
    local parts = {}
    if ids ~= nil then
        for _, id in ipairs(ids) do
            table.insert(parts, tostring(id))
        end
    end
    if #parts == 0 then
        return "nenhuma"
    end
    return table.concat(parts, ",")
end

local function emit_collision(tag)
    if event ~= nil and type(event.emit) == "function" then
        event.emit("validation_collision", tag)
    end
end

function on_start()
    game.log("[COLLISION-ID] pronto: mova o player até os 2 triggers amarelos")
end

function on_update(dt)
    if type(game.get_current_collision_ids) ~= "function" then
        entity.set_text("[COLLISION] API ausente")
        return
    end

    local ids = game.get_current_collision_ids()

    local probe_ids = { "ent-reg-trigger-a", "ent-reg-trigger-b" }
    for _, id in ipairs(probe_ids) do
        local entered = type(game.collision_enter_id) == "function" and game.collision_enter_id(id)
        local staying = type(game.collision_stay_id) == "function" and game.collision_stay_id(id)
        local exited = type(game.collision_exit_id) == "function" and game.collision_exit_id(id)

        if entered then
            enter_count = enter_count + 1
            last_event = "enter:" .. tostring(id)
            emit_collision("enter:" .. tostring(id))
            game.log("[COLLISION-ID] enter_id=" .. tostring(id))
        end

        if staying then
            stay_count = stay_count + 1
            last_event = "stay:" .. tostring(id)
        end

        if exited then
            exit_count = exit_count + 1
            last_event = "exit:" .. tostring(id)
            emit_collision("exit:" .. tostring(id))
            game.log("[COLLISION-ID] exit_id=" .. tostring(id))
        end
    end

    -- fallback observável: se a API de enter/exit falhar, ainda vemos os ids atuais
    local a_now = has_id(ids, "ent-reg-trigger-a")
    local b_now = has_id(ids, "ent-reg-trigger-b")

    entity.set_text(
        "[COLLISION] atuais=" .. append_ids(ids) ..
        "\n[COLLISION] A=" .. tostring(a_now) .. " | B=" .. tostring(b_now) ..
        " | enter=" .. tostring(enter_count) ..
        " | stay=" .. tostring(stay_count) ..
        " | exit=" .. tostring(exit_count) ..
        " | last=" .. tostring(last_event)
    )
end
