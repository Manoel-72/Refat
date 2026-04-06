# Checklist oficial de regressão — RS2BR-Engine V0.9.1

Use a cena `demo_platformer/assets/scenes/validation.scene.json` para validar a engine depois de mudanças no runtime, Lua ou colisão.

## Fluxo principal
1. Abrir `validation.scene.json`.
2. Pressionar **Play**.
3. Coletar uma moeda e confirmar:
   - a moeda sobe/desce levemente
   - a moeda some ao tocar
   - `session.score` sobe
4. Coletar o pickup de validação e confirmar:
   - `session.pickups` sobe
   - `entity.destroy()` remove o pickup no fim do frame
5. Encostar no inimigo e confirmar:
   - `collision_enter/stay/exit` continuam respondendo
   - a patrulha do inimigo continua estável
6. Salvar o jogo com **💾 Salvar**.
7. Mover o player para outro ponto e fechar/parar.
8. Usar **⤴ Continuar** e confirmar:
   - a cena salva volta a abrir
   - o player reaparece na posição salva
   - a velocidade do player é restaurada de forma segura
   - `session.*` volta limpo
9. Usar **▶ Novo jogo** e confirmar:
   - `session.score` zera
   - `session.pickups` zera
   - `state.*` do gameplay reinicia
10. Confirmar que `save.best_score` continua disponível após salvar e continuar.

## Cobertura esperada
- `save.*` persistente
- `session.*` por partida
- `state.*` por entidade/script
- `collision_enabled` desacoplado de `visible`
- `collision_enter/stay/exit`
- `entity.destroy()` adiado para fim do frame
- fluxo oficial de **Novo jogo** vs **Continuar**

## Observações
- A engine ainda recomenda `game.delta_time()` e `game.elapsed_time()` como API oficial.
- Aliases antigos existem só para compatibilidade temporária.
- O restore de continue nesta fase mira o estado mínimo seguro: cena, posição e velocidade do player principal.
