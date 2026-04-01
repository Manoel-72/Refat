
// Script de exemplo para controlar um personagem no runtime.
// Anexe este arquivo no componente Script da entidade desejada.
//
// Diretivas interpretadas no Play:
// @start_message Jogador pronto para a cena!
// @player_controller 240
// @camera_follow

// Como acessar input e transform (via diretivas):
// input.key_w, input.key_a, input.key_s, input.key_d  // teclas WASD
// input.key_space, input.key_enter                   // espaço, enter
// input.mouse_left, input.mouse_right                // mouse
// input.mouse_pos                                    // posição do cursor (x, y)
// transform.x, transform.y, transform.rotation       // posição e rotação da entidade
// Você pode ler e modificar transform.x/y/rotation dentro de update()
// Exemplos de diretivas:
// @on_update if input.key_w { transform.y += 100.0 * delta; }
// @on_update if input.key_s { transform.y -= 100.0 * delta; }
// @on_update if input.key_a { transform.x -= 100.0 * delta; }
// @on_update if input.key_d { transform.x += 100.0 * delta; }
// @on_update if input.key_space { transform.rotation += 90.0 * delta; }
// @on_update if input.mouse_left { transform.x += 50.0 * delta; }
// @on_update if input.mouse_right { transform.x -= 50.0 * delta; }
// @on_update if input.mouse_pos { /* use input.mouse_pos.0 e .1 para acessar x/y do cursor */ }
//
// Basta adicionar essas linhas no topo do seu script para controlar a entidade!

pub struct PlayerController;

impl PlayerController {
    pub fn start(&mut self) {
        println!("PlayerController iniciado!");
    }

    pub fn update(&mut self, delta_time: f32) {
        let _ = delta_time;
        // O runtime atual usa as diretivas acima para mover a entidade com WASD.
    }
}
