# Refatoração do ECS atual

O que foi melhorado sem trocar a base inteira do projeto:

- `ComponentKind` para identificar tipos de componente sem depender de índice mágico.
- `Entity::add_component` agora substitui componentes únicos em vez de duplicar.
- `Transform` passa a ser obrigatório em toda entidade.
- `remove_component` não remove `Transform`.
- Helpers tipados adicionados:
  - `transform()` / `transform_mut()`
  - `sprite()` / `sprite_mut()`
  - `camera()` / `camera_mut()`
  - `rigidbody()` / `rigidbody_mut()`
  - `collider()` / `collider_mut()`
  - `script()` / `script_mut()`
- `Scene::ensure_main_camera()` garante que exista uma câmera principal válida.
- `Scene::main_camera()` e `main_camera_mut()` agora usam busca recursiva segura.
- `AssetManager` ficou mais seguro contra operações fora da pasta `assets`.

Observação:
essa refatoração melhora o ECS atual sem migrar para `TypeId + Any`, o que evita quebrar editor, runtime e serialização de uma vez.
