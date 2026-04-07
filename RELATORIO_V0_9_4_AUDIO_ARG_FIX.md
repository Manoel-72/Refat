# Hotfix — argumento `audio_runtime` na recursão

Corrigido erro de compilação em `src/runtime/systems.rs`:

- chamada recursiva de `update_entities_runtime_recursive(...)` estava com 27 argumentos
- a função agora exige 28 argumentos
- foi adicionado o parâmetro faltante:
  - `audio_runtime`

Erro corrigido:
- `error[E0061]: this function takes 28 arguments but 27 arguments were supplied`
