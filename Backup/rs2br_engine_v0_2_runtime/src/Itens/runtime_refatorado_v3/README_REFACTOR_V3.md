# Refatoração V3 — Runtime modular + fim do acesso frágil ao Transform

## O que foi feito
- runtime dividido em módulos reais:
  - `runtime/state.rs`
  - `runtime/input.rs`
  - `runtime/camera.rs`
  - `runtime/systems.rs`
  - `runtime/renderer.rs`
  - `runtime/script.rs`
- `runtime/mod.rs` virou fachada
- removido o acesso frágil por índice (`components.get_mut(0)`) dentro do runtime
- runtime agora usa `entity.transform()` e `entity.transform_mut()`
- lógica de scripts foi isolada
- lógica de câmera foi isolada
- lógica de update/simulação foi isolada
- lógica de desenho foi isolada

## Benefício real
Antes, o runtime estava centralizado em um arquivo gigante.
Agora, cada parte tem uma responsabilidade clara.

## O que você precisa substituir no projeto
Copie por cima estes arquivos/pastas:
- `runtime/`
- `engine/entity.rs` (somente se você tiver mudado e quiser manter igual ao pacote)

## Próximo passo recomendado
1. compilar
2. me mandar os erros reais de compilação se aparecerem
3. no próximo passo, eu posso limpar o `editor/mod.rs` e terminar a separação editor/runtime
