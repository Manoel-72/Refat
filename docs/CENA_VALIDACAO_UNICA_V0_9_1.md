# Cena técnica única de validação — V0.9.1

Arquivo principal:
- `demo_platformer/assets/scenes/v0_9_1_regression.scene.json`

Scripts adicionados:
- `demo_platformer/assets/scripts/validation_time_api.lua`
- `demo_platformer/assets/scripts/validation_events.lua`
- `demo_platformer/assets/scripts/validation_timers.lua`
- `demo_platformer/assets/scripts/validation_collision_id.lua`

## O que a cena valida
- API oficial de tempo: `game.delta_time()` e `game.elapsed_time()`
- aliases de compatibilidade de tempo
- colisão por ID com 2 colliders de mesmo nome e IDs diferentes
- eventos leves via `event.emit(...)` e `event.listen(...)`
- timers via `game.after(...)` e `game.every(...)`

## Como testar
1. Abra a cena no editor.
2. Pressione Play.
3. Veja os logs no console.
4. Mova o Player até tocar os dois blocos amarelos.
5. Confirme:
   - logs `[TIME]`
   - logs `[COLLISION-ID] enter_id/exit_id`
   - logs `[EVENT]`
   - logs `[TIMER]`

## Observações
- Mantive o projeto incremental, sem trocar a cena inicial padrão.
- A cena é de regressão manual, pequena e local.
- Se a build ainda não tiver `event.*` ou timers ativos, os scripts logam essa ausência em vez de inventar workaround.
