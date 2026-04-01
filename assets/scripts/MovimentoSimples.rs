// Script: MovimentoSimples.rs
// Criado pelo Rust2D Engine
//
// Este script é anexado a uma entidade via componente Script.
// O runtime atual interpreta as diretivas abaixo durante o Play:
//
// @start_message Olá, mundo!

// @move_y 0

// @player_controller 220
// @camera_follow
//
// Também são aceitos formatos estilo Rust dentro do arquivo:

// player_controller = 220;

// Script de movimentação simples para Rust2D Engine

pub struct MovimentoSimples {
    pub x: f32,
    pub velocidade: f32,
}

impl MovimentoSimples {
    pub fn start(&mut self) {
        self.x = 0.0;
        self.velocidade = 100.0; // pixels por segundo
    }

    pub fn update(&mut self, delta: f32, input: &InputState) {
        // Move para a esquerda
        if input.key_down("A") || input.key_down("ArrowLeft") {
            self.x -= self.velocidade * delta;
        }
        // Move para a direita
        if input.key_down("D") || input.key_down("ArrowRight") {
            self.x += self.velocidade * delta;
        }
        // Aqui você pode aplicar self.x à posição da entidade
    }
}
