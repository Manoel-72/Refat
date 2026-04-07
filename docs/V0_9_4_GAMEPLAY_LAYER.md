# RS2BR-Engine V0.9.4

## Entrou nesta versão
- damage system leve por entidade
- camera shake e zoom via Lua
- set_anim/get_anim/is_anim_finished/flip_x
- demo limpa com uma única cena principal
- botões de runtime para reset/exit
- canvas UI simples usando sprite screen_space
- prefab de inimigo para teste de spawn

## Limitações conhecidas
- `game.spawn_entity` e `game.spawn_prefab` continuam assíncronos por frame; o retorno direto do handle ainda não foi fechado nesta versão
- botão Lua customizado (`on_click`) ainda não foi adicionado; a UI Button continua oficial para ações simples de cena/fechar runtime
- o canvas é um painel screen-space simples, não um editor dedicado de partículas
