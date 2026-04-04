# Formato de cena

```json
{
  "name": "Main",
  "background_color": [0.15, 0.15, 0.18],
  "entities": []
}
```

## Componentes novos na V0.8
### Animator
```json
{
  "Animator": {
    "current": "idle",
    "timer": 0.0,
    "playing": true,
    "looped": true,
    "clips": {
      "idle": {
        "frames": [
          "assets/sprites/player_idle_01.png",
          "assets/sprites/player_idle_02.png"
        ],
        "fps": 8.0
      }
    }
  }
}
```

### TextLabel
```json
{
  "TextLabel": {
    "text": "Score: 0",
    "font_size": 24.0,
    "color_r": 1.0,
    "color_g": 1.0,
    "color_b": 1.0,
    "color_a": 1.0,
    "screen_space": true
  }
}
```

### UIButton
```json
{
  "UIButton": {
    "text": "Jogar",
    "width": 220.0,
    "height": 48.0,
    "font_size": 20.0,
    "target_scene": "assets/scenes/fase1.scene.json",
    "close_runtime": false,
    "color_r": 0.18,
    "color_g": 0.58,
    "color_b": 0.32,
    "color_a": 1.0,
    "text_r": 1.0,
    "text_g": 1.0,
    "text_b": 1.0,
    "text_a": 1.0,
    "screen_space": true
  }
}
```
