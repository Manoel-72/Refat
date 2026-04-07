# RS2BR Engine — ajustes de input, spawn e emissor nativo

## O que foi corrigido

- Ampliação do snapshot de input Lua: `Q`, `E`, `F`, `Shift`, `Ctrl`, `0..9`.
- `game.spawn_entity(...)` e `game.spawn_prefab(...)` agora retornam um handle de request com inicialização de spawn (`set_velocity`, `add_tag`, `set_hp`, `set_anim`).
- Coleta real dos comandos de spawn do `game._cmds` no bridge Lua → runtime.
- Limpeza dos comandos coletados para evitar reaplicação em frames seguintes.
- Novo emissor CPU nativo simples via `game.spawn_emitter(...)`, atualizado pelo runtime sem depender de script por frame.

## Limitações restantes

- O handle retornado pelo spawn ainda é um handle de request, não um ponteiro para a entidade já criada no mesmo frame.
- Gamepad não foi implementado neste patch porque o runtime embutido atual depende do input disponível no egui/host.
- O emissor nativo atual é intencionalmente simples: posição fixa, emissão por taxa, duração, cor e escala.
