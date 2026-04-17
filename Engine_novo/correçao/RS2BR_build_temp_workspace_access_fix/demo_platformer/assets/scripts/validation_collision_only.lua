local ID_A = "ent-reg-trigger-a"
local ID_B = "ent-reg-trigger-b"

local enter_total = 0
local stay_total = 0
local exit_total = 0
local last_transition = "none"

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

local function ids_text(ids)
    if ids == nil or #ids == 0 then
        return "nenhum"
    end
    local parts = {}
    for _, id in ipairs(ids) do
        table.insert(parts, tostring(id))
    end
    return table.concat(parts, ",")
end

function on_update(dt)
    if type(game.get_current_collision_ids) ~= "function" then
        entity.set_text("[COLLISION] API ausente")
        return
    end

    local ids = game.get_current_collision_ids()
    local a_now = has_id(ids, ID_A)
    local b_now = has_id(ids, ID_B)

    local a_enter = type(game.collision_enter_id) == "function" and game.collision_enter_id(ID_A)
    local a_stay = type(game.collision_stay_id) == "function" and game.collision_stay_id(ID_A)
    local a_exit = type(game.collision_exit_id) == "function" and game.collision_exit_id(ID_A)

    local b_enter = type(game.collision_enter_id) == "function" and game.collision_enter_id(ID_B)
    local b_stay = type(game.collision_stay_id) == "function" and game.collision_stay_id(ID_B)
    local b_exit = type(game.collision_exit_id) == "function" and game.collision_exit_id(ID_B)

    if a_enter then enter_total = enter_total + 1; last_transition = "enter:A" end
    if b_enter then enter_total = enter_total + 1; last_transition = "enter:B" end
    if a_stay or b_stay then
        stay_total = stay_total + 1
        if a_stay then
            last_transition = "stay:A"
        elseif b_stay then
            last_transition = "stay:B"
        end
    end
    if a_exit then exit_total = exit_total + 1; last_transition = "exit:A" end
    if b_exit then exit_total = exit_total + 1; last_transition = "exit:B" end

    entity.set_text(
        "[COLLISION] ids=" .. ids_text(ids) ..
        " | A=" .. tostring(a_now) .. " B=" .. tostring(b_now) ..
        "\n[COLLISION] enter=" .. tostring(enter_total) ..
        " | stay=" .. tostring(stay_total) ..
        " | exit=" .. tostring(exit_total) ..
        " | last=" .. tostring(last_transition)
    )
end
