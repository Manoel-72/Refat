use std::{collections::HashSet, path::{Path, PathBuf}, time::Instant};

use crate::{
    editor::EditorPlayState,
    core::scene::Scene,
    runtime::{camera, scene_manager::SceneManager, systems::{self, RuntimeCommand}},
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
    pub scene_manager: SceneManager,
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
            scene_manager: SceneManager::new(),
        }
    }

    pub fn start_from_scene(&mut self, scene: &Scene) {
        self.scene_manager.set_editor_scene(scene.clone());
        self.active_scene = self.scene_manager.current_scene.clone();
        self.reset_timing_state();
    }

    pub fn start_from_path(&mut self, path: PathBuf) -> Result<(), String> {
        self.scene_manager.load_scene(path)?;
        self.active_scene = self.scene_manager.current_scene.clone();
        self.reset_timing_state();
        Ok(())
    }

    pub fn queue_scene_change(&mut self, path: PathBuf) {
        self.scene_manager.change_scene(path);
    }

    pub fn reload_current_scene(&mut self, fallback_scene: &Scene) {
        if self.scene_manager.reload_scene().is_err() {
            self.start_from_scene(fallback_scene);
        } else {
            self.active_scene = self.scene_manager.current_scene.clone();
            self.reset_timing_state();
        }
    }

    pub fn stop(&mut self) {
        self.active_scene = None;
        self.scene_manager.clear_runtime_scene();
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
        if matches!(self.scene_manager.apply_pending_change(), Ok(true)) {
            self.active_scene = self.scene_manager.current_scene.clone();
            self.reset_timing_state();
        }

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

    fn reset_timing_state(&mut self) {
        self.elapsed_time = 0.0;
        self.delta_time = 0.0;
        self.frame_count = 0;
        self.last_frame_at = Some(Instant::now());
        self.started_scripts.clear();
        self.input = RuntimeInput::default();
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
            let runtime_command = systems::update_entities_runtime(
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
            self.scene_manager.current_scene = Some(scene.clone());

            if let Some(command) = runtime_command {
                match command {
                    RuntimeCommand::ChangeScene(path) => {
                        self.queue_scene_change(project_root.join(path));
                    }
                    RuntimeCommand::ReloadScene => {
                        self.scene_manager.change_scene(
                            self.scene_manager
                                .current_path
                                .clone()
                                .unwrap_or_else(|| project_root.join("assets/scenes/fase1.scene.json")),
                        );
                    }
                }
            }
        }
    }
}
