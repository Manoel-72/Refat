# Otimização leve do runtime — RS2BR-Engine

## Resumo executivo
- HUD/debug: textos de debug passaram a usar snapshot/cache com atualização desacelerada.
- Logs: prints repetitivos de RS2/Lua agora são limitados por intervalo curto para evitar flood por frame.
- Colisão: foi adicionada uma checagem barata de separação antes do AABB/MTV completo.
- Medição: foram adicionados campos leves de métricas no runtime e exibidos no overlay.
- Preservado: formato de cena, API pública Lua, save/load, fluxo principal do runtime e comportamento geral de gameplay.

## Arquivos alterados
- src/runtime/mod.rs
- src/runtime/state.rs
- src/runtime/systems.rs
- src/runtime/systems/collision_system.rs
- src/runtime/systems/script_system.rs
- src/runtime/lua_runtime.rs

## Otimizações concluídas
- Cache visual para HUD/debug com atualização em baixa frequência.
- Overlay de performance simples no runtime.
- Redução de spam de log em RS2/Lua.
- Early-out simples na colisão.

## Otimizações parciais
- Métrica de colisão está agregada dentro da janela principal de atualização de scripts/runtime, não separada por subsistema interno.
- A ponte Lua continua compatível; foram priorizados logs e custo visual antes de mudanças mais profundas de alocação.

## Riscos
- Logs repetidos agora podem aparecer com pequeno atraso por causa do throttle.
- Métricas são aproximadas e voltadas para diagnóstico leve, não profiling profundo.

## Próximo passo recomendado
- Separar medição de Lua e colisão em pontos ainda mais específicos quando o ambiente local tiver toolchain Rust disponível para validar benchmark fino.
