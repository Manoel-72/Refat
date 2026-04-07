# RS2BR-Engine — runtime fixes (VM / flip_x / raycast / timers)

## O que foi corrigido
- `run_lua_script()` agora reutiliza VM por chave estável no caminho direto, em vez de criar `Lua::new()` toda chamada.
- `flip_x` agora só aplica quando o script realmente envia booleano explícito.
- `game.raycast(...)` deixou de usar stepping fixo e passou a usar interseção exata com AABB (slab test).
- timers repetitivos (`game.every`) agora recalculam `remaining` sem acumular deriva negativa em lag spikes.

## Arquivo alterado
- `src/runtime/lua_runtime.rs`

## Observações
- O runtime principal já tinha cache de VMs em `script_system.rs`; este patch fecha também o caminho direto de `run_lua_script()`.
- O raycast exato reduz tunelamento em colliders pequenos e melhora shooter/platformer.
- O fix de timer dispara no máximo uma vez por frame e preserva fase melhor após frame longo.

## Limitação honesta
- Não validei com `cargo check` neste ambiente porque o toolchain Rust não está disponível aqui.
