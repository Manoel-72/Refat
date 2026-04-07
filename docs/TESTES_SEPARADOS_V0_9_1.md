# Testes separados V0.9.1

Foram adicionadas quatro cenas isoladas para regressão:

- `demo_platformer/assets/scenes/v0_9_1_test_time.scene.json`
- `demo_platformer/assets/scenes/v0_9_1_test_events.scene.json`
- `demo_platformer/assets/scenes/v0_9_1_test_timers.scene.json`
- `demo_platformer/assets/scenes/v0_9_1_test_collision.scene.json`

## Objetivo
Separar cada validação para leitura mais limpa, menos sobreposição de texto e diagnóstico mais direto.

## O que mudou
- fontes menores e mais legíveis
- HUDs reposicionadas no canto superior esquerdo
- cena única `v0_9_1_regression.scene.json` mantida como visão geral
- nova HUD de colisão separada da entidade do player

## Uso recomendado
1. Rode `v0_9_1_test_time.scene.json`
2. Rode `v0_9_1_test_events.scene.json`
3. Rode `v0_9_1_test_timers.scene.json`
4. Rode `v0_9_1_test_collision.scene.json`

A cena única continua útil como smoke test rápido.
