# Governanca do Repositorio

Este documento define regras simples para manter o repositorio previsivel, reduzir retrabalho e facilitar release.

## 1) Fonte unica de verdade

Codigo oficial (vivo):

- `src/`
- `assets/`
- `docs/`
- `Cargo.toml`
- `build.rs`
- `README.md`

Somente esses caminhos devem receber feature nova, correcao e refatoracao.

## 2) Arquivo historico

Snapshots, backups e copias paralelas (ex.: `Backup/`, `Engine_novo/`, `vald/`, arquivos `.zip`) sao historico.

Regras:

- Nao usar como base de desenvolvimento.
- Nao abrir PR a partir desses snapshots.
- Manter fora do fluxo diario (idealmente em `_archive/`).

## 3) Fluxo de trabalho (solo dev)

- Branch principal: `main`.
- Branch de trabalho: `feature/<nome-curto>`, `fix/<nome-curto>`, `refactor/<nome-curto>`.
- Merge para `main` somente apos build e testes locais.

## 4) Regra de qualidade minima

Antes de finalizar uma mudanca:

1. `cargo build`
2. `cargo test`
3. Atualizar documentacao quando houver mudanca de fluxo/comportamento

### CI (GitHub Actions)

No GitHub, o workflow `.github/workflows/ci.yml` roda em **push** e **pull request** para `main` ou `master`: `cargo build` e `cargo test` em **Ubuntu**, com pacotes de sistema para audio (`libasound2`) e dependencias comuns de linking para OpenGL/X11.

Voce tambem pode disparar manualmente em **Actions** → workflow **CI** → **Run workflow** (se `workflow_dispatch` estiver habilitado no arquivo).

## 5) Definicao de pronto (DoD)

Uma mudanca so e considerada pronta quando:

- Compila sem erro.
- Nao quebra testes existentes.
- Tem escopo claro e isolado.
- Nao depende de codigo historico/snapshot.

## 6) Objetivo

Manter o projeto com baixa ambiguidade operacional: todo mundo sabe onde mexer, como validar e o que pode (ou nao) entrar em release.
