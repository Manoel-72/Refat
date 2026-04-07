# RS2BR-Engine — Referência Lua API (V0.9)

Todos os scripts `.lua` recebem acesso às tabelas globais abaixo.
Escreva funções `on_start()` e `on_update(dt)` — o engine chama automaticamente.

---

## `entity` — a entidade dona do script

| Campo / Função | Tipo | Descrição |
|---|---|---|
| `entity.x` | number (leitura) | Posição X no mundo |
| `entity.y` | number (leitura) | Posição Y no mundo |
| `entity.vx` | number (leitura) | Velocidade X atual |
| `entity.vy` | number (leitura) | Velocidade Y atual |
| `entity.rotation` | number (leitura) | Rotação em graus |
| `entity.scale_x` | number (leitura) | Escala X |
| `entity.scale_y` | number (leitura) | Escala Y |
| `entity.grounded` | bool (leitura) | True se tocando o chão |
| `entity.visible` | bool (leitura) | Visibilidade atual |
| `entity.name` | string (leitura) | Nome da entidade |
| `entity.id` | string (leitura) | UUID único da entidade |
| `entity.set_position(x, y)` | função | Move para coordenada absoluta |
| `entity.set_velocity(vx, vy)` | função | Define velocidade (px/s) |
| `entity.apply_impulse(ix, iy)` | função | Soma impulso instantâneo à velocidade |
| `entity.set_rotation(r)` | função | Define rotação em graus |
| `entity.get_name()` | função → string | Retorna o nome da entidade |
| `entity.get_id()` | função → string | Retorna o id único da entidade |
| `entity.set_visible(bool)` | função | Mostra/oculta a entidade |
| `entity.is_visible()` | função → bool | Consulta a visibilidade atual |
| `entity.set_collision_enabled(bool)` | função | Liga/desliga a colisão da entidade |
| `entity.is_collision_enabled()` | função → bool | Consulta se a colisão está ligada |
| `entity.play_anim(clip)` | função | Troca clip do Animator (ex: `"run"`, `"idle"`) |

---

## `input` — leitura de controles

| Função | Retorno | Descrição |
|---|---|---|
| `input.key_held(name)` | bool | Tecla mantida pressionada neste frame |
| `input.key_pressed(name)` | bool | Tecla pressionada **neste frame apenas** |
| `input.mouse_pos() / input.get_mouse_pos()` | `{x, y}` | Posição do mouse na janela |
| `input.mouse_left` | bool | Botão esquerdo do mouse pressionado |
| `input.mouse_right` | bool | Botão direito do mouse |

**Nomes de teclas válidos:** `"A"` `"B"` … `"Z"`, `"Up"` `"Down"` `"Left"` `"Right"`,
`"Space"` `"Enter"` `"Escape"` `"W"` `"S"` `"D"`

---

## `game` — controle geral do jogo

| Campo / Função | Tipo | Descrição |
|---|---|---|
| `game.delta_time()` (oficial) | função → number | Segundos desde o último frame |
| `game.elapsed_time()` (oficial) | função → number | Segundos desde o início da cena |
| `game.log(msg)` | função | Imprime no terminal do engine |
| `game.change_scene(path)` | função | Carrega outra cena (ex: `"assets/scenes/fase2.scene.json"`) |
| `game.get_collisions()` | função → lista | Nomes das entidades colidindo com esta neste frame |
| `game.get_current_collisions()` | função → lista | Snapshot atual dos nomes em contato (compatibilidade) |
| `game.get_previous_collisions()` | função → lista | Snapshot dos contatos do frame anterior |
| `game.collision_enter(name)` | função → bool | True somente no frame em que o contato começou |
| `game.collision_stay(name)` | função → bool | True enquanto o contato continua |
| `game.collision_exit(name)` | função → bool | True somente no frame em que o contato terminou |
| `game.raycast(ox,oy,dx,dy,dist)` | função → tabela | Lança raio; retorna `{hit, x, y, dist, name}` |

**Raycast exemplo:**
```lua
local hit = game.raycast(entity.x, entity.y, 1, 0, 100)
if hit.hit then
    game.log("Acertou: " .. hit.name .. " a " .. hit.dist .. "px")
end
```

---

## `save` — persistência entre sessões

| Função | Retorno | Descrição |
|---|---|---|
| `save.set(key, value)` | — | Salva valor persistente (number, bool, string) |
| `save.get(key)` | value ou nil | Lê valor persistente |
| `save.has(key)` | bool | Verifica se a chave existe |
| `save.remove(key)` | — | Remove chave |

Use `save.*` apenas para progresso real: checkpoint, inventário persistente, unlocks, config e score global.

## `session` — estado temporário da partida atual

| Função | Retorno | Descrição |
|---|---|---|
| `session.set(key, value)` | — | Salva valor temporário da rodada |
| `session.get(key, default)` | value | Lê valor temporário da rodada |
| `session.has(key)` | bool | Verifica se a chave existe |
| `session.remove(key)` | — | Remove chave |
| `session.clear()` | — | Limpa a sessão atual |

Use `session.*` para score da rodada, moedas coletadas na execução atual, timer da fase e flags temporárias.

## `state` — memória local por entidade/script

| Função | Retorno | Descrição |
|---|---|---|
| `state.set(key, value)` | — | Salva valor local da entidade/script |
| `state.get(key, default)` | value | Lê valor local |
| `state.has(key)` | bool | Verifica se a chave existe |
| `state.remove(key)` | — | Remove chave local |
| `state.clear()` | — | Limpa a memória local |

Use `state.*` para patrulha, cooldown, debounce, timers internos e memória entre frames.

---

## Estrutura mínima de script

```lua
function on_start()
    -- Executado uma vez ao iniciar a cena
end

function on_update(dt)
    -- Executado todo frame. dt = game.delta_time() em segundos.
end
```

## Padrão de movimento (platformer)

```lua
local SPEED = 150
local JUMP  = 350

function on_update(dt)
    local vx = 0
    if input.key_held("A") or input.key_held("Left")  then vx = -SPEED end
    if input.key_held("D") or input.key_held("Right") then vx =  SPEED end
    entity.set_velocity(vx, entity.vy)

    if input.key_pressed("Space") and entity.grounded then
        entity.apply_impulse(0, JUMP)
    end
end
```

## Padrão de coleta (trigger)

```lua
function on_start()
    state.set("collected", false)
end

function on_update(dt)
    if state.get("collected", false) then
        return
    end

    local cols = game.get_collisions()
    for _, name in ipairs(cols) do
        if name == "Player" then
            state.set("collected", true)
            entity.set_visible(false)

            local sc = session.get("score", 0)
            session.set("score", sc + 1)
        end
    end
end
```

---

*Versão da API: RS2BR-Engine V0.9 — sujeita a extensão sem quebra de retrocompatibilidade.*


### entity.collision_enabled / entity.set_collision_enabled(bool)
Controla explicitamente se o `BoxCollider` participa da colisão de gameplay. Não depende de `visible`.


### Eventos de colisão por frame

- `game.collision_enter(name)` → verdadeiro só no frame de entrada
- `game.collision_stay(name)` → verdadeiro enquanto o contato permanece
- `game.collision_exit(name)` → verdadeiro só no frame de saída

Helpers auxiliares de debug:
- `game.get_collisions()` → contatos atuais
- `game.get_current_collisions()` → contatos atuais
- `game.get_previous_collisions()` → contatos do frame anterior


## Logs Lua
- Erros de script agora saem padronizados com cena, entidade, id, script, stage e kind.
- `print(...)` em Lua recebe prefixo contextual automaticamente.
- Helpers extras de log disponíveis: `game.log(msg)`, `game.warn(msg)`, `game.error(msg)`.


## Bloco K — entity.destroy()

- `entity.destroy()` agenda a remoção da entidade para o fim do frame.
- O runtime evita remoção imediata no meio da iteração.
- Pedidos duplicados de destroy para a mesma entidade são ignorados com segurança.
- Quando a entidade é removida, o runtime limpa também estado temporário associado:
  - `script_state` da entidade
  - cache de VM Lua da entidade
  - contatos de colisão rastreados para a entidade

Use isso para pickups, projéteis e inimigos derrotados sem quebrar a atualização atual do frame.


## Documento curto oficial
Consulte também `docs/API_CURTA_OFICIAL_V0_9.md` para a versão resumida e oficial da API e da regra de estado (`save.*`, `session.*`, `state.*`).


## Bloco 2 — colisão por id
- `game.collision_enter_id(id)` / `game.collision_stay_id(id)` / `game.collision_exit_id(id)`
- `game.get_current_collision_ids()` / `game.get_previous_collision_ids()`
- `game.get_collision_ids()` / `game.get_collision_names()`
- `game.get_current_collision_info()` retorna lista de `{ id, name }`
- `game.raycast(...)` agora também retorna `id` quando houver hit


- `player_probe_state.lua` pode ser usado na cena de validação para provar isolamento de `state.*` entre scripts da mesma entidade.


## Adições V0.9.3
- `game.spawn_entity(name, x, y)` → retorna handle opcional (`request_id`, `set_velocity`, `set_position`, `add_tag`, `set_hp`, `set_anim`)
- `game.spawn_prefab(path, x, y)` → retorna handle opcional (`request_id`, `set_velocity`, `set_position`, `add_tag`, `set_hp`, `set_anim`)
- `game.find_by_tag(tag)`
- `game.find_one_by_tag(tag)`
- `game.spawn_particle(x, y, vx, vy, life, r, g, b, scale)`
- `game.spawn_emitter(x, y, rate, particle_life, speed_min, speed_max, r, g, b, scale, duration)`
- `entity.add_tag(tag)`
- `entity.has_tag(tag)`
- `entity.set_color(r, g, b, a)`


## Teclas adicionais suportadas

Além de WASD, setas, Space, Enter e Escape, o snapshot Lua agora expõe:

- `Q`, `E`, `F`
- `Shift`, `Ctrl`
- números `0` a `9`

Observação: gamepad ainda não é exposto ao Lua neste runtime embutido.


## Audio Lua

- `audio.play_sound(key, path)`
- `audio.play_ex(key, path, looped?, volume?)`
- `audio.stop_sound(key)`
- `audio.set_volume(key, volume)`
- aliases: `audio.play`, `audio.stop`

## Eventos de spawn

- `spawn_result:<seq>`
- `rs2_spawn_done:<seq>`
