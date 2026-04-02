# Refatoração aplicada

## O que foi melhorado
- `editor/state.rs` agora é a fonte única de verdade para o estado do editor.
- `editor/actions.rs` passou a depender da API de `Scene` em vez de duplicar busca recursiva.
- `engine/entity.rs` ganhou utilitários de busca, visita e acesso ao `Transform`.
- `engine/scene.rs` virou o ponto central das operações de hierarquia.
- `main.rs` agora declara `renderer` explicitamente.
- `runtime/renderer.rs` e `runtime/script.rs` deixaram de ser arquivos vazios.

## Problemas graves corrigidos conceitualmente
- Duplicação de estado entre `editor/mod.rs` e `editor/state.rs`.
- Busca recursiva espalhada em múltiplos lugares.
- Dependência direta de detalhes internos da hierarquia em módulos de UI.

## O que ainda falta numa V2 profissional
- Separar editor e runtime em crates distintos.
- Tirar a lógica de simulação gigante de `runtime/mod.rs`.
- Criar scheduler/systems para ECS.
- Criar AssetId/metadata/cache de assets.
- Adicionar testes de serialização de cena e entidade.

## Observação honesta
Este pacote não veio com `Cargo.toml`, então a refatoração foi feita com foco em arquitetura e consistência de código, mas sem validação por compilação local.
