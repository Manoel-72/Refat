use eframe::egui;

use crate::runtime::{input::key_code::KeyCode, RuntimeInput};

pub fn capture_runtime_input(ctx: &egui::Context, previous: &RuntimeInput) -> RuntimeInput {
    let mut keyboard = previous.keyboard.clone();
    keyboard.begin_frame();

    // wants_keyboard_input é true quando há um TextEdit focado.
    // Nesse caso ainda capturamos as teclas de jogo (WASD, setas, Space),
    // mas ignoramos apenas se o contexto for de digitação explícita.
    // A solução correta: sempre ler key_down — o egui não "consome" key_down,
    // só key_pressed em alguns widgets. key_down reflete o estado físico da tecla.
    ctx.input(|i| {
        keyboard.set_key_down(KeyCode::W, i.key_down(egui::Key::W));
        keyboard.set_key_down(KeyCode::A, i.key_down(egui::Key::A));
        keyboard.set_key_down(KeyCode::S, i.key_down(egui::Key::S));
        keyboard.set_key_down(KeyCode::D, i.key_down(egui::Key::D));
        keyboard.set_key_down(KeyCode::Q, i.key_down(egui::Key::Q));
        keyboard.set_key_down(KeyCode::E, i.key_down(egui::Key::E));
        keyboard.set_key_down(KeyCode::F, i.key_down(egui::Key::F));
        keyboard.set_key_down(KeyCode::Up, i.key_down(egui::Key::ArrowUp));
        keyboard.set_key_down(KeyCode::Down, i.key_down(egui::Key::ArrowDown));
        keyboard.set_key_down(KeyCode::Left, i.key_down(egui::Key::ArrowLeft));
        keyboard.set_key_down(KeyCode::Right, i.key_down(egui::Key::ArrowRight));
        keyboard.set_key_down(KeyCode::Space, i.key_down(egui::Key::Space));
        keyboard.set_key_down(KeyCode::Enter, i.key_down(egui::Key::Enter));
        keyboard.set_key_down(KeyCode::Escape, i.key_down(egui::Key::Escape));
        keyboard.set_key_down(KeyCode::Shift, i.modifiers.shift);
        keyboard.set_key_down(KeyCode::Ctrl, i.modifiers.ctrl || i.modifiers.command);
        keyboard.set_key_down(KeyCode::Num0, i.key_down(egui::Key::Num0));
        keyboard.set_key_down(KeyCode::Num1, i.key_down(egui::Key::Num1));
        keyboard.set_key_down(KeyCode::Num2, i.key_down(egui::Key::Num2));
        keyboard.set_key_down(KeyCode::Num3, i.key_down(egui::Key::Num3));
        keyboard.set_key_down(KeyCode::Num4, i.key_down(egui::Key::Num4));
        keyboard.set_key_down(KeyCode::Num5, i.key_down(egui::Key::Num5));
        keyboard.set_key_down(KeyCode::Num6, i.key_down(egui::Key::Num6));
        keyboard.set_key_down(KeyCode::Num7, i.key_down(egui::Key::Num7));
        keyboard.set_key_down(KeyCode::Num8, i.key_down(egui::Key::Num8));
        keyboard.set_key_down(KeyCode::Num9, i.key_down(egui::Key::Num9));

        RuntimeInput {
            keyboard,
            mouse_left: i.pointer.button_down(egui::PointerButton::Primary),
            mouse_right: i.pointer.button_down(egui::PointerButton::Secondary),
            mouse_middle: i.pointer.button_down(egui::PointerButton::Middle),
            mouse_pos: i
                .pointer
                .hover_pos()
                .map(|p| (p.x, p.y))
                .unwrap_or((0.0, 0.0)),
            gamepad_buttons: std::collections::HashSet::new(),
            gamepad_axes: std::collections::HashMap::new(),
        }
    })
}

pub fn player_axis(input: &RuntimeInput) -> egui::Vec2 {
    let mut axis = egui::vec2(0.0, 0.0);
    if input.is_key_held(KeyCode::A) {
        axis.x -= 1.0;
    }
    if input.is_key_held(KeyCode::D) {
        axis.x += 1.0;
    }
    if input.is_key_held(KeyCode::W) {
        axis.y += 1.0;
    }
    if input.is_key_held(KeyCode::S) {
        axis.y -= 1.0;
    }
    axis
}

pub fn camera_axis(input: &RuntimeInput) -> egui::Vec2 {
    let mut axis = egui::vec2(0.0, 0.0);
    if input.is_key_held(KeyCode::Left) {
        axis.x -= 1.0;
    }
    if input.is_key_held(KeyCode::Right) {
        axis.x += 1.0;
    }
    if input.is_key_held(KeyCode::Up) {
        axis.y += 1.0;
    }
    if input.is_key_held(KeyCode::Down) {
        axis.y -= 1.0;
    }
    axis
}

pub fn zoom_delta(ctx: &egui::Context) -> f32 {
    ctx.input(|i| {
        let mut zoom = 0.0;
        if i.key_down(egui::Key::Q) {
            zoom -= 1.0;
        }
        if i.key_down(egui::Key::E) {
            zoom += 1.0;
        }
        zoom + i.raw_scroll_delta.y * 0.02
    })
}
