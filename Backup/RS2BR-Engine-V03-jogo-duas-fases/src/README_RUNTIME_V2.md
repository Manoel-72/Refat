# Refatoração V2 do runtime

## O que foi mudado

- `runtime/mod.rs` virou uma fachada pequena.
- Estado do runtime foi movido para `runtime/state.rs`.
- Input foi movido para `runtime/input.rs`.
- Câmera foi movida para `runtime/camera.rs`.
- Systems de atualização ficaram em `runtime/systems.rs`.
- Parser e carregamento de scripts ficaram em `runtime/script.rs`.
- Render do runtime ficou em `runtime/renderer.rs`.

## Objetivo

Parar de concentrar toda a lógica num único `runtime/mod.rs` gigante.

## Benefícios

- manutenção muito melhor
- responsabilidades mais claras
- base pronta para crescer
- editor e runtime mais fáceis de separar no futuro

## Observação

Essa refatoração foi feita para manter o comportamento o mais próximo possível do que você já tinha, mas com a arquitetura melhor.
Depois do teste, o próximo passo recomendado é:

1. tirar a dependência de `get_mut(0)` para `Transform`
2. criar systems mais explícitos (`physics_system`, `script_system`, `camera_follow_system`)
3. separar editor e runtime em crates diferentes
