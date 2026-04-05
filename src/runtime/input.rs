pub mod input_state;
pub mod key_code;

use eframe::egui;

use crate::runtime::{camera, context::{RuntimeContext, RuntimePlayState}, systems, state::RuntimeState};

/// Captura e aplica inputs de câmera/zoom/reload no runtime.
/// Desacoplado do EditorApp — usa RuntimeContext.
pub fn apply_runtime_inputs<H: RuntimeContext>(host: &H, runtime: &mut RuntimeState, ctx: &egui::Context) {
    let step = if host.play_state() == RuntimePlayState::Paused {
        1.0 / 60.0
    } else {
        runtime.delta_time.max(1.0 / 120.0)
    };

    let previous_input = runtime.input.clone();
    runtime.input = systems::input_system::capture_runtime_input(ctx, &previous_input);

    let camera_input = systems::input_system::camera_axis(&runtime.input);
    let zoom_input   = systems::input_system::zoom_delta(ctx);
    let reload_scene = ctx.input(|i| i.key_pressed(egui::Key::R));

    if reload_scene {
        let fallback = host.active_scene_snapshot().clone();
        runtime.reload_current_scene(&fallback);
        return;
    }

    let Some(scene) = runtime.active_scene.as_mut() else {
        return;
    };

    if camera_input != egui::Vec2::ZERO || zoom_input.abs() > f32::EPSILON {
        let camera_speed = 260.0 * step;
        camera::move_main_camera(
            &mut scene.entities,
            camera_input.x * camera_speed,
            camera_input.y * camera_speed,
            zoom_input * step.max(0.02),
        );
    }

    runtime.scene_manager.current_scene = Some(scene.clone());
}
