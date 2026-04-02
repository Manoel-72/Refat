use eframe::egui;

use crate::runtime::RuntimeInput;

pub fn capture_runtime_input(ctx: &egui::Context) -> RuntimeInput {
    ctx.input(|i| RuntimeInput {
        key_w: i.key_down(egui::Key::W),
        key_a: i.key_down(egui::Key::A),
        key_s: i.key_down(egui::Key::S),
        key_d: i.key_down(egui::Key::D),
        key_up: i.key_down(egui::Key::ArrowUp),
        key_down: i.key_down(egui::Key::ArrowDown),
        key_left: i.key_down(egui::Key::ArrowLeft),
        key_right: i.key_down(egui::Key::ArrowRight),
        key_space: i.key_down(egui::Key::Space),
        key_enter: i.key_down(egui::Key::Enter),
        mouse_left: i.pointer.button_down(egui::PointerButton::Primary),
        mouse_right: i.pointer.button_down(egui::PointerButton::Secondary),
        mouse_middle: i.pointer.button_down(egui::PointerButton::Middle),
        mouse_pos: i.pointer.hover_pos().map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0)),
    })
}

pub fn player_axis(input: &RuntimeInput) -> egui::Vec2 {
    let mut axis = egui::vec2(0.0, 0.0);
    if input.key_a { axis.x -= 1.0; }
    if input.key_d { axis.x += 1.0; }
    if input.key_w { axis.y += 1.0; }
    if input.key_s { axis.y -= 1.0; }
    axis
}

pub fn camera_axis(input: &RuntimeInput) -> egui::Vec2 {
    let mut axis = egui::vec2(0.0, 0.0);
    if input.key_left { axis.x -= 1.0; }
    if input.key_right { axis.x += 1.0; }
    if input.key_up { axis.y += 1.0; }
    if input.key_down { axis.y -= 1.0; }
    axis
}

pub fn zoom_delta(ctx: &egui::Context) -> f32 {
    ctx.input(|i| {
        let mut zoom = 0.0;
        if i.key_down(egui::Key::Q) { zoom -= 1.0; }
        if i.key_down(egui::Key::E) { zoom += 1.0; }
        zoom + i.raw_scroll_delta.y * 0.02
    })
}
