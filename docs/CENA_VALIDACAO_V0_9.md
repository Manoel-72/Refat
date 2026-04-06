# Cena oficial de validação — RS2BR-Engine V0.9

Arquivo principal:
- `demo_platformer/assets/scenes/validation.scene.json`

Objetivo:
- servir como cena oficial de regressão do runtime
- provar no jogo a separação entre `save.*`, `session.*` e `state.*`
- validar `collision_enter/stay/exit`, `collision_enabled` e `entity.destroy()`

## O que a cena cobre

### 1) `session.*`
- `player_platformer.lua` inicializa `session.score`
- `coin_collector.lua` incrementa `session.score`
- `pickup_test.lua` incrementa `session.pickups`
- `score_hud.lua` mostra o score da sessão atual

### 2) `state.*`
- `enemy_patrol.lua` usa `state.origin_x` e `state.dir`
- `coin_collector.lua` usa `state.collected` e `state.base_y`
- `collision_probe.lua` usa `state.phase`

### 3) `save.*`
- `save_validation_hud.lua` espelha o melhor score em `save.best_score`
- isso demonstra que save é separado da sessão atual
- para persistir em disco, use salvar no runtime/editor e depois `Continuar`

### 4) Colisão
- `collision_probe.lua` valida:
  - `game.collision_enter("Player")`
  - `game.collision_stay("Player")`
  - `game.collision_exit("Player")`

### 5) Destruição segura
- `pickup_test.lua` usa `entity.destroy()`
- o pickup some no fim do frame sem quebrar a atualização

## Fluxo recomendado de teste

1. Abra `validation.scene.json` no editor.
2. Pressione `Play`.
3. Mova o Player e colete moedas:
   - o `ScoreHUD` deve subir usando `session.score`
4. Pegue o pickup verde:
   - o contador de pickups na HUD de validação deve subir
   - a entidade deve ser destruída com segurança
5. Encoste e saia da Probe roxa:
   - o terminal deve mostrar `ENTER` e `EXIT`
6. Observe o inimigo vermelho:
   - ele deve patrulhar usando `state.dir`
7. Salve o jogo/runtime.
8. Use `Continuar`:
   - `session.*` deve reiniciar para a nova rodada
   - `save.best_score` deve permanecer disponível

## Leitura esperada
- `session.*` muda durante a rodada e reinicia em novo jogo
- `state.*` pertence a cada entidade/script e não vaza entre entidades
- `save.*` representa progresso persistível e não substitui memória temporária
