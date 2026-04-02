use eframe::egui;

use crate::{
    editor::{EditorApp, EditorPlayState},
    runtime::{camera, systems},
};

pub fn apply_runtime_inputs(app: &mut EditorApp, ctx: &egui::Context) {
    let step = if app.play_state == EditorPlayState::Paused {
        1.0 / 60.0
    } else {
        app.runtime.delta_time.max(1.0 / 120.0)
    };

    app.runtime.input = systems::input_system::capture_runtime_input(ctx);

    let player_input = systems::input_system::player_axis(&app.runtime.input);
    let camera_input = systems::input_system::camera_axis(&app.runtime.input);
    let zoom_input = systems::input_system::zoom_delta(ctx);
    let reload_scene = ctx.input(|i| i.key_pressed(egui::Key::R));

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
