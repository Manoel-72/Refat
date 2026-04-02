# Engine V4

## O que mudou

- `Entity::add_component` agora substitui componentes únicos automaticamente (`Sprite`, `Camera2D`, `RigidBody2D`, `BoxCollider`, `Transform`).
- `Transform` passa a ser protegido: a entidade sempre mantém um `Transform` válido.
- Novos helpers tipados:
  - `transform()` / `transform_mut()`
  - `sprite()` / `sprite_mut()`
  - `camera()` / `camera_mut()`
  - `rigidbody()` / `rigidbody_mut()`
  - `collider()` / `collider_mut()`
- Novo `ComponentKind` para remover a dependência de índice mágico.
- `Scene::ensure_main_camera()` garante que a cena sempre tenha uma câmera principal.
- `AssetManager` ganhou validação para impedir operações fora de `/assets`.

## O que ainda falta depois desta etapa

- atualizar `editor/` e `runtime/` para usar `entity.transform_mut()` em vez de `components.get_mut(0)`
- criar `systems.rs` de verdade para física, script e render
- separar `core` / `editor` / `runtime` em crates no futuro

## Como aplicar

Substitua a pasta `src/engine/` pela pasta `engine/` deste pacote.

## Observação honesta

Esta V4 foi feita para **encaixar no seu projeto atual sem destruir o restante**. Ela melhora muito a base, mas ainda não é o ECS genérico final com `TypeId + Any`, porque isso exigiria refatorar `editor`, `runtime`, serialização e inspector ao mesmo tempo.
