# RS2BR-Engine V0.9.4 — Lua ↔ Rust 100% funcional (patch)

## Aplicado neste patch

1. **VM Lua reutilizada por entidade+script**
   - Mantido cache em `RuntimeState.lua_vms`.
   - Continua reutilizando a VM entre frames e recarrega apenas quando o arquivo muda.

2. **`flip_x` lendo `LuaValue::Boolean`**
   - Coleta robusta em `lua_runtime.rs`.

3. **Raycast preciso sem step fixo**
   - `collision_system.rs` atualizado para slab test (raio vs AABB).
   - Evita tunelamento e retorna o menor hit paramétrico válido.

4. **Timers repetitivos com overshoot controlado**
   - `advance_timers` agora limita `remaining` ao próximo intervalo útil.
   - Dispara callback apenas uma vez por frame, mesmo com frame longo.

5. **Input Lua expandido**
   - Snapshot cobre `Shift`, `Ctrl`, `Q`, `E`, `F`, `0..9`.
   - Adicionada API `input.gamepad_button(n)`.
   - Observação: o backend atual ainda não alimenta botões reais de gamepad; a superfície da API já está pronta.

6. **Spawn devolvendo ID para Lua**
   - `PendingSpawnRequest` agora carrega `request_seq`.
   - Após criar a entidade, o runtime emite `spawn_result:<seq>` com o `entity.id` gerado.
   - Uso em Lua:
     - `event.listen("spawn_result:1", function(id) ... end)`

## Arquivos alterados

- `src/runtime/lua_runtime.rs`
- `src/runtime/state.rs`
- `src/runtime/systems/script_system.rs`
- `src/runtime/systems/collision_system.rs`
- `src/runtime/systems/input_system.rs`

