# RS2BR-Engine V0.9.3 — Camada de Produção

## O que entrou
- `game.spawn_entity(name, x, y)`
- `game.spawn_prefab(path, x, y)`
- `entity.add_tag(tag)` / `entity.has_tag(tag)`
- `game.find_by_tag(tag)` / `game.find_one_by_tag(tag)`
- `entity.set_color(r, g, b, a)`
- suporte a `Sprite.screen_space` para UI Image/Panel simples
- `game.spawn_particle(x, y, vx, vy, life, r, g, b, scale)`

## Como testar
1. Adicione um LuaScript com `demo_platformer/assets/scripts/v0_9_3_production_test.lua`.
2. Pressione `P` no runtime para spawnar prefab + partícula.
3. Pressione `O` para spawnar entidade simples.
4. Use tags em entidades e consulte por Lua com `game.find_by_tag("enemy")`.

## Limitações conhecidas
- UI ainda é propositalmente simples e baseada nos componentes já existentes.
- `find_by_tag` retorna snapshot do frame atual (id e name), não referência viva.
- partículas são leves e intencionais; não substituem um sistema VFX completo.
