# RS2BR-Engine V0.7-A

Entrega incremental segura sobre a base V0.6.

## O que entrou
- Input centralizado com `pressed`, `held` e `released`
- Novo componente `Velocity`
- Movimento desacoplado usando `Transform + Velocity`
- Física simples com gravidade e `grounded`
- Colisão AABB mantendo rollback simples e seguro
- Script RS2 com suporte incremental a `@on_start` e `@on_update` em linha única ou em bloco
- Cena e script de teste em `assets/scenes` e `assets/scripts`

## Como rodar
1. Tenha Rust instalado.
2. No terminal, entre na pasta do projeto.
3. Execute `cargo run`.

## Como testar
### Input
- Entre em Play.
- Segure `W A S D` para mover o player.
- As teclas direcionais continuam controlando a câmera.
- O estado interno agora diferencia pressionado, segurado e solto por frame.

### Movimento
- O player usa `Velocity` quando disponível.
- O `player_controller` do script define a velocidade horizontal/vertical a partir do input.

### Colisão
- Abra `assets/scenes/v07_a_test.scene.json`.
- Rode a cena.
- O player não deve atravessar o chão.

### Física
- O player tem `RigidBody2D` com gravidade.
- Ao tocar o chão, a velocidade vertical é zerada e `grounded` fica verdadeiro.

### Script
Arquivo de exemplo: `assets/scripts/player_controller.rs2`

Exemplo suportado:

```rs2
@on_start
print("on_start do player executado")

@on_update
move_x(10)
move_y(0)
```

## Arquivos principais alterados
- `src/runtime/input.rs`
- `src/runtime/input/input_state.rs`
- `src/runtime/input/key_code.rs`
- `src/runtime/systems/input_system.rs`
- `src/runtime/systems/movement_system.rs`
- `src/runtime/systems/physics_system.rs`
- `src/runtime/systems/script_system.rs`
- `src/runtime/script.rs`
- `src/runtime/systems.rs`
- `src/runtime/state.rs`
- `src/core/component.rs`
- `src/core/entity.rs`
- `src/editor/inspector.rs`
- `src/editor/hierarchy.rs`
- `src/editor/scene_view.rs`
- `src/editor/warnings.rs`

## Observação
Eu não consegui compilar aqui porque o ambiente não tem `cargo` instalado. Então a entrega foi feita como alteração estrutural e de código, com foco em mudanças pequenas, locais e seguras.
