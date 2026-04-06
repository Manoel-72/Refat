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
| `entity.set_visible(bool)` | função | Mostra/oculta a entidade |
| `entity.play_anim(clip)` | função | Troca clip do Animator (ex: `"run"`, `"idle"`) |

---

## `input` — leitura de controles

| Função | Retorno | Descrição |
|---|---|---|
| `input.key_held(name)` | bool | Tecla mantida pressionada neste frame |
| `input.key_pressed(name)` | bool | Tecla pressionada **neste frame apenas** |
| `input.mouse_pos()` | `{x, y}` | Posição do mouse na janela |
| `input.mouse_left` | bool | Botão esquerdo do mouse pressionado |
| `input.mouse_right` | bool | Botão direito do mouse |

**Nomes de teclas válidos:** `"A"` `"B"` … `"Z"`, `"Up"` `"Down"` `"Left"` `"Right"`,
`"Space"` `"Enter"` `"Escape"` `"W"` `"S"` `"D"`

---

## `game` — controle geral do jogo

| Campo / Função | Tipo | Descrição |
|---|---|---|
| `game.delta_time` | number | Segundos desde o último frame |
| `game.elapsed_time` | number | Segundos desde o início da cena |
| `game.log(msg)` | função | Imprime no terminal do engine |
| `game.change_scene(path)` | função | Carrega outra cena (ex: `"assets/scenes/fase2.scene.json"`) |
| `game.get_collisions()` | função → lista | Nomes das entidades colidindo com esta neste frame |
| `game.collision_enter(name)` | função → bool | True se `name` está colidindo este frame |
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
| `save.set(key, value)` | — | Salva valor (number, bool, string) |
| `save.get(key)` | value ou nil | Lê valor salvo |
| `save.has(key)` | bool | Verifica se a chave existe |
| `save.remove(key)` | — | Remove chave |

---

## Estrutura mínima de script

```lua
function on_start()
    -- Executado uma vez ao iniciar a cena
end

function on_update(dt)
    -- Executado todo frame. dt = game.delta_time em segundos.
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
function on_update(dt)
    local cols = game.get_collisions()
    for _, name in ipairs(cols) do
        if name == "Player" then
            entity.set_visible(false)
            local sc = save.get("score") or 0
            save.set("score", sc + 1)
        end
    end
end
```

---

*Versão da API: RS2BR-Engine V0.9 — sujeita a extensão sem quebra de retrocompatibilidade.*
