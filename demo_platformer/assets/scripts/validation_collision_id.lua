local known_ids = {}
local seen_enter = {}
local seen_exit = {}

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

function on_start()
    game.log("[COLLISION-ID] pronto: mova o player até os 2 triggers amarelos")
end

function on_update(dt)
    if type(game.get_current_collision_ids) ~= "function" then
        return
    end

    local ids = game.get_current_collision_ids()
    entity.set_text("[COLLISION-ID] atuais=" .. append_ids(ids))

    if ids ~= nil then
        for _, id in ipairs(ids) do
            if not known_ids[id] then
                known_ids[id] = true
                game.log("[COLLISION-ID] contato atual com id=" .. tostring(id))
            end
        end
    end

    local probe_ids = { "ent-reg-trigger-a", "ent-reg-trigger-b" }
    for _, id in ipairs(probe_ids) do
        if type(game.collision_enter_id) == "function" and game.collision_enter_id(id) and not seen_enter[id] then
            seen_enter[id] = true
            game.log("[COLLISION-ID] enter_id=" .. tostring(id))
        end

        if type(game.collision_stay_id) == "function" and game.collision_stay_id(id) then
            -- sem log contínuo para não poluir demais
        end

        if type(game.collision_exit_id) == "function" and game.collision_exit_id(id) and not seen_exit[id] then
            seen_exit[id] = true
            game.log("[COLLISION-ID] exit_id=" .. tostring(id))
        end
    end
end
