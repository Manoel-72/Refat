use std::{collections::HashSet, path::Path, time::Instant};

use crate::{
    editor::EditorPlayState,
    engine::scene::Scene,
    runtime::{camera, systems},
};

#[derive(Default, Clone)]
pub struct RuntimeInput {
    pub key_w: bool,
    pub key_a: bool,
    pub key_s: bool,
    pub key_d: bool,
    pub key_up: bool,
    pub key_down: bool,
    pub key_left: bool,
    pub key_right: bool,
    pub key_space: bool,
    pub key_enter: bool,
    pub mouse_left: bool,
    pub mouse_right: bool,
    pub mouse_middle: bool,
    pub mouse_pos: (f32, f32),
}

pub struct RuntimeState {
    pub active_scene: Option<Scene>,
    pub elapsed_time: f32,
    pub delta_time: f32,
    pub frame_count: u64,
    last_frame_at: Option<Instant>,
    pub window_open: bool,
    pub started_scripts: HashSet<String>,
    pub input: RuntimeInput,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            active_scene: None,
            elapsed_time: 0.0,
            delta_time: 0.0,
            frame_count: 0,
            last_frame_at: None,
            window_open: false,
            started_scripts: HashSet::new(),
            input: RuntimeInput::default(),
        }
    }

    pub fn start_from_scene(&mut self, scene: &Scene) {
        self.active_scene = Some(scene.clone());
        self.elapsed_time = 0.0;
        self.delta_time = 0.0;
        self.frame_count = 0;
        self.last_frame_at = Some(Instant::now());
        self.started_scripts.clear();
        self.input = RuntimeInput::default();
    }

    pub fn stop(&mut self) {
        self.active_scene = None;
        self.elapsed_time = 0.0;
        self.delta_time = 0.0;
        self.frame_count = 0;
        self.last_frame_at = None;
        self.started_scripts.clear();
        self.input = RuntimeInput::default();
    }

    pub fn sync_with_mode(
        &mut self,
        mode: EditorPlayState,
        source_scene: &Scene,
        project_root: &Path,
        ground_y: f32,
    ) {
        match mode {
            EditorPlayState::Edit => {
                if self.active_scene.is_some() {
                    self.stop();
                }
                self.window_open = false;
            }
            EditorPlayState::Playing => {
                if self.active_scene.is_none() {
                    self.start_from_scene(source_scene);
                }
                self.window_open = true;
                self.update_frame(project_root, ground_y);
            }
            EditorPlayState::Paused => {
                if self.active_scene.is_none() {
                    self.start_from_scene(source_scene);
                }
                self.window_open = true;
                self.delta_time = 0.0;
                self.last_frame_at = Some(Instant::now());
            }
        }
    }

    pub fn estimated_fps(&self) -> f32 {
        if self.delta_time <= f32::EPSILON {
            0.0
        } else {
            1.0 / self.delta_time
        }
    }

    fn update_frame(&mut self, project_root: &Path, ground_y: f32) {
        let now = Instant::now();
        let dt = self
            .last_frame_at
            .map(|last| (now - last).as_secs_f32().clamp(1.0 / 240.0, 1.0 / 20.0))
            .unwrap_or(1.0 / 60.0);

        self.last_frame_at = Some(now);
        self.delta_time = dt;
        self.elapsed_time += dt;
        self.frame_count += 1;

        if let Some(scene) = &mut self.active_scene {
            let mut camera_follow_target = None;
            systems::update_entities_runtime(
                &mut scene.entities,
                dt,
                project_root,
                &mut self.started_scripts,
                &mut camera_follow_target,
                ground_y,
            );
            if let Some((x, y)) = camera_follow_target {
                camera::set_main_camera_position(&mut scene.entities, x, y);
            }
        }
    }
}
