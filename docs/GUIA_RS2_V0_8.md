# RS2 na V0.8

## Observação importante
Agora o runtime faz cache do parse dos scripts `.rs2`. Isso evita reler o arquivo inteiro a cada frame.

## Diretivas principais
- `@start_message`
- `@move_x`
- `@move_y`
- `@rotate_speed`
- `@player_controller`
- `@camera_follow`
- `@on_start`
- `@on_update`
- `@on_collision`
- `@on_trigger`

## Exemplo simples
```rs2
@start_message Player iniciado
@player_controller 220
@camera_follow
```
