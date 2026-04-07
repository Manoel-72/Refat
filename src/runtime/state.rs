use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{
    core::{
        scene::Scene,
        entity::Entity,
        component::{Component, Sprite, BoxCollider, RigidBody2D, Velocity},
    },
    runtime::context::RuntimePlayState,
    runtime::{
        camera,
        input::{input_state::InputState, key_code::KeyCode},
        save::{SaveData, SaveValue},
        scene_manager::SceneManager,
        systems::{self, audio_system::AudioRuntime, RuntimeCommand},
    },
};

pub type ScriptStateScopeKey = String;
pub type ScriptState = HashMap<ScriptStateScopeKey, HashMap<String, SaveValue>>;

pub fn make_script_state_scope_key(entity_id: &str, script_path: &str) -> ScriptStateScopeKey {
    format!("{}::{}", entity_id, script_path)
}

pub fn script_scope_prefix(entity_id: &str) -> String {
    format!("{}::", entity_id)
}

const SAVE_KEY_PLAYER_ID: &str = "continue.player.id";
const SAVE_KEY_PLAYER_NAME: &str = "continue.player.name";
const SAVE_KEY_PLAYER_X: &str = "continue.player.x";
const SAVE_KEY_PLAYER_Y: &str = "continue.player.y";
const SAVE_KEY_PLAYER_VX: &str = "continue.player.vx";
const SAVE_KEY_PLAYER_VY: &str = "continue.player.vy";
const SAVE_KEY_CHECKPOINT_SCENE: &str = "continue.checkpoint.scene";

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

#[derive(Debug, Clone)]
pub struct RuntimeEvent {
    pub name: String,
    pub data: Option<SaveValue>,
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

#[derive(Debug, Clone, Default)]
pub struct RuntimePerfStats {
    pub scripts_ms: f32,
    pub collisions_ms: f32,
    pub hud_ms: f32,
}

#[derive(Debug, Clone)]
pub struct RuntimeDebugSnapshot {
    pub hud_summary: String,
    pub controls_hint: String,
    pub overlay_left: String,
    pub overlay_right: String,
    pub perf_summary: String,
}

impl Default for RuntimeDebugSnapshot {
    fn default() -> Self {
        Self {
            hud_summary: "▶ Runtime Preview | Status: Edição".to_string(),
            controls_hint: "WASD = player_controller | Setas = câmera | Q/E ou scroll = zoom | ESC = pause | Novo jogo limpa session/state".to_string(),
            overlay_left: "Entidades: 0  •  Scripts: 0  •  Delta: 0.000 ms".to_string(),
            overlay_right: "HUD/UI  •  Cena: -  •  Score: 0  •  Flow: Editing".to_string(),
            perf_summary: "Perf(ms) — scripts: 0.00 • colisão: 0.00 • HUD: 0.00".to_string(),
        }
    }
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
    /// Estado persistente do jogo (save/load em save/save.json).
    pub save_data: SaveData,
    /// Estado temporário da sessão atual (não persistido em arquivo por padrão).
    pub session_state: HashMap<String, SaveValue>,
    /// Estado temporário por entidade+script (memória de execução, não persistida).
    /// Chave: "entity_id::script_path"
    pub script_state: ScriptState,
    /// Cache de VMs Lua: chave = "entity_id::script_path", valor = (VM, mtime do arquivo).
    /// Evita criar uma Lua::new() por frame — criada uma vez, reutilizada.
    pub lua_vms: HashMap<String, (mlua::Lua, std::time::SystemTime)>,
    /// Eventos disponíveis para consumo neste frame.
    pub current_runtime_events: Vec<RuntimeEvent>,
    /// Eventos emitidos durante o frame atual para consumo no próximo frame.
    pub pending_runtime_events: Vec<RuntimeEvent>,
    /// Contatos de colisão do frame atual: entity_id -> nomes das entidades em contato.
    pub collision_contacts: HashMap<String, Vec<String>>,
    /// Contatos de colisão do frame anterior: entity_id -> nomes das entidades em contato.
    pub previous_collision_contacts: HashMap<String, Vec<String>>,
    pub collision_contact_ids: HashMap<String, Vec<String>>,
    pub previous_collision_contact_ids: HashMap<String, Vec<String>>,
    /// Entradas de colisão reais do frame atual: entity_id -> nomes que começaram contato neste frame.
    pub collision_enter_contacts: HashMap<String, Vec<String>>,
    /// Permanências de colisão reais do frame atual: entity_id -> nomes que continuaram em contato neste frame.
    pub collision_stay_contacts: HashMap<String, Vec<String>>,
    /// Saídas de colisão reais do frame atual: entity_id -> nomes que deixaram contato neste frame.
    pub collision_exit_contacts: HashMap<String, Vec<String>>,
    pub collision_enter_contact_ids: HashMap<String, Vec<String>>,
    pub collision_stay_contact_ids: HashMap<String, Vec<String>>,
    pub collision_exit_contact_ids: HashMap<String, Vec<String>>,
    pub perf_stats: RuntimePerfStats,
    pub debug_snapshot: RuntimeDebugSnapshot,
    pub debug_text_accumulator: f32,
    pub debug_detail_accumulator: f32,
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
            save_data: SaveData::new(),
            session_state: HashMap::new(),
            script_state: ScriptState::new(),
            lua_vms: HashMap::new(),
            current_runtime_events: Vec::new(),
            pending_runtime_events: Vec::new(),
            collision_contacts: HashMap::new(),
            previous_collision_contacts: HashMap::new(),
            collision_contact_ids: HashMap::new(),
            previous_collision_contact_ids: HashMap::new(),
            collision_enter_contacts: HashMap::new(),
            collision_stay_contacts: HashMap::new(),
            collision_exit_contacts: HashMap::new(),
            collision_enter_contact_ids: HashMap::new(),
            collision_stay_contact_ids: HashMap::new(),
            collision_exit_contact_ids: HashMap::new(),
            perf_stats: RuntimePerfStats::default(),
            debug_snapshot: RuntimeDebugSnapshot::default(),
            debug_text_accumulator: 0.0,
            debug_detail_accumulator: 0.0,
        }
    }

    pub fn start_from_scene(&mut self, scene: &Scene) {
        self.start_from_document(scene, None);
    }

    pub fn start_from_scene_as_new_game(&mut self, scene: &Scene) {
        self.start_new_game_session();
        self.start_from_document(scene, None);
    }

    pub fn start_from_document(&mut self, scene: &Scene, source_path: Option<PathBuf>) {
        self.scene_manager.set_editor_scene(scene.clone());
        self.scene_manager.current_path = source_path;
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

    pub fn start_from_path_as_new_game(&mut self, path: PathBuf) -> Result<(), String> {
        self.start_new_game_session();
        self.start_from_path(path)
    }

    pub fn load_scene_as_new_game(&mut self, path: PathBuf) -> Result<(), String> {
        self.start_from_path_as_new_game(path)
    }

    /// Continua um jogo salvo sem reaproveitar estados temporários da sessão anterior.
    ///
    /// Regras oficiais:
    /// - save_data: mantido/restaurado do arquivo de save
    /// - session_state: limpo
    /// - script_state: limpo
    pub fn continue_from_save(&mut self, project_root: &Path, fallback_scene: &Scene) -> Result<(), String> {
        let Some(data) = SaveData::load_from_project(project_root) else {
            return Err("Nenhum save encontrado para continuar.".to_string());
        };

        self.save_data = data;
        self.clear_session_state();
        self.clear_script_state();
        self.lua_vms.clear();
        self.clear_lua_runtime_events();
        self.game_state.score = 0;
        self.game_state.loading_label = None;

        let saved_scene = self
            .save_data
            .current_scene
            .clone()
            .or_else(|| self.save_data.get_text(SAVE_KEY_CHECKPOINT_SCENE).map(str::to_string));

        if let Some(saved_scene) = saved_scene {
            if let Some(scene_path) = Self::resolve_saved_scene_path(project_root, &saved_scene) {
                self.start_from_path(scene_path)?;
                self.restore_continue_runtime_state();
                return Ok(());
            }
        }

        self.start_from_scene(fallback_scene);
        self.restore_continue_runtime_state();
        Ok(())
    }

    fn resolve_saved_scene_path(project_root: &Path, saved_scene: &str) -> Option<PathBuf> {
        let trimmed = saved_scene.trim();
        if trimmed.is_empty() {
            return None;
        }

        let raw = PathBuf::from(trimmed);
        let normalized = PathBuf::from(trimmed.replace('\\', "/"));
        let mut candidates = Vec::new();

        if raw.is_absolute() {
            candidates.push(raw.clone());
        }
        candidates.push(project_root.join(&raw));
        candidates.push(project_root.join(&normalized));
        candidates.push(project_root.join("assets/scenes").join(&raw));
        candidates.push(project_root.join("assets/scenes").join(&normalized));

        if raw.extension().is_none() && !trimmed.ends_with(".scene") {
            candidates.push(project_root.join("assets/scenes").join(format!("{}.scene.json", trimmed)));
            candidates.push(project_root.join("assets/scenes").join(format!("{}.json", trimmed)));
        }

        candidates.into_iter().find(|path| path.exists())
    }

    pub fn queue_scene_change(&mut self, path: PathBuf) {
        self.game_state.flow = RuntimeGameFlow::Loading;
        self.game_state.loading_label = Some(path.display().to_string());
        self.scene_manager.change_scene(path);
    }

    pub fn queue_scene_change_snapshot(
        &mut self,
        scene: Scene,
        source_path: Option<PathBuf>,
        label: Option<String>,
    ) {
        self.game_state.flow = RuntimeGameFlow::Loading;
        self.game_state.loading_label = Some(
            label
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| scene.name.clone()),
        );
        self.scene_manager.change_scene_snapshot(scene, source_path);
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
        self.clear_session_state();
        self.clear_script_state();
        self.lua_vms.clear();
        self.clear_lua_runtime_events();
        self.clear_collision_tracking();
        self.perf_stats = RuntimePerfStats::default();
        self.debug_snapshot = RuntimeDebugSnapshot::default();
        self.debug_text_accumulator = 0.0;
        self.debug_detail_accumulator = 0.0;
    }

    /// Inicia uma nova sessão de jogo limpando apenas estados temporários.
    /// Não altera `save_data` persistente por padrão.
    pub fn start_new_game_session(&mut self) {
        self.clear_session_state();
        self.clear_script_state();
        self.lua_vms.clear();
        self.clear_lua_runtime_events();
        self.game_state.score = 0;
        self.game_state.loading_label = None;
    }

    pub fn clear_session_state(&mut self) {
        self.session_state.clear();
    }

    pub fn session_get(&self, key: &str) -> Option<&SaveValue> {
        self.session_state.get(key)
    }

    pub fn session_set<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<SaveValue>,
    {
        self.session_state.insert(key.into(), value.into());
    }

    pub fn session_has(&self, key: &str) -> bool {
        self.session_state.contains_key(key)
    }

    pub fn session_remove(&mut self, key: &str) -> Option<SaveValue> {
        self.session_state.remove(key)
    }

    pub fn clear_script_state(&mut self) {
        self.script_state.clear();
    }

    pub fn clear_script_state_for_entity(&mut self, entity_id: &str) {
        let prefix = script_scope_prefix(entity_id);
        self.script_state.retain(|scope_key, _| !scope_key.starts_with(&prefix));
    }

    pub fn clear_script_state_for_scope(&mut self, entity_id: &str, script_path: &str) {
        let scope_key = make_script_state_scope_key(entity_id, script_path);
        self.script_state.remove(&scope_key);
    }

    pub fn script_get(&self, entity_id: &str, script_path: &str, key: &str) -> Option<&SaveValue> {
        let scope_key = make_script_state_scope_key(entity_id, script_path);
        self.script_state.get(&scope_key)?.get(key)
    }

    pub fn script_set<K, V>(&mut self, entity_id: &str, script_path: &str, key: K, value: V)
    where
        K: Into<String>,
        V: Into<SaveValue>,
    {
        let scope_key = make_script_state_scope_key(entity_id, script_path);
        self.script_state
            .entry(scope_key)
            .or_default()
            .insert(key.into(), value.into());
    }

    pub fn script_has(&self, entity_id: &str, script_path: &str, key: &str) -> bool {
        let scope_key = make_script_state_scope_key(entity_id, script_path);
        self.script_state
            .get(&scope_key)
            .map(|state| state.contains_key(key))
            .unwrap_or(false)
    }

    pub fn script_remove(&mut self, entity_id: &str, script_path: &str, key: &str) -> Option<SaveValue> {
        let scope_key = make_script_state_scope_key(entity_id, script_path);
        let removed = self
            .script_state
            .get_mut(&scope_key)
            .and_then(|state| state.remove(key));

        let should_prune = self
            .script_state
            .get(&scope_key)
            .map(|state| state.is_empty())
            .unwrap_or(false);

        if should_prune {
            self.script_state.remove(&scope_key);
        }

        removed
    }

    fn clear_lua_runtime_events(&mut self) {
        self.current_runtime_events.clear();
        self.pending_runtime_events.clear();
    }

    fn clear_collision_tracking(&mut self) {
        self.collision_contacts.clear();
        self.previous_collision_contacts.clear();
        self.collision_contact_ids.clear();
        self.previous_collision_contact_ids.clear();
        self.collision_enter_contacts.clear();
        self.collision_stay_contacts.clear();
        self.collision_exit_contacts.clear();
        self.collision_enter_contact_ids.clear();
        self.collision_stay_contact_ids.clear();
        self.collision_exit_contact_ids.clear();
    }

    fn rebuild_collision_events(&mut self) {
        self.collision_enter_contacts.clear();
        self.collision_stay_contacts.clear();
        self.collision_exit_contacts.clear();
        self.collision_enter_contact_ids.clear();
        self.collision_stay_contact_ids.clear();
        self.collision_exit_contact_ids.clear();

        let mut entity_ids: HashSet<String> = HashSet::new();
        entity_ids.extend(self.collision_contacts.keys().cloned());
        entity_ids.extend(self.previous_collision_contacts.keys().cloned());
        entity_ids.extend(self.collision_contact_ids.keys().cloned());
        entity_ids.extend(self.previous_collision_contact_ids.keys().cloned());

        for entity_id in entity_ids {
            let current_names: HashSet<String> = self.collision_contacts.get(&entity_id).cloned().unwrap_or_default().into_iter().collect();
            let previous_names: HashSet<String> = self.previous_collision_contacts.get(&entity_id).cloned().unwrap_or_default().into_iter().collect();

            let mut enter_names: Vec<String> = current_names.difference(&previous_names).cloned().collect();
            let mut stay_names: Vec<String> = current_names.intersection(&previous_names).cloned().collect();
            let mut exit_names: Vec<String> = previous_names.difference(&current_names).cloned().collect();
            enter_names.sort();
            stay_names.sort();
            exit_names.sort();
            if !enter_names.is_empty() { self.collision_enter_contacts.insert(entity_id.clone(), enter_names); }
            if !stay_names.is_empty() { self.collision_stay_contacts.insert(entity_id.clone(), stay_names); }
            if !exit_names.is_empty() { self.collision_exit_contacts.insert(entity_id.clone(), exit_names); }

            let current_ids: HashSet<String> = self.collision_contact_ids.get(&entity_id).cloned().unwrap_or_default().into_iter().collect();
            let previous_ids: HashSet<String> = self.previous_collision_contact_ids.get(&entity_id).cloned().unwrap_or_default().into_iter().collect();

            let mut enter_ids: Vec<String> = current_ids.difference(&previous_ids).cloned().collect();
            let mut stay_ids: Vec<String> = current_ids.intersection(&previous_ids).cloned().collect();
            let mut exit_ids: Vec<String> = previous_ids.difference(&current_ids).cloned().collect();
            enter_ids.sort();
            stay_ids.sort();
            exit_ids.sort();
            if !enter_ids.is_empty() { self.collision_enter_contact_ids.insert(entity_id.clone(), enter_ids); }
            if !stay_ids.is_empty() { self.collision_stay_contact_ids.insert(entity_id.clone(), stay_ids); }
            if !exit_ids.is_empty() { self.collision_exit_contact_ids.insert(entity_id.clone(), exit_ids); }
        }
    }

    pub fn stop_audio_by_name(&mut self, name: &str) -> usize {
        let stopped = self.audio_runtime.stop_by_name(name);
        if stopped > 0 {
            let lowered = name.trim().to_ascii_lowercase();
            self.started_audio.retain(|key| !key.to_ascii_lowercase().contains(&lowered));
        }
        stopped
    }

    pub fn sync_with_mode(
        &mut self,
        mode: RuntimePlayState,
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
            RuntimePlayState::Edit => {
                if self.active_scene.is_some() {
                    self.stop();
                }
                self.window_open = false;
                self.game_state.flow = RuntimeGameFlow::Editing;
            }
            RuntimePlayState::Playing => {
                if self.active_scene.is_none() {
                    self.start_from_scene(source_scene);
                }
                self.window_open = true;
                self.game_state.flow = RuntimeGameFlow::Playing;
                self.update_frame(project_root, ground_y);
            }
            RuntimePlayState::Paused => {
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
        let entity_id = entity_id.into();
        if self.pending_destroys.iter().any(|request| request.entity_id == entity_id) {
            return;
        }
        self.pending_destroys.push(PendingDestroyRequest { entity_id });
    }

    pub fn queue_destroy_last_spawned(&mut self) -> bool {
        let Some(id) = self.last_spawned_entity_id.clone() else {
            return false;
        };
        self.queue_destroy(id);
        true
    }


    pub fn update_debug_snapshot(&mut self, scene_name: &str, entity_count: usize, script_count: usize) {
        self.debug_text_accumulator += self.delta_time;
        self.debug_detail_accumulator += self.delta_time;

        if self.debug_text_accumulator >= 0.12 || self.frame_count <= 1 {
            self.debug_snapshot.hud_summary = format!(
                "▶ Runtime Preview | 📌 {} | ⏱ {:.2}s | FPS ~ {:.0} | Status: {} | Flow: {:?} | Etapa: {:?}",
                scene_name,
                self.elapsed_time,
                self.estimated_fps(),
                match self.game_state.flow {
                    RuntimeGameFlow::Playing => "Executando",
                    RuntimeGameFlow::Paused => "Pausado",
                    RuntimeGameFlow::Editing => "Edição",
                    RuntimeGameFlow::GameOver => "Game Over",
                    RuntimeGameFlow::Loading => "Loading",
                },
                self.game_state.flow,
                self.last_stage,
            );
            self.debug_snapshot.controls_hint =
                "WASD = player_controller | Setas = câmera | Q/E ou scroll = zoom | ESC = pause | Novo jogo limpa session/state"
                    .to_string();
            self.debug_text_accumulator = 0.0;
        }

        if self.debug_detail_accumulator >= 0.20 || self.frame_count <= 1 {
            self.debug_snapshot.overlay_left = format!(
                "Entidades: {}  •  Scripts: {}  •  Delta: {:.3} ms",
                entity_count,
                script_count,
                self.delta_time * 1000.0,
            );
            self.debug_snapshot.overlay_right = format!(
                "HUD/UI  •  Cena: {}  •  Score: {}  •  Flow: {:?}",
                scene_name,
                self.game_state.score,
                self.game_state.flow,
            );
            self.debug_snapshot.perf_summary = format!(
                "Perf(ms) — scripts: {:.2} • colisão: {:.2} • HUD: {:.2}",
                self.perf_stats.scripts_ms,
                self.perf_stats.collisions_ms,
                self.perf_stats.hud_ms,
            );
            self.debug_detail_accumulator = 0.0;
        }
    }

    pub fn estimated_fps(&self) -> f32 {
        if self.delta_time <= f32::EPSILON {
            0.0
        } else {
            1.0 / self.delta_time
        }
    }

    // ── save / load ──────────────────────────────────────────

    /// Salva `save_data` em `<project_root>/save/save.json`.
    /// Grava também a cena atual para permitir o fluxo oficial de "Continuar".
    pub fn save_game(&mut self, project_root: &Path) -> Result<(), String> {
        if let Some(path) = &self.scene_manager.current_path {
            let persisted_path = path
                .strip_prefix(project_root)
                .unwrap_or(path.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            self.save_data.current_scene = Some(persisted_path.clone());
            self.save_data.set_text(SAVE_KEY_CHECKPOINT_SCENE, persisted_path);
        } else if let Some(scene) = &self.active_scene {
            self.save_data.current_scene = Some(scene.name.clone());
            self.save_data.set_text(SAVE_KEY_CHECKPOINT_SCENE, scene.name.clone());
        }

        if let Some(scene) = &self.active_scene {
            if let Some((player_id, player_name, x, y, vx, vy)) = capture_continue_player_snapshot(scene) {
                self.save_data.set_text(SAVE_KEY_PLAYER_ID, player_id);
                self.save_data.set_text(SAVE_KEY_PLAYER_NAME, player_name);
                self.save_data.set_float(SAVE_KEY_PLAYER_X, x as f64);
                self.save_data.set_float(SAVE_KEY_PLAYER_Y, y as f64);
                self.save_data.set_float(SAVE_KEY_PLAYER_VX, vx as f64);
                self.save_data.set_float(SAVE_KEY_PLAYER_VY, vy as f64);
            }
        }

        self.save_data.save_to_project(project_root)
    }

    /// Carrega `<project_root>/save/save.json` para `save_data`.
    /// Retorna `false` se não existir arquivo (primeira sessão).
    pub fn load_game(&mut self, project_root: &Path) -> bool {
        if let Some(data) = SaveData::load_from_project(project_root) {
            self.save_data = data;
            true
        } else {
            false
        }
    }

    /// Apaga o save e reinicia `save_data`.
    pub fn delete_save(&mut self, project_root: &Path) -> bool {
        self.save_data = SaveData::new();
        SaveData::delete_save(project_root)
    }

    fn restore_continue_runtime_state(&mut self) {
        let Some(scene) = self.active_scene.as_mut() else {
            return;
        };

        let saved_player_id = self.save_data.get_text(SAVE_KEY_PLAYER_ID).map(str::to_string);
        let saved_player_name = self.save_data.get_text(SAVE_KEY_PLAYER_NAME).map(str::to_string);
        let saved_x = self.save_data.get_float(SAVE_KEY_PLAYER_X).map(|v| v as f32);
        let saved_y = self.save_data.get_float(SAVE_KEY_PLAYER_Y).map(|v| v as f32);
        let saved_vx = self.save_data.get_float(SAVE_KEY_PLAYER_VX).map(|v| v as f32);
        let saved_vy = self.save_data.get_float(SAVE_KEY_PLAYER_VY).map(|v| v as f32);

        let Some(player) = find_continue_player_mut(scene, saved_player_id.as_deref(), saved_player_name.as_deref()) else {
            return;
        };

        if let Some(transform) = player.transform_mut() {
            if let Some(x) = saved_x { transform.x = x; }
            if let Some(y) = saved_y { transform.y = y; }
        }

        if let Some(velocity) = player.velocity_mut() {
            if let Some(vx) = saved_vx { velocity.x = vx; }
            if let Some(vy) = saved_vy { velocity.y = vy; }
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
        self.clear_script_state();
        self.lua_vms.clear();
        self.clear_lua_runtime_events();
        self.clear_collision_tracking();
        self.perf_stats = RuntimePerfStats::default();
        self.debug_snapshot = RuntimeDebugSnapshot::default();
        self.debug_text_accumulator = 0.0;
        self.debug_detail_accumulator = 0.0;
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
        self.current_runtime_events = std::mem::take(&mut self.pending_runtime_events);

        let player_input = systems::input_system::player_axis(&self.input);

        let mut runtime_command = None;
        let mut scene_snapshot = None;
        let mut finalize_frame = false;

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
            let elapsed = self.elapsed_time;
            let input_snap = self.input.clone();
            let incoming_lua_events = self.current_runtime_events.clone();
            self.previous_collision_contacts = self.collision_contacts.clone();
            self.previous_collision_contact_ids = self.collision_contact_ids.clone();
            self.collision_contacts.clear();
            self.collision_contact_ids.clear();
            let scene_label = Some(scene.name.as_str());
            let scripts_start = Instant::now();
            runtime_command = systems::update_entities_runtime(
                &mut scene.entities,
                dt,
                elapsed,
                project_root,
                scene_label,
                &mut self.started_scripts,
                &mut camera_follow_target,
                ground_y,
                &mut self.save_data,
                &mut self.session_state,
                &mut self.script_state,
                &input_snap,
                &mut self.lua_vms,
                &mut self.collision_contacts,
                &mut self.collision_contact_ids,
                &self.previous_collision_contacts,
                &self.previous_collision_contact_ids,
                &mut self.pending_destroys,
                &incoming_lua_events,
                &mut self.pending_runtime_events,
            );
            self.perf_stats.scripts_ms = scripts_start.elapsed().as_secs_f32() * 1000.0;
            self.perf_stats.collisions_ms = self.perf_stats.scripts_ms;

            self.last_stage = RuntimeFrameStage::UpdateCamera;
            if let Some((x, y)) = camera_follow_target {
                camera::set_main_camera_position(&mut scene.entities, x, y);
            }

            scene_snapshot = Some(scene.clone());
            finalize_frame = true;
        }

        if finalize_frame {
            self.rebuild_collision_events();

            self.last_stage = RuntimeFrameStage::FinalizeFrame;
            let pending_destroys = std::mem::take(&mut self.pending_destroys);
            let pending_spawns = std::mem::take(&mut self.pending_spawns);
            if let Some(mut scene) = self.active_scene.take() {
                self.apply_pending_entity_commands(
                    &mut scene,
                    pending_destroys,
                    pending_spawns,
                );
                self.scene_manager.current_scene = Some(scene.clone());
                self.active_scene = Some(scene);
            } else if let Some(scene) = scene_snapshot {
                self.scene_manager.current_scene = Some(scene);
            }

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
        &mut self,
        scene: &mut Scene,
        pending_destroys: Vec<PendingDestroyRequest>,
        pending_spawns: Vec<PendingSpawnRequest>,
    ) {
        for entity_id in pending_destroys.into_iter().map(|request| request.entity_id) {
            if scene.remove_entity_by_id(&entity_id) {
                self.cleanup_destroyed_entity_runtime_data(&entity_id);
                if self.last_spawned_entity_id.as_deref() == Some(entity_id.as_str()) {
                    self.last_spawned_entity_id = None;
                }
            }
        }

        for request in pending_spawns {
            let entity = Self::build_spawn_entity(&request.template, request.x, request.y);
            self.last_spawned_entity_id = Some(entity.id.clone());
            scene.add_entity(entity);
        }
    }

    fn cleanup_destroyed_entity_runtime_data(&mut self, entity_id: &str) {
        let script_prefix = script_scope_prefix(entity_id);
        let started_prefix = format!("lua::{}::", entity_id);
        let vm_prefix = format!("vm::{}::", entity_id);
        self.script_state.retain(|scope_key, _| !scope_key.starts_with(&script_prefix));
        self.started_scripts.retain(|key| !key.starts_with(&started_prefix));
        self.started_audio.retain(|key| !key.contains(entity_id));
        self.lua_vms.retain(|key: &String, _| !key.starts_with(&vm_prefix));
        self.purge_entity_from_collision_tracking(entity_id);
    }


    fn purge_entity_from_collision_tracking(&mut self, entity_id: &str) {
        let removed_name = self.find_entity_name_in_collision_tracking(entity_id);
        self.collision_contacts.remove(entity_id);
        self.previous_collision_contacts.remove(entity_id);
        self.collision_contact_ids.remove(entity_id);
        self.previous_collision_contact_ids.remove(entity_id);
        self.collision_enter_contacts.remove(entity_id);
        self.collision_stay_contacts.remove(entity_id);
        self.collision_exit_contacts.remove(entity_id);
        self.collision_enter_contact_ids.remove(entity_id);
        self.collision_stay_contact_ids.remove(entity_id);
        self.collision_exit_contact_ids.remove(entity_id);
        if let Some(name) = removed_name.as_ref() {
            for contacts in self.collision_contacts.values_mut() { contacts.retain(|existing| existing != name); }
            for contacts in self.previous_collision_contacts.values_mut() { contacts.retain(|existing| existing != name); }
            for contacts in self.collision_enter_contacts.values_mut() { contacts.retain(|existing| existing != name); }
            for contacts in self.collision_stay_contacts.values_mut() { contacts.retain(|existing| existing != name); }
            for contacts in self.collision_exit_contacts.values_mut() { contacts.retain(|existing| existing != name); }
        }
        for contacts in self.collision_contact_ids.values_mut() { contacts.retain(|id| id != entity_id); }
        for contacts in self.previous_collision_contact_ids.values_mut() { contacts.retain(|id| id != entity_id); }
        for contacts in self.collision_enter_contact_ids.values_mut() { contacts.retain(|id| id != entity_id); }
        for contacts in self.collision_stay_contact_ids.values_mut() { contacts.retain(|id| id != entity_id); }
        for contacts in self.collision_exit_contact_ids.values_mut() { contacts.retain(|id| id != entity_id); }
    }

    fn find_entity_name_in_collision_tracking(&self, entity_id: &str) -> Option<String> {
        self.active_scene.as_ref().and_then(|scene| scene.find_entity(entity_id)).map(|entity| {
            if entity.name.trim().is_empty() { entity.id.clone() } else { entity.name.clone() }
        })
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
            collision_enabled: true,
            layer: 0,
            mask: 0,
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



fn capture_continue_player_snapshot(scene: &Scene) -> Option<(String, String, f32, f32, f32, f32)> {
    let player = find_continue_player(scene, None, None)?;
    let transform = player.transform()?;
    let velocity = player.velocity().cloned().unwrap_or_default();
    Some((
        player.id.clone(),
        player.name.clone(),
        transform.x,
        transform.y,
        velocity.x,
        velocity.y,
    ))
}

fn find_continue_player<'a>(scene: &'a Scene, preferred_id: Option<&str>, preferred_name: Option<&str>) -> Option<&'a Entity> {
    if let Some(id) = preferred_id {
        if let Some(entity) = scene.find_entity(id) {
            return Some(entity);
        }
    }

    let mut match_id: Option<String> = None;
    if let Some(name) = preferred_name {
        scene.visit_entities(|entity| {
            if match_id.is_none() && entity.name == name {
                match_id = Some(entity.id.clone());
            }
        });
        if let Some(id) = match_id.as_deref() {
            return scene.find_entity(id);
        }
    }

    scene.visit_entities(|entity| {
        if match_id.is_none() && entity.name.eq_ignore_ascii_case("Player") {
            match_id = Some(entity.id.clone());
        }
    });

    match_id.as_deref().and_then(|id| scene.find_entity(id))
}

fn find_continue_player_mut<'a>(scene: &'a mut Scene, preferred_id: Option<&str>, preferred_name: Option<&str>) -> Option<&'a mut Entity> {
    if let Some(id) = preferred_id {
        if scene.find_entity(id).is_some() {
            return scene.find_entity_mut(id);
        }
    }

    let mut match_id: Option<String> = None;
    if let Some(name) = preferred_name {
        scene.visit_entities(|entity| {
            if match_id.is_none() && entity.name == name {
                match_id = Some(entity.id.clone());
            }
        });
    }
    if match_id.is_none() {
        scene.visit_entities(|entity| {
            if match_id.is_none() && entity.name.eq_ignore_ascii_case("Player") {
                match_id = Some(entity.id.clone());
            }
        });
    }
    match_id.and_then(|id| scene.find_entity_mut(&id))
}

