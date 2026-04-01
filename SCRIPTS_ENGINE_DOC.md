# Rust2D Engine — Guia de Scripts e Input

## Como usar scripts na sua cena

1. Crie um arquivo `.rs` em `assets/scripts/` (ex: `player_controller.rs`).
2. No Inspector, adicione um componente **Script** na entidade desejada.
3. Arraste o arquivo `.rs` para o campo do Script ou digite o caminho.
4. Use diretivas especiais no topo do script para controlar o comportamento.

---

## Diretivas suportadas (coloque no topo do script)

- `@start_message Mensagem` — Mostra mensagem ao iniciar o script
- `@player_controller VELOCIDADE` — Permite mover com WASD
- `@camera_follow` — Faz a câmera seguir a entidade
- `@move_x VALOR` — Move automaticamente no eixo X
- `@move_y VALOR` — Move automaticamente no eixo Y
- `@rotate_speed VALOR` — Rotaciona automaticamente
- `@on_update ...` — Executa comandos customizados a cada frame (ver exemplos)

---

## Como acessar input e transform no script

Você pode usar as seguintes variáveis nas diretivas `@on_update`:

- `input.key_w`, `input.key_a`, `input.key_s`, `input.key_d` — teclas WASD
- `input.key_space`, `input.key_enter` — espaço, enter
- `input.mouse_left`, `input.mouse_right` — mouse
- `input.mouse_pos` — posição do cursor (x, y)
- `transform.x`, `transform.y`, `transform.rotation` — posição e rotação da entidade
- `delta` — tempo entre frames (delta_time)

---

## Exemplos práticos de diretivas @on_update

```rust
// Move para cima com W
@on_update if input.key_w { transform.y += 100.0 * delta; }
// Move para baixo com S
@on_update if input.key_s { transform.y -= 100.0 * delta; }
// Move para esquerda com A
@on_update if input.key_a { transform.x -= 100.0 * delta; }
// Move para direita com D
@on_update if input.key_d { transform.x += 100.0 * delta; }
// Rotaciona com espaço
@on_update if input.key_space { transform.rotation += 90.0 * delta; }
// Move para a direita ao clicar com o mouse esquerdo
@on_update if input.mouse_left { transform.x += 50.0 * delta; }
// Move para a esquerda ao clicar com o mouse direito
@on_update if input.mouse_right { transform.x -= 50.0 * delta; }
```

---

## Dicas

- Você pode combinar várias diretivas no mesmo script.
- O campo `delta` garante movimento suave independente do FPS.
- Para ações mais avançadas, peça novas diretivas ou padrões!

---

## Exemplo completo de script

```rust
// @start_message Jogador pronto!
// @player_controller 240
// @camera_follow
// @on_update if input.key_w { transform.y += 100.0 * delta; }
// @on_update if input.key_space { transform.rotation += 90.0 * delta; }

pub struct PlayerController;

impl PlayerController {
    pub fn start(&mut self) {
        println!("PlayerController iniciado!");
    }

    pub fn update(&mut self, delta_time: f32) {
        // O runtime executa as diretivas acima automaticamente.
    }
}
```

---

## Observações

- O sistema de scripts atualmente interpreta diretivas e comandos simples.
- Para execução de código Rust real, consulte a documentação futura.
- Sugestões e dúvidas: abra um issue ou envie feedback!
