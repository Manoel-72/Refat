# RS2BR-Engine V0.9.3

## Resumo executivo
Esta atualização adiciona a base de produção pedida para a V0.9.3 sem reescrever a engine:
- spawn por Lua de entidade simples e prefab
- tags em entidade + busca por tag no Lua
- UI/HUD expandida com `set_color` e suporte `Sprite.screen_space`
- partículas leves no runtime

## Arquivos alterados
- `Cargo.toml`
- `src/core/component.rs`
- `src/core/entity.rs`
- `src/core/version.rs`
- `src/runtime/lua_runtime.rs`
- `src/runtime/mod.rs`
- `src/runtime/renderer.rs`
- `src/runtime/state.rs`
- `src/runtime/systems.rs`
- `src/runtime/systems/script_system.rs`
- `demo_platformer/LUA_API.md`
- `docs/API_CURTA_OFICIAL_V0_9.md`
- `docs/V0_9_3_CAMADA_PRODUCAO.md`
- `demo_platformer/assets/prefabs/enemy_basic.prefab.json`
- `demo_platformer/assets/scripts/v0_9_3_production_test.lua`

## Como testar rápido
- `game.spawn_entity("pickup", x, y)`
- `game.spawn_prefab("enemy_basic", x, y)`
- `entity.add_tag("enemy")`
- `game.find_by_tag("enemy")`
- `entity.set_color(1.0, 0.2, 0.2, 1.0)`
- `game.spawn_particle(x, y, 0, 30, 0.8, 1.0, 0.8, 0.2, 1.0)`

## Limitações conhecidas
- UI ainda é simples; não há layout completo.
- `find_by_tag` retorna snapshot com `id` e `name`.
- não foi criado sistema de referência viva entre entidades no Lua.
