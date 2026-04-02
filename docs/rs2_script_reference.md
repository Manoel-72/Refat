# RS2 Script Reference

## Extensão oficial
`.rs2`

## Pasta padrão
`assets/scripts/`

## Comandos suportados

### `@start_message`
Exibe uma mensagem quando a entidade inicia.

Exemplo:
```text
@start_message Olá mundo
```

### `@move_x`
Move no eixo X automaticamente.

Exemplo:
```text
@move_x 50
```

### `@move_y`
Move no eixo Y automaticamente.

Exemplo:
```text
@move_y -30
```

### `@rotate_speed`
Aplica rotação contínua em graus por segundo.

Exemplo:
```text
@rotate_speed 90
```

### `@player_controller`
Ativa controle de movimento por input.

Exemplo:
```text
@player_controller 220
```

### `@camera_follow`
Faz a câmera seguir a entidade.

Exemplo:
```text
@camera_follow
```

### `@on_collision change_scene`
Troca a cena quando houver colisão.

Exemplo:
```text
@on_collision change_scene assets/scenes/fase2.scene.json
```

### `@on_collision reload_scene`
Recarrega a cena atual quando houver colisão.

Exemplo:
```text
@on_collision reload_scene
```
