# Arquitetura do runtime, Lua e RS2

## Fluxo de frame
1. Captura input do frame.
2. Atualiza scripts RS2 e Animator.
3. Aplica movimento e gravidade.
4. Executa Lua da entidade.
5. Resolve colisão e triggers.
6. Atualiza câmera e finaliza o frame.

## RuntimeContext
`RuntimeContext` é o contrato entre o runtime e o host. Isso permite rodar dentro do editor hoje e preparar um host standalone depois, sem acoplar a lógica do jogo ao editor.

O host fornece:
- estado de play/pause/edit
- projeto raiz
- snapshot da cena ativa
- lista de cenas
- status para UI
- cache de texturas para render inline

## Quando usar RS2
RS2 é o caminho mais simples para comportamento declarativo e eventos rápidos. Ele é ideal para:
- trocar cena
- mensagens de início
- movimentos simples
- gatilhos rápidos

## Quando usar Lua
Lua é o caminho para lógica mais dinâmica. Ele é melhor para:
- IA
- combate
- timers
- leitura/escrita de save
- lógica reativa por input ou colisão

## Como Lua e RS2 coexistem
Os dois podem coexistir na mesma engine, mas com papéis diferentes:
- RS2: camada simples e orientada a eventos
- Lua: camada de lógica programável

A recomendação atual é evitar duplicar responsabilidade na mesma entidade. Para comportamento simples, prefira RS2. Para lógica mais rica, prefira Lua.

## API Lua disponível (V0.9)
- `entity.x`, `entity.y`, `entity.vx`, `entity.vy`, `entity.name`, `entity.id`
- `entity.grounded`, `entity.visible`, `entity.rotation`, `entity.scale_x`, `entity.scale_y`
- `entity.set_position(x, y)`
- `entity.set_velocity(vx, vy)`
- `entity.apply_impulse(ix, iy)` — soma impulso à velocidade instantaneamente
- `entity.set_rotation(r)`
- `entity.set_visible(bool)`
- `entity.play_anim("clip")`
- `input.key_held("A")`, `input.key_pressed("Space")`
- `input.mouse_pos() / input.get_mouse_pos()`, `input.mouse_left`, `input.mouse_right`
- `game.delta_time()` (oficial), `game.elapsed_time()` (oficial)
- `game.log("msg")`
- `game.change_scene("path")`
- `game.get_collisions()` — lista de nomes das entidades em contato
- `game.collision_enter(name)` — bool, true se name colide neste frame
- `game.raycast(ox, oy, dx, dy, dist)` → `{hit, x, y, dist, name}`
- `save.set`, `save.get`, `save.has`, `save.remove`

## Física (V0.9)
- Gravidade: 540 px/s² por padrão (gravity_scale=1)
- Terminal velocity: 800 px/s máx de queda
- Pulo: use `entity.apply_impulse(0, 380)` com `entity.grounded` como guarda
- Colisão: AABB + MTV, resolução por eixo de menor sobreposição
- Layers/masks: bit flags 0–7, 0 = interage com tudo

## Demo Platformer
O projeto `demo_platformer/` contém uma cena testável imediatamente:
- Player azul controlável (WASD + Espaço)
- Chão + 3 plataformas (retângulos coloridos, sem sprites externos)
- 2 moedas coletáveis (trigger amarelo com efeito bob)
- 1 inimigo patrulhando (vermelho com reversão por raycast)
- Score persistido via `save`

Abra `demo_platformer/project.json` como projeto no engine.

## Observações de performance
A VM Lua agora é cacheada por `entity_id + script_path`, e só é recriada quando o arquivo muda. Isso evita criar uma VM nova a cada frame.


## Fluxo oficial: Novo jogo vs Continuar

A runtime agora diferencia explicitamente dois fluxos:

- **Novo jogo**
  - limpa `session_state`
  - limpa `script_state`
  - reinicia a cena do zero
  - preserva `save_data` persistente por padrão

- **Continuar**
  - carrega `save/save.json`
  - mantém apenas `save_data`
  - limpa `session_state`
  - limpa `script_state`
  - reabre a cena salva em `save_data.current_scene` quando possível

### API/fluxo de runtime

- `start_from_scene_as_new_game(scene)`
- `start_from_path_as_new_game(path)`
- `load_scene_as_new_game(path)`
- `continue_from_save(project_root, fallback_scene)`

### Regra oficial

- `save.*` = persistente
- `session.*` = temporário da partida atual
- `state.*` = memória temporária por entidade + script

### Migração oficial dos scripts demo

Os scripts oficiais da demo foram alinhados com a regra nova:

- `player_platformer.lua` inicializa score da rodada em `session.*`
- `score_hud.lua` lê score da sessão atual
- `coin_collector.lua` usa `session.*` para score e `state.*` para evitar coleta duplicada
- `enemy_patrol.lua` usa `state.*` para origem/direção da patrulha
- `pickup_test.lua` usa `session.*` para pickups da rodada
- `collision_probe.lua` usa `state.*` para memória local de contato

Scripts de gameplay **não devem** depender de limpeza manual para iniciar um novo jogo. O reset oficial da sessão deve acontecer no runtime.


## Colisão por entidade
- `BoxCollider.collision_enabled = true/false` controla participação em colisão de gameplay.
- Isso é independente de `entity.visible`: a entidade pode ficar invisível e continuar colidindo, ou visível com colisão desativada.
- Em Lua: `entity.collision_enabled` e `entity.set_collision_enabled(bool)`.


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


## Cena oficial de validação (Bloco L)
- Cena: `demo_platformer/assets/scenes/validation.scene.json`
- Scripts principais:
  - `player_platformer.lua` → sessão atual
  - `coin_collector.lua` → score por `session.*`
  - `enemy_patrol.lua` → patrulha por `state.*`
  - `collision_probe.lua` → `collision_enter/stay/exit`
  - `pickup_test.lua` → `entity.destroy()`
  - `save_validation_hud.lua` → separação entre sessão atual e melhor score persistível em `save.*`
- Guia rápido: `docs/CENA_VALIDACAO_V0_9.md`


## Documentação oficial curta (Bloco M)
- Documento principal: `docs/API_CURTA_OFICIAL_V0_9.md`
- Objetivo: registrar de forma curta e oficial a API de gameplay e a regra de estado da engine.
- Regra oficial: `save.*` persistente, `session.*` sessão atual, `state.*` memória por entidade/script.
- Referências práticas: `demo_platformer/LUA_API.md` e `docs/CENA_VALIDACAO_V0_9.md`.


## Bloco 2 — colisão por id
- `game.collision_enter_id(id)` / `game.collision_stay_id(id)` / `game.collision_exit_id(id)`
- `game.get_current_collision_ids()` / `game.get_previous_collision_ids()`
- `game.get_collision_ids()` / `game.get_collision_names()`
- `game.get_current_collision_info()` retorna lista de `{ id, name }`
- `game.raycast(...)` agora também retorna `id` quando houver hit
