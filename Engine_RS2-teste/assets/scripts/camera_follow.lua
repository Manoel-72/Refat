function on_update(dt)
    game.camera_zoom(1.0)
    local player_id = session.get("player_id", "")
    if player_id ~= "" then
        game.set_camera_target(player_id, 10.0)
    end
end
