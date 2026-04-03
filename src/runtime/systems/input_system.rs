use eframe::egui;

use crate::runtime::{
    input::key_code::KeyCode,
    RuntimeInput,
};

pub fn capture_runtime_input(ctx: &egui::Context, previous: &RuntimeInput) -> RuntimeInput {
    let mut keyboard = previous.keyboard.clone();
    keyboard.begin_frame();

    ctx.input(|i| {
        keyboard.set_key_down(KeyCode::W, i.key_down(egui::Key::W));
        keyboard.set_key_down(KeyCode::A, i.key_down(egui::Key::A));
        keyboard.set_key_down(KeyCode::S, i.key_down(egui::Key::S));
        keyboard.set_key_down(KeyCode::D, i.key_down(egui::Key::D));
        keyboard.set_key_down(KeyCode::Up, i.key_down(egui::Key::ArrowUp));
        keyboard.set_key_down(KeyCode::Down, i.key_down(egui::Key::ArrowDown));
        keyboard.set_key_down(KeyCode::Left, i.key_down(egui::Key::ArrowLeft));
        keyboard.set_key_down(KeyCode::Right, i.key_down(egui::Key::ArrowRight));
        keyboard.set_key_down(KeyCode::Space, i.key_down(egui::Key::Space));
        keyboard.set_key_down(KeyCode::Enter, i.key_down(egui::Key::Enter));
        keyboard.set_key_down(KeyCode::Escape, i.key_down(egui::Key::Escape));

        RuntimeInput {
            keyboard,
            mouse_left: i.pointer.button_down(egui::PointerButton::Primary),
            mouse_right: i.pointer.button_down(egui::PointerButton::Secondary),
            mouse_middle: i.pointer.button_down(egui::PointerButton::Middle),
            mouse_pos: i.pointer.hover_pos().map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0)),
        }
    })
}

pub fn player_axis(input: &RuntimeInput) -> egui::Vec2 {
    let mut axis = egui::vec2(0.0, 0.0);
    if input.is_key_held(KeyCode::A) { axis.x -= 1.0; }
    if input.is_key_held(KeyCode::D) { axis.x += 1.0; }
    if input.is_key_held(KeyCode::W) { axis.y += 1.0; }
    if input.is_key_held(KeyCode::S) { axis.y -= 1.0; }
    axis
}

pub fn camera_axis(input: &RuntimeInput) -> egui::Vec2 {
    let mut axis = egui::vec2(0.0, 0.0);
    if input.is_key_held(KeyCode::Left) { axis.x -= 1.0; }
    if input.is_key_held(KeyCode::Right) { axis.x += 1.0; }
    if input.is_key_held(KeyCode::Up) { axis.y += 1.0; }
    if input.is_key_held(KeyCode::Down) { axis.y -= 1.0; }
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
