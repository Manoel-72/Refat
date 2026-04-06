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
- `input.mouse_pos()`, `input.mouse_left`, `input.mouse_right`
- `game.delta_time`, `game.elapsed_time`
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
