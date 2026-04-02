# RS2BR-Engine V0.6

## O que entrou
- versão do projeto atualizada para 0.6.0
- asset records oficiais com:
  - id
  - tipo
  - path
  - status de carregamento
  - validação
- `AssetManager::list_asset_records()`
- criação de cena pelo Asset Browser
- duplicação de assets e pastas
- asset details com tipo/status/validação
- serializers com `Result` mais claro para cenas e prefabs
- testes básicos de roundtrip para scene/prefab
- criação automática das pastas `assets/sounds`, `assets/fonts`, `assets/prefabs`
- menubar atualizada para v0.6.0

## Fluxo de produção melhorado
- criar cena pelo editor
- criar script RS2 pelo editor
- criar pasta pelo editor
- importar sprite
- exportar entidade selecionada como prefab
- duplicar asset
- renomear asset
- deletar asset

## Melhorias de robustez
- validação de asset quebrado ou tipo inválido
- status `Ready`, `Missing`, `Invalid`
- `try_load_scene_from_path` e `try_load_prefab_from_path`
