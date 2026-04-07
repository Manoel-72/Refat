# Hotfix V0.9.4 — input gamepad + mouse world

Alterações aplicadas:
- `RuntimeInput` agora expõe `gamepad_axes: HashMap<u32, f32>`.
- `input.gamepad_axis(n)` exposto para Lua.
- `input.mouse_world_pos` e `input.get_mouse_world_pos()` expostos para Lua.
- `systems.rs` agora captura snapshot da câmera principal e repassa para `script_system`/`lua_runtime`.
- Backend atual continua sem integração real com controle; por isso `gamepad_buttons` e `gamepad_axes` seguem vazios por padrão no host egui.

Observação:
- A conversão usada para mundo segue a convenção atual do renderer: `world_x = mx / zoom + cam_x` e `world_y = cam_y - my / zoom`.
