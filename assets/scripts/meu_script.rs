// Script: meu_script.rs
// Criado pelo Rust2D Engine
//
// Este script é anexado a uma entidade via componente Script.
// O runtime atual interpreta as diretivas abaixo durante o Play:
//
// @start_message Olá, mundo!
// @move_x 80
// @move_y 0
// @rotate_speed 45
// @player_controller 220
// @camera_follow
//
// Também são aceitos formatos estilo Rust dentro do arquivo:
// move_x = 80;
// rotate_speed = 45;
// player_controller = 220;

pub struct MeuScript {
    // Campos do script aqui
}

impl MeuScript {
    /// Chamado uma vez quando o jogo começa
    pub fn start(&mut self) {
        println!("Script 'meu_script.rs' iniciado!");
    }

    /// Chamado todo frame
    pub fn update(&mut self, delta_time: f32) {
        // delta_time = tempo em segundos desde o último frame
        // Exemplo de script compatível com o runtime atual:
        // move_x = 80;
        // rotate_speed = 45;
        let _ = delta_time;
    }
}
