# RS2BR-Engine V0.7-B

Entrega incremental da V0.7-B com foco em mudanças pequenas, seguras e locais.

## O que foi fechado nesta correção

- popup inicial com versão atual da engine e botão **OK**
- título da janela com versão **V0.7-B** e status **EM TESTE**
- HUD mínimo no runtime
- menu principal mínimo quando a cena ativa é uma cena de menu
- pause/resume por botão e por **ESC**
- loading simples durante troca de cena
- spawn simples via fila no fim do frame
- destroy simples via fila no fim do frame
- cenas de teste em `assets/scenes`

## Estrutura de teste

- `assets/scenes/menu.scene.json`
- `assets/scenes/fase1.scene.json`
- `assets/scripts/player_controller.rs2`

## Como rodar

1. Abra o projeto Rust normalmente.
2. Compile e execute a engine.
3. Ao iniciar, aparecerá um pop-up com a versão atual. Clique em **OK**.
4. Abra a cena `assets/scenes/menu.scene.json` ou `assets/scenes/fase1.scene.json`.
5. Entre em **Play**.

## Como testar

### Menu
- Abra `menu.scene.json`.
- Entre em Play.
- O overlay **Menu Principal** deve aparecer.
- Clique em **Play** para agendar a primeira cena jogável encontrada.
- Clique em **Exit** para sair do runtime.

### Play
- Abra `fase1.scene.json`.
- Entre em Play.
- O player deve aparecer na cena.

### Pause
- Durante o runtime, use o botão **Pause/Resume** ou pressione **ESC**.
- Em pausa, o gameplay deve congelar.

### Scene transition
- Use os botões de troca de cena no runtime.
- A troca deve ser agendada com pending scene change.
- O texto `Loading...` pode aparecer durante a transição.

### Camera follow
- Na cena `fase1.scene.json`, o script do player usa `@camera_follow`.
- Ao mover o player, a câmera principal deve seguir.

### Spawn
- No runtime, clique em **Spawn Enemy**.
- A entidade é criada por fila no final do frame.

### Destroy
- Após spawnar, clique em **Destroy Last Spawn**.
- A última entidade spawnada é removida por fila no final do frame.

### Loading
- Troque de cena no runtime.
- O estado de fluxo entra em `Loading` antes de voltar para `Playing`.

## Limitações atuais

- o spawn usa template simples interno, não prefab completo
- o HUD é mínimo
- o menu principal é um overlay simples
- o loading é textual
- não houve validação de build neste ambiente

## Arquivos alterados principais

- `src/core/version.rs`
- `src/core/mod.rs`
- `src/main.rs`
- `src/editor/mod.rs`
- `src/runtime/mod.rs`
- `src/runtime/state.rs`
- `src/runtime/systems/input_system.rs`
- `src/runtime/input/key_code.rs`
- `Cargo.toml`

## Observação

A prioridade desta entrega foi manter o projeto incremental e estável, sem refatoração grande.
