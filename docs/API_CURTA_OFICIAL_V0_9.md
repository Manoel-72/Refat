# API curta oficial — RS2BR-Engine V0.9

Este documento resume o uso oficial da API de gameplay da engine.

## Regra principal de estado

- `save.*` = persistente entre sessões
- `session.*` = temporário da partida atual
- `state.*` = memória temporária por entidade + script

### Use `save.*` para
- checkpoint
- inventário persistente
- unlocks
- configuração
- melhor score global

### Use `session.*` para
- score da rodada atual
- moedas coletadas nesta execução
- timer da fase
- flags temporárias da partida

### Use `state.*` para
- patrulha de inimigo
- cooldown
- debounce
- timer interno
- memória entre frames

## API principal

### game
- `game.delta_time()` (oficial) → delta time do frame
- `game.elapsed_time()` (oficial) → tempo acumulado da cena
- `game.get_position()` → posição atual da entidade/script atual, quando suportado pelo runtime
- `game.set_position(x, y)` → move a entidade atual, quando suportado pelo runtime
- `game.get_velocity()` → velocidade atual da entidade/script atual, quando suportado pelo runtime
- `game.set_velocity(vx, vy)` → define velocidade da entidade atual
- `game.get_collisions()` → contatos atuais por nome
- `game.collision_enter(name)` → true no frame de entrada
- `game.collision_stay(name)` → true enquanto o contato permanece
- `game.collision_exit(name)` → true no frame de saída
- `game.raycast(ox, oy, dx, dy, dist)` → raycast simples
- `game.change_scene(path)` → troca de cena
- `game.log(msg)` / `game.warn(msg)` / `game.error(msg)` → logs Lua contextualizados

### input
- `input.key_held(name)`
- `input.key_pressed(name)`
- `input.mouse_pos() / input.get_mouse_pos()`
- `input.mouse_left`
- `input.mouse_right`

### entity
- `entity.get_name()`
- `entity.get_id()`
- `entity.set_visible(bool)`
- `entity.is_visible()`
- `entity.set_collision_enabled(bool)`
- `entity.is_collision_enabled()`
- `entity.destroy()` → remoção segura no fim do frame
- `entity.set_text(text)` → quando a entidade possuir componente de texto

### anim
- `entity.play_anim(clip)` → troca o clip atual do animator

## Estado persistente e temporário

### save
- `save.set(key, value)`
- `save.get(key)`
- `save.has(key)`
- `save.remove(key)`

### session
- `session.set(key, value)`
- `session.get(key, default)`
- `session.has(key)`
- `session.remove(key)`
- `session.clear()`

### state
- `state.set(key, value)`
- `state.get(key, default)`
- `state.has(key)`
- `state.remove(key)`
- `state.clear()`

## Exemplo curto

```lua
function on_start()
    state.set("cooldown", 0.0)
    if not session.has("score") then
        session.set("score", 0)
    end
end

function on_update(dt)
    local cd = state.get("cooldown", 0.0)
    cd = math.max(0.0, cd - game.delta_time())
    state.set("cooldown", cd)

    if game.collision_enter("Coin") and cd <= 0.0 then
        session.set("score", session.get("score", 0) + 1)
        state.set("cooldown", 0.2)
    end
end
```

## Cena oficial de regressão

Use `demo_platformer/assets/scenes/validation.scene.json` para validar:
- score em `session.*`
- melhor score em `save.*`
- patrulha com `state.*`
- colisão `enter/stay/exit`
- `entity.destroy()`

## Referências
- `demo_platformer/LUA_API.md`
- `docs/ARQUITETURA_RUNTIME_LUA_RS2.md`
- `docs/CENA_VALIDACAO_V0_9.md`


## Bloco 2 — colisão por id
- `game.collision_enter_id(id)` / `game.collision_stay_id(id)` / `game.collision_exit_id(id)`
- `game.get_current_collision_ids()` / `game.get_previous_collision_ids()`
- `game.get_collision_ids()` / `game.get_collision_names()`
- `game.get_current_collision_info()` retorna lista de `{ id, name }`
- `game.raycast(...)` agora também retorna `id` quando houver hit
