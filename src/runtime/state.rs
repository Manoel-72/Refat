use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{
    core::{
        scene::Scene,
        entity::Entity,
        component::{Component, Sprite, BoxCollider, RigidBody2D, Velocity},
    },
    editor::EditorPlayState,
    runtime::{
        camera,
        input::{input_state::InputState, key_code::KeyCode},
        scene_manager::SceneManager,
        systems::{self, audio_system::AudioRuntime, RuntimeCommand},
    },
};


#[derive(Debug, Clone)]
pub struct PendingSpawnRequest {
    pub template: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct PendingDestroyRequest {
    pub entity_id: String,
}

#[derive(Clone, Default)]
pub struct RuntimeInput {
    pub keyboard: InputState,
    pub mouse_left: bool,
    pub mouse_right: bool,
    pub mouse_middle: bool,
    pub mouse_pos: (f32, f32),
}

impl RuntimeInput {
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keyboard.is_pressed(key)
    }

    pub fn is_key_held(&self, key: KeyCode) -> bool {
        self.keyboard.is_held(key)
    }

    pub fn is_key_released(&self, key: KeyCode) -> bool {
        self.keyboard.is_released(key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuntimeFrameStage {
    #[default]
    Idle,
    CaptureInput,
    ApplyPlayerInput,
    UpdateScriptsAndMovement,
    ApplyPhysics,
    ResolveCollisions,
    UpdateCamera,
    FinalizeFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuntimeGameFlow {
    #[default]
    Editing,
    Playing,
    Paused,
    GameOver,
    Loading,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeGameState {
    pub flow: RuntimeGameFlow,
    pub score: i32,
    pub loading_label: Option<String>,
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
    pub game_state: RuntimeGameState,
    pub last_stage: RuntimeFrameStage,
    pub started_audio: HashSet<String>,
    pub audio_runtime: AudioRuntime,
    pub pending_spawns: Vec<PendingSpawnRequest>,
    pub pending_destroys: Vec<PendingDestroyRequest>,
    pub last_spawned_entity_id: Option<String>,
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
            game_state: RuntimeGameState::default(),
            last_stage: RuntimeFrameStage::Idle,
            started_audio: HashSet::new(),
            audio_runtime: AudioRuntime::new(),
            pending_spawns: Vec::new(),
            pending_destroys: Vec::new(),
            last_spawned_entity_id: None,
        }
    }

    pub fn start_from_scene(&mut self, scene: &Scene) {
        self.scene_manager.set_editor_scene(scene.clone());
        self.active_scene = self.scene_manager.current_scene.clone();
        self.reset_timing_state();
        self.game_state.flow = RuntimeGameFlow::Playing;
    }

    pub fn start_from_path(&mut self, path: PathBuf) -> Result<(), String> {
        self.scene_manager.load_scene(path)?;
        self.active_scene = self.scene_manager.current_scene.clone();
        self.reset_timing_state();
        self.game_state.flow = RuntimeGameFlow::Playing;
        Ok(())
    }

    pub fn queue_scene_change(&mut self, path: PathBuf) {
        self.game_state.flow = RuntimeGameFlow::Loading;
        self.game_state.loading_label = Some(path.display().to_string());
        self.scene_manager.change_scene(path);
    }

    pub fn reload_current_scene(&mut self, fallback_scene: &Scene) {
        self.game_state.flow = RuntimeGameFlow::Loading;
        if self.scene_manager.reload_scene().is_err() {
            self.start_from_scene(fallback_scene);
        } else {
            self.active_scene = self.scene_manager.current_scene.clone();
            self.reset_timing_state();
            self.game_state.flow = RuntimeGameFlow::Playing;
            self.game_state.loading_label = None;
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
        self.started_audio.clear();
        self.audio_runtime.stop_all();
        self.input = RuntimeInput::default();
        self.last_stage = RuntimeFrameStage::Idle;
        self.pending_spawns.clear();
        self.pending_destroys.clear();
        self.last_spawned_entity_id = None;
        self.game_state = RuntimeGameState::default();
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
            self.game_state.flow = RuntimeGameFlow::Playing;
            self.game_state.loading_label = None;
        }

        match mode {
            EditorPlayState::Edit => {
                if self.active_scene.is_some() {
                    self.stop();
                }
                self.window_open = false;
                self.game_state.flow = RuntimeGameFlow::Editing;
            }
            EditorPlayState::Playing => {
                if self.active_scene.is_none() {
                    self.start_from_scene(source_scene);
                }
                self.window_open = true;
                self.game_state.flow = RuntimeGameFlow::Playing;
                self.update_frame(project_root, ground_y);
            }
            EditorPlayState::Paused => {
                if self.active_scene.is_none() {
                    self.start_from_scene(source_scene);
                }
                self.window_open = true;
                self.delta_time = 0.0;
                self.last_frame_at = Some(Instant::now());
                self.last_stage = RuntimeFrameStage::Idle;
                self.game_state.flow = RuntimeGameFlow::Paused;
            }
        }
    }

    pub fn queue_spawn(&mut self, template: impl Into<String>, x: f32, y: f32) {
        self.pending_spawns.push(PendingSpawnRequest {
            template: template.into(),
            x,
            y,
        });
    }

    pub fn queue_destroy(&mut self, entity_id: impl Into<String>) {
        self.pending_destroys.push(PendingDestroyRequest {
            entity_id: entity_id.into(),
        });
    }

    pub fn queue_destroy_last_spawned(&mut self) -> bool {
        let Some(id) = self.last_spawned_entity_id.clone() else {
            return false;
        };
        self.queue_destroy(id);
        true
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
        self.started_audio.clear();
        self.audio_runtime.stop_all();
        self.input = RuntimeInput::default();
        self.last_stage = RuntimeFrameStage::Idle;
        self.pending_spawns.clear();
        self.pending_destroys.clear();
        self.last_spawned_entity_id = None;
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

        let player_input = systems::input_system::player_axis(&self.input);

        if let Some(scene) = &mut self.active_scene {
            let mut camera_follow_target = None;

            self.last_stage = RuntimeFrameStage::CaptureInput;

            self.last_stage = RuntimeFrameStage::ApplyPlayerInput;
            if player_input != eframe::egui::Vec2::ZERO {
                systems::apply_player_controller_input(
                    &mut scene.entities,
                    project_root,
                    dt,
                    player_input,
                );
            }

            self.last_stage = RuntimeFrameStage::UpdateScriptsAndMovement;
            systems::apply_audio_autoplay(
                &scene.entities,
                project_root,
                &mut self.started_audio,
                &mut self.audio_runtime,
            );
            self.audio_runtime.maintain();
            self.last_stage = RuntimeFrameStage::ApplyPhysics;
            self.last_stage = RuntimeFrameStage::ResolveCollisions;
            let runtime_command = systems::update_entities_runtime(
                &mut scene.entities,
                dt,
                project_root,
                &mut self.started_scripts,
                &mut camera_follow_target,
                ground_y,
            );

            self.last_stage = RuntimeFrameStage::UpdateCamera;
            if let Some((x, y)) = camera_follow_target {
                camera::set_main_camera_position(&mut scene.entities, x, y);
            }

            self.last_stage = RuntimeFrameStage::FinalizeFrame;
            let pending_destroys = std::mem::take(&mut self.pending_destroys);
            let pending_spawns = std::mem::take(&mut self.pending_spawns);
            Self::apply_pending_entity_commands(
                scene,
                pending_destroys,
                pending_spawns,
                &mut self.last_spawned_entity_id,
            );
            self.scene_manager.current_scene = Some(scene.clone());

            if let Some(command) = runtime_command {
                match command {
                    RuntimeCommand::ChangeScene(path) => {
                        self.queue_scene_change(project_root.join(path));
                    }
                    RuntimeCommand::ReloadScene => {
                        self.scene_manager.change_scene(
                            self.scene_manager.current_path.clone().unwrap_or_else(|| {
                                project_root.join("assets/scenes/fase1.scene.json")
                            }),
                        );
                    }
                }
            }
        }
    }

    fn apply_pending_entity_commands(
        scene: &mut Scene,
        pending_destroys: Vec<PendingDestroyRequest>,
        pending_spawns: Vec<PendingSpawnRequest>,
        last_spawned_entity_id: &mut Option<String>,
    ) {
        for entity_id in pending_destroys.into_iter().map(|request| request.entity_id) {
            if scene.remove_entity_by_id(&entity_id) {
                if last_spawned_entity_id.as_deref() == Some(entity_id.as_str()) {
                    *last_spawned_entity_id = None;
                }
            }
        }

        for request in pending_spawns {
            let entity = Self::build_spawn_entity(&request.template, request.x, request.y);
            *last_spawned_entity_id = Some(entity.id.clone());
            scene.add_entity(entity);
        }
    }

    fn build_spawn_entity(template: &str, x: f32, y: f32) -> Entity {
        let template_name = template.trim().to_ascii_lowercase();
        let mut entity = Entity::new(match template_name.as_str() {
            "enemy" => "Enemy",
            other if !other.is_empty() => other,
            _ => "Spawned Entity",
        });

        if let Some(transform) = entity.transform_mut() {
            transform.x = x;
            transform.y = y;
            transform.scale_x = 1.0;
            transform.scale_y = 1.0;
        }

        entity.add_component(Component::Velocity(Velocity::default()));
        entity.add_component(Component::RigidBody2D(RigidBody2D {
            gravity_scale: 1.0,
            is_static: false,
            grounded: false,
        }));
        entity.add_component(Component::BoxCollider(BoxCollider {
            width: 32.0,
            height: 32.0,
            offset_x: 0.0,
            offset_y: 0.0,
            is_trigger: false,
        }));
        entity.add_component(Component::Sprite(Sprite {
            texture_path: String::new(),
            color_r: 0.85,
            color_g: 0.25,
            color_b: 0.25,
            color_a: 1.0,
        }));
        entity
    }
}
