# rs2br-engine - Distribuição (dist/release)

Este diretório contém um build `release` do binário `rs2br-engine` e os ativos básicos para iniciar um projeto.

Como usar:

- No Windows, execute `rs2br-engine.exe` em `dist/release`.
- Se preferir, extraia o ZIP `dist/rs2br-engine-windows.zip` e rode o binário.

Conteúdo esperado:
- `rs2br-engine.exe` (ou `rs2br-engine`) — binário compilado em release.
- `assets/` — pasta de ativos usada pelo engine (se presente no repositório original).
- `project.json` e `Cargo.toml` — arquivos auxiliares copiados para referência.

Observações:
- Este pacote é um ponto de partida. Para "montar o jogo do zero", crie um novo projeto (pasta) dentro de `dist/release` com `project.json` e `assets/` seguindo os formatos em `docs/`.
- Quer que eu gere um template de projeto minimal dentro de `dist/release` (ex.: `demo_project/`)?