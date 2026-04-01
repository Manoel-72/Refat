// Script de exemplo para girar continuamente um sprite.
//
// @start_message Girando sprite...
// @rotate_speed 90

pub struct SpinDemo;

impl SpinDemo {
    pub fn start(&mut self) {
        println!("SpinDemo iniciado!");
    }

    pub fn update(&mut self, delta_time: f32) {
        let _ = delta_time;
        // O runtime atual usa @rotate_speed para girar automaticamente.
    }
}
