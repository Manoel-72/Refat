use eframe::egui;

use crate::{
    editor::{EditorApp, EditorPlayState},
    runtime::{camera, systems, RuntimeInput},
};

pub fn apply_runtime_inputs(app: &mut EditorApp, ctx: &egui::Context) {
    let step = if app.play_state == EditorPlayState::Paused {
        1.0 / 60.0
    } else {
        app.runtime.delta_time.max(1.0 / 120.0)
    };

    ctx.input(|i| {
        let mut input = RuntimeInput::default();
        input.key_w = i.key_down(egui::Key::W);
        input.key_a = i.key_down(egui::Key::A);
        input.key_s = i.key_down(egui::Key::S);
        input.key_d = i.key_down(egui::Key::D);
        input.key_up = i.key_down(egui::Key::ArrowUp);
        input.key_down = i.key_down(egui::Key::ArrowDown);
        input.key_left = i.key_down(egui::Key::ArrowLeft);
        input.key_right = i.key_down(egui::Key::ArrowRight);
        input.key_space = i.key_down(egui::Key::Space);
        input.key_enter = i.key_down(egui::Key::Enter);
        input.mouse_left = i.pointer.button_down(egui::PointerButton::Primary);
        input.mouse_right = i.pointer.button_down(egui::PointerButton::Secondary);
        input.mouse_middle = i.pointer.button_down(egui::PointerButton::Middle);
        input.mouse_pos = i
            .pointer
            .hover_pos()
            .map(|p| (p.x, p.y))
            .unwrap_or((0.0, 0.0));
        app.runtime.input = input;
    });

    let (player_input, camera_input, zoom_input, reload_scene) = ctx.input(|i| {
        let mut player = egui::vec2(0.0, 0.0);
        if i.key_down(egui::Key::A) {
            player.x -= 1.0;
        }
        if i.key_down(egui::Key::D) {
            player.x += 1.0;
        }
        if i.key_down(egui::Key::W) {
            player.y += 1.0;
        }
        if i.key_down(egui::Key::S) {
            player.y -= 1.0;
        }

        let mut camera_move = egui::vec2(0.0, 0.0);
        if i.key_down(egui::Key::ArrowLeft) {
            camera_move.x -= 1.0;
        }
        if i.key_down(egui::Key::ArrowRight) {
            camera_move.x += 1.0;
        }
        if i.key_down(egui::Key::ArrowUp) {
            camera_move.y += 1.0;
        }
        if i.key_down(egui::Key::ArrowDown) {
            camera_move.y -= 1.0;
        }

        let mut zoom = 0.0;
        if i.key_down(egui::Key::Q) {
            zoom -= 1.0;
        }
        if i.key_down(egui::Key::E) {
            zoom += 1.0;
        }
        zoom += i.raw_scroll_delta.y * 0.02;

        (player, camera_move, zoom, i.key_pressed(egui::Key::R))
    });

    if reload_scene {
        app.runtime.reload_current_scene(&app.scene);
        return;
    }

    let Some(scene) = app.runtime.active_scene.as_mut() else {
        return;
    };

    if player_input != egui::Vec2::ZERO {
        systems::apply_player_controller_input(
            &mut scene.entities,
            &app.project_root,
            step,
            player_input,
        );
    }

    if camera_input != egui::Vec2::ZERO || zoom_input.abs() > f32::EPSILON {
        let camera_speed = 260.0 * step;
        camera::move_main_camera(
            &mut scene.entities,
            camera_input.x * camera_speed,
            camera_input.y * camera_speed,
            zoom_input * step.max(0.02),
        );
    }

    app.runtime.scene_manager.current_scene = Some(scene.clone());
}
