# Plano 0 a 30 dias (solo dev, baixa dor de suporte)

Objetivo deste mes: reduzir ambiguidade do repo, evitar regressao boba e ganhar performance visivel onde custa pouco.

## Ja feito (baseline)

- [x] Fonte oficial documentada (`README.md` + `docs/REPO_GOVERNANCA.md`)
- [x] `.gitignore` para zips e backups acidentais
- [x] CI no GitHub (`.github/workflows/ci.yml`) + **CI local** (`scripts/ci.ps1` / `scripts/ci.sh`)
- [x] Correcao dos testes `run_lua_script` em `lua_runtime.rs`

## Semana 1 (dias 1-7)

- [ ] Confirmar **Actions** no GitHub rodando verde (ou usar so `scripts/ci.ps1` antes de cada push)
- [ ] Se ainda existir pasta de snapshot na raiz: mover para `_archive/` (alinhado a governanca)
- [ ] Antes de cada merge na `main`: `scripts/ci.ps1` (Windows) ou `bash scripts/ci.sh`

## Semana 2 (dias 8-14)

- [x] **Colisao broad-phase:** deduplicacao de candidatos sem `Vec::contains` (`HashSet` reutilizavel via `thread_local` em `collision_system.rs`)
- [ ] Opcional: versionar `Cargo.lock` (tirar do `.gitignore`) para builds reproduziveis — decidir se o binario e produto distribuido

## Semana 3 (dias 15-21)

- [ ] Primeiro corte de **modularizacao** do runtime: extrair um modulo pequeno de `state.rs` ou `lua_runtime.rs` **sem mudar comportamento** (so mover funcoes + `mod`)
- [ ] Lista curta de **mensagens de erro** que usuarios repetem — melhorar texto ou log em 3-5 pontos

## Semana 4 (dias 22-30)

- [ ] **FAQ interno** (10 linhas): como rodar, como exportar, erros comuns (Rust nao encontrado, VC++ redist, etc.)
- [ ] Revisao de **escopo MVP** para o mes seguinte: congelar o que entra e o que fica fora

## Criterio de sucesso do mes

- Todo push relevante passa em `cargo build` + `cargo test` (local ou GitHub)
- Nenhuma duvida de "qual pasta e a engine oficial"
- Um ganho mensuravel de manutencao (modulo extra) ou de performance (colisao)
