# RS2 Top-Down Shooter Demo

Jogo simples feito usando a estrutura da RS2 Engine.

## Como testar

1. Extraia o ZIP.
2. Abra um terminal na pasta extraída.
3. Rode a engine normalmente:

```bash
cargo run
```

4. Dê Play no editor/runtime da RS2.
5. Clique em **Iniciar Jogo**.

## Controles

- **WASD** ou **Setas**: mover o player verde
- **Espaço**: atirar

## Regras

- O player é um quadrado verde.
- Existem 4 inimigos vermelhos.
- Cada inimigo morre com 3 tiros.
- Se matar os 4 inimigos: vitória.
- Se a vida chegar a 0: game over.

## Arquivos principais

- `assets/scenes/menu.scene.json`
- `assets/scenes/game.scene.json`
- `assets/scripts/player.lua`
- `assets/scripts/enemy.lua`
- `assets/scripts/bullet.lua`
- `assets/scripts/hud.lua`
- `assets/scripts/camera_follow.lua`


## Atualização do tiro

- O tiro agora é no botão esquerdo do mouse.
- A bala mira na posição do cursor no mundo.
- Cooldown reduzido para deixar o disparo mais responsivo.
- A bala avança no primeiro frame para reduzir a sensação de atraso/travamento.
