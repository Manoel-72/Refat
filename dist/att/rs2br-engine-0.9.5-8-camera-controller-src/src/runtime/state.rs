use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{
    core::{
        component::{Component, Velocity},
        entity::Entity,
        scene::Scene,
    },
    runtime::context::RuntimePlayState,
    runtime::{
        camera::CameraController,
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
/// Runtime alvo: 120 FPS (intervalo mínimo ~8.33ms entre updates).
const MIN_FRAME_DT: f32 = 1.0 / 120.0;

#[derive(Debug, Clone)]
pub enum PendingSpawnKind {
    Template(String),
    PrefabPath(String),
}

#[derive(Debug, Clone)]
pub struct PendingSpawnRequest {
    pub kind: PendingSpawnKind,
    pub x: f32,
    pub y: f32,
    pub velocity: Option<(f32, f32)>,
    pub tags: Vec<String>,
    pub hp: Option<f32>,
    pub anim: Option<String>,
    /// Sequência do pedido de spawn vinda do Lua. Quando presente,
    /// o runtime devolve um evento spawn_result:<seq> com o id criado.
    pub request_seq: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RuntimeParticleEmitter {
    pub x: f32,
    pub y: f32,
    pub rate: f32,
    pub particle_life: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub color: [f32; 4],
    pub scale: f32,
    pub duration: f32,
    pub accumulator: f32,
}

#[derive(Debug, Clone)]
pub struct RuntimeParticle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
    pub color: [f32; 4],
    pub scale: f32,
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
    /// Snapshot simples dos botões de gamepad ativos no frame.
    /// O backend atual do runtime ainda não alimenta isso; a API Lua
    /// já fica pronta para evolução sem quebrar scripts.
    pub gamepad_buttons: HashSet<u32>,
    /// Snapshot simples dos eixos de gamepad.
    /// O backend atual do runtime ainda não alimenta isso; por enquanto
    /// os scripts recebem 0.0 quando o host não preencher esse mapa.
    pub gamepad_axes: HashMap<u32, f32>,
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

    pub fn is_gamepad_button_held(&self, button: u32) -> bool {
        self.gamepad_buttons.contains(&button)
    }

    pub fn gamepad_axis_value(&self, axis: u32) -> f32 {
        self.gamepad_axes.get(&axis).copied().unwrap_or(0.0)
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
    /// Ring buffer de deltas dos últimos 30 frames para FPS suavizado.
    fps_samples: Vec<f32>,
    fps_sample_idx: usize,
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
    pub particles: Vec<RuntimeParticle>,
    pub pending_particles: Vec<RuntimeParticle>,
    pub emitters: Vec<RuntimeParticleEmitter>,
    pub camera_shake_time: f32,
    pub camera_shake_intensity: f32,
    pub camera_zoom_override: Option<f32>,
    pub camera_controller: CameraController,
    /// Estado persistente do jogo (save/load em save/save.json).
    pub save_data: SaveData,
    /// Estado temporário da sessão atual (não persistido em arquivo por padrão).
    pub session_state: HashMap<String, SaveValue>,
    /// Estado temporário por entidade+script (memória de execução, não persistida).
    /// Chave: "entity_id::script_path"
    pub script_state: ScriptState,
    /// Cache de VMs Lua: chave = "entity_id::script_path", valor = (VM, mtime do arquivo, último frame em que o mtime foi consultado).
    /// Evita criar uma Lua::new() por frame — criada uma vez, reutilizada.
    pub lua_vms: HashMap<String, (mlua::Lua, std::time::SystemTime, u64)>,
    /// Cache simples de cenas para trocas de cena frequentes sem reler disco toda vez.
    pub scene_cache: HashMap<PathBuf, Scene>,
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
    /// Contatos de trigger do frame atual: entity_id -> nomes.
    pub trigger_contacts: HashMap<String, Vec<String>>,
    /// Contatos de trigger do frame anterior: entity_id -> nomes.
    pub previous_trigger_contacts: HashMap<String, Vec<String>>,
    pub trigger_contact_ids: HashMap<String, Vec<String>>,
    pub previous_trigger_contact_ids: HashMap<String, Vec<String>>,
    pub trigger_enter_contacts: HashMap<String, Vec<String>>,
    pub trigger_stay_contacts: HashMap<String, Vec<String>>,
    pub trigger_exit_contacts: HashMap<String, Vec<String>>,
    pub trigger_enter_contact_ids: HashMap<String, Vec<String>>,
    pub trigger_stay_contact_ids: HashMap<String, Vec<String>>,
    pub trigger_exit_contact_ids: HashMap<String, Vec<String>>,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            active_scene: None,
            elapsed_time: 0.0,
            delta_time: 0.0,
            frame_count: 0,
            last_frame_at: None,
            fps_samples: vec![1.0 / 60.0; 30],
            fps_sample_idx: 0,
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
            particles: Vec::new(),
            pending_particles: Vec::new(),
            emitters: Vec::new(),
            camera_shake_time: 0.0,
            camera_shake_intensity: 0.0,
            camera_zoom_override: None,
            camera_controller: CameraController::default(),
            save_data: SaveData::new(),
            session_state: HashMap::new(),
            script_state: ScriptState::new(),
            lua_vms: HashMap::new(),
            scene_cache: HashMap::new(),
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
            trigger_contacts: HashMap::new(),
            previous_trigger_contacts: HashMap::new(),
            trigger_contact_ids: HashMap::new(),
            previous_trigger_contact_ids: HashMap::new(),
            trigger_enter_contacts: HashMap::new(),
            trigger_stay_contacts: HashMap::new(),
            trigger_exit_contacts: HashMap::new(),
            trigger_enter_contact_ids: HashMap::new(),
            trigger_stay_contact_ids: HashMap::new(),
            trigger_exit_contact_ids: HashMap::new(),
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
    pub fn continue_from_save(
        &mut self,
        project_root: &Path,
        fallback_scene: &Scene,
    ) -> Result<(), String> {
        let Some(data) = SaveData::load_from_project(project_root) else {
            return Err("Nenhum save encontrado para continuar.".to_string());
        };

        self.save_data = data;
        self.clear_session_state();
        self.clear_script_state();
        self.lua_vms.clear();
        self.scene_cache.clear();
        self.clear_lua_runtime_events();
        self.game_state.score = 0;
        self.game_state.loading_label = None;

        let saved_scene = self.save_data.current_scene.clone().or_else(|| {
            self.save_data
                .get_text(SAVE_KEY_CHECKPOINT_SCENE)
                .map(str::to_string)
        });

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
            candidates.push(
                project_root
                    .join("assets/scenes")
                    .join(format!("{}.scene.json", trimmed)),
            );
            candidates.push(
                project_root
                    .join("assets/scenes")
                    .join(format!("{}.json", trimmed)),
            );
        }

        candidates.into_iter().find(|path| path.exists())
    }

    pub fn queue_scene_change(&mut self, path: PathBuf) {
        self.game_state.flow = RuntimeGameFlow::Loading;
        self.game_state.loading_label = Some(path.display().to_string());
        self.scene_manager.change_scene(path);
    }

    pub fn queue_scene_change_cached(&mut self, path: PathBuf) {
        if let Some(scene) = self.scene_cache.get(&path).cloned() {
            let label = scene.name.clone();
            self.queue_scene_change_snapshot(scene, Some(path), Some(label));
            return;
        }

        match crate::serialization::scene_serializer::try_load_scene_from_path(&path) {
            Ok(scene) => {
                let label = scene.name.clone();
                self.scene_cache.insert(path.clone(), scene.clone());
                self.queue_scene_change_snapshot(scene, Some(path), Some(label));
            }
            Err(_) => {
                self.queue_scene_change(path);
            }
        }
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
        self.fps_samples = vec![1.0 / 60.0; 30];
        self.fps_sample_idx = 0;
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
        self.scene_cache.clear();
        self.clear_lua_runtime_events();
        self.clear_collision_tracking();
    }

    /// Inicia uma nova sessão de jogo limpando apenas estados temporários.
    /// Não altera `save_data` persistente por padrão.
    pub fn start_new_game_session(&mut self) {
        self.clear_session_state();
        self.clear_script_state();
        self.lua_vms.clear();
        self.scene_cache.clear();
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
        self.script_state
            .retain(|scope_key, _| !scope_key.starts_with(&prefix));
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

    pub fn script_remove(
        &mut self,
        entity_id: &str,
        script_path: &str,
        key: &str,
    ) -> Option<SaveValue> {
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
        self.trigger_enter_contacts.clear();
        self.trigger_stay_contacts.clear();
        self.trigger_exit_contacts.clear();
        self.trigger_enter_contact_ids.clear();
        self.trigger_stay_contact_ids.clear();
        self.trigger_exit_contact_ids.clear();

        fn diff_names_and_ids(
            current_names: &[String],
            previous_names: &[String],
            current_ids: &[String],
            previous_ids: &[String],
        ) -> (
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
        ) {
            let mut enter_names = Vec::new();
            let mut stay_names = Vec::new();
            let mut exit_names = Vec::new();
            let mut enter_ids = Vec::new();
            let mut stay_ids = Vec::new();
            let mut exit_ids = Vec::new();

            for (index, current_id) in current_ids.iter().enumerate() {
                let current_name = current_names
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| current_id.clone());
                if previous_ids.iter().any(|id| id == current_id) {
                    stay_ids.push(current_id.clone());
                    stay_names.push(current_name);
                } else {
                    enter_ids.push(current_id.clone());
                    enter_names.push(current_name);
                }
            }

            for previous_id in previous_ids {
                if current_ids.iter().any(|id| id == previous_id) {
                    continue;
                }
                let previous_name = previous_ids
                    .iter()
                    .position(|id| id == previous_id)
                    .and_then(|index| previous_names.get(index))
                    .cloned()
                    .unwrap_or_else(|| previous_id.clone());
                exit_ids.push(previous_id.clone());
                exit_names.push(previous_name);
            }

            (
                enter_names,
                stay_names,
                exit_names,
                enter_ids,
                stay_ids,
                exit_ids,
            )
        }

        fn diff_maps(
            current_names_map: &HashMap<String, Vec<String>>,
            previous_names_map: &HashMap<String, Vec<String>>,
            current_ids_map: &HashMap<String, Vec<String>>,
            previous_ids_map: &HashMap<String, Vec<String>>,
            enter_names_out: &mut HashMap<String, Vec<String>>,
            stay_names_out: &mut HashMap<String, Vec<String>>,
            exit_names_out: &mut HashMap<String, Vec<String>>,
            enter_ids_out: &mut HashMap<String, Vec<String>>,
            stay_ids_out: &mut HashMap<String, Vec<String>>,
            exit_ids_out: &mut HashMap<String, Vec<String>>,
        ) {
            for (entity_id, current_ids) in current_ids_map {
                let current_names = current_names_map
                    .get(entity_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let previous_names = previous_names_map
                    .get(entity_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let previous_ids = previous_ids_map
                    .get(entity_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let (enter_names, stay_names, exit_names, enter_ids, stay_ids, exit_ids) =
                    diff_names_and_ids(current_names, previous_names, current_ids, previous_ids);
                if !enter_names.is_empty() {
                    enter_names_out.insert(entity_id.clone(), enter_names);
                }
                if !stay_names.is_empty() {
                    stay_names_out.insert(entity_id.clone(), stay_names);
                }
                if !exit_names.is_empty() {
                    exit_names_out.insert(entity_id.clone(), exit_names);
                }
                if !enter_ids.is_empty() {
                    enter_ids_out.insert(entity_id.clone(), enter_ids);
                }
                if !stay_ids.is_empty() {
                    stay_ids_out.insert(entity_id.clone(), stay_ids);
                }
                if !exit_ids.is_empty() {
                    exit_ids_out.insert(entity_id.clone(), exit_ids);
                }
            }

            for (entity_id, previous_ids) in previous_ids_map {
                if current_ids_map.contains_key(entity_id) {
                    continue;
                }
                let previous_names = previous_names_map
                    .get(entity_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let (enter_names, stay_names, exit_names, enter_ids, stay_ids, exit_ids) =
                    diff_names_and_ids(&[], previous_names, &[], previous_ids);
                if !enter_names.is_empty() {
                    enter_names_out.insert(entity_id.clone(), enter_names);
                }
                if !stay_names.is_empty() {
                    stay_names_out.insert(entity_id.clone(), stay_names);
                }
                if !exit_names.is_empty() {
                    exit_names_out.insert(entity_id.clone(), exit_names);
                }
                if !enter_ids.is_empty() {
                    enter_ids_out.insert(entity_id.clone(), enter_ids);
                }
                if !stay_ids.is_empty() {
                    stay_ids_out.insert(entity_id.clone(), stay_ids);
                }
                if !exit_ids.is_empty() {
                    exit_ids_out.insert(entity_id.clone(), exit_ids);
                }
            }
        }

        diff_maps(
            &self.collision_contacts,
            &self.previous_collision_contacts,
            &self.collision_contact_ids,
            &self.previous_collision_contact_ids,
            &mut self.collision_enter_contacts,
            &mut self.collision_stay_contacts,
            &mut self.collision_exit_contacts,
            &mut self.collision_enter_contact_ids,
            &mut self.collision_stay_contact_ids,
            &mut self.collision_exit_contact_ids,
        );

        diff_maps(
            &self.trigger_contacts,
            &self.previous_trigger_contacts,
            &self.trigger_contact_ids,
            &self.previous_trigger_contact_ids,
            &mut self.trigger_enter_contacts,
            &mut self.trigger_stay_contacts,
            &mut self.trigger_exit_contacts,
            &mut self.trigger_enter_contact_ids,
            &mut self.trigger_stay_contact_ids,
            &mut self.trigger_exit_contact_ids,
        );

        fn emit_pair_events(
            prefix: &str,
            map: &HashMap<String, Vec<String>>,
            pending: &mut Vec<RuntimeEvent>,
        ) {
            for (entity_id, names) in map {
                for other in names {
                    pending.push(RuntimeEvent {
                        name: format!("{}:{}:{}", prefix, entity_id, other),
                        data: None,
                    });
                }
            }
        }

        emit_pair_events(
            "collision_enter",
            &self.collision_enter_contacts,
            &mut self.pending_runtime_events,
        );
        emit_pair_events(
            "collision_stay",
            &self.collision_stay_contacts,
            &mut self.pending_runtime_events,
        );
        emit_pair_events(
            "collision_exit",
            &self.collision_exit_contacts,
            &mut self.pending_runtime_events,
        );
        emit_pair_events(
            "trigger_enter",
            &self.trigger_enter_contacts,
            &mut self.pending_runtime_events,
        );
        emit_pair_events(
            "trigger_stay",
            &self.trigger_stay_contacts,
            &mut self.pending_runtime_events,
        );
        emit_pair_events(
            "trigger_exit",
            &self.trigger_exit_contacts,
            &mut self.pending_runtime_events,
        );
    }

    fn update_emitters(&mut self, dt: f32) {
        if self.emitters.is_empty() {
            return;
        }
        let mut spawned = Vec::new();
        for emitter in &mut self.emitters {
            emitter.duration = (emitter.duration - dt).max(0.0);
            emitter.accumulator += emitter.rate.max(0.0) * dt;
            let count = emitter.accumulator.floor() as i32;
            if count > 0 {
                emitter.accumulator -= count as f32;
            }
            let speed_mid = (emitter.speed_min + emitter.speed_max) * 0.5;
            for idx in 0..count.max(0) {
                let dir = if idx % 2 == 0 { -1.0 } else { 1.0 };
                let spread = 0.35 + (idx as f32 * 0.11).sin().abs() * 0.65;
                spawned.push(RuntimeParticle {
                    x: emitter.x,
                    y: emitter.y,
                    vx: dir * speed_mid * spread,
                    vy: -emitter.speed_max.max(1.0) * (0.6 + spread * 0.4),
                    life: emitter.particle_life,
                    max_life: emitter.particle_life,
                    color: emitter.color,
                    scale: emitter.scale,
                });
            }
        }
        self.emitters
            .retain(|emitter| emitter.duration > 0.0 && emitter.rate > 0.0);
        if !spawned.is_empty() {
            self.particles.extend(spawned);
        }
    }

    fn update_particles(&mut self, dt: f32) {
        if self.particles.is_empty() {
            return;
        }

        for particle in &mut self.particles {
            particle.x += particle.vx * dt;
            particle.y += particle.vy * dt;
            particle.life -= dt;
        }

        self.particles
            .retain(|particle| particle.life > 0.0 && particle.scale > 0.0);
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
        self.particles.clear();
        self.pending_particles.clear();
        self.emitters.clear();
        self.camera_shake_time = 0.0;
        self.camera_shake_intensity = 0.0;
        self.camera_zoom_override = None;
        self.camera_controller = CameraController::default();
        self.clear_script_state();
        self.lua_vms.clear();
        self.scene_cache.clear();
        self.clear_lua_runtime_events();
        self.clear_collision_tracking();
    }

    fn restore_continue_runtime_state(&mut self) {
        let Some(scene) = self.active_scene.as_mut() else {
            return;
        };

        let saved_player_id = self
            .save_data
            .get_text(SAVE_KEY_PLAYER_ID)
            .map(str::to_string);
        let saved_player_name = self
            .save_data
            .get_text(SAVE_KEY_PLAYER_NAME)
            .map(str::to_string);
        let saved_x = self
            .save_data
            .get_float(SAVE_KEY_PLAYER_X)
            .map(|v| v as f32);
        let saved_y = self
            .save_data
            .get_float(SAVE_KEY_PLAYER_Y)
            .map(|v| v as f32);
        let saved_vx = self
            .save_data
            .get_float(SAVE_KEY_PLAYER_VX)
            .map(|v| v as f32);
        let saved_vy = self
            .save_data
            .get_float(SAVE_KEY_PLAYER_VY)
            .map(|v| v as f32);

        let Some(player) = find_continue_player_mut(
            scene,
            saved_player_id.as_deref(),
            saved_player_name.as_deref(),
        ) else {
            return;
        };

        if let Some(transform) = player.transform_mut() {
            if let Some(x) = saved_x {
                transform.x = x;
            }
            if let Some(y) = saved_y {
                transform.y = y;
            }
        }

        if let Some(velocity) = player.velocity_mut() {
            if let Some(vx) = saved_vx {
                velocity.x = vx;
            }
            if let Some(vy) = saved_vy {
                velocity.y = vy;
            }
        }
    }

    pub fn sync_with_mode(
        &mut self,
        mode: &RuntimePlayState,
        source_scene: &Scene,
        project_root: &Path,
        ground_y: f32,
    ) {
        if matches!(self.scene_manager.apply_pending_change(), Ok(true)) {
            self.active_scene = self.scene_manager.current_scene.clone();
            if let (Some(path), Some(scene)) = (
                self.scene_manager.current_path.clone(),
                self.scene_manager.current_scene.clone(),
            ) {
                self.scene_cache.insert(path, scene);
            }
            self.reset_timing_state();
            self.game_state.flow = RuntimeGameFlow::Playing;
            self.game_state.loading_label = None;
        }

        match *mode {
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

    pub fn estimated_fps(&self) -> f32 {
        // Média dos últimos 30 frames — elimina picos de um único frame lento.
        let sum: f32 = self.fps_samples.iter().sum();
        if sum <= f32::EPSILON {
            return 0.0;
        }
        let avg_dt = sum / self.fps_samples.len() as f32;
        (1.0 / avg_dt).min(120.0)
    }

    pub fn queue_spawn(&mut self, template: impl Into<String>, x: f32, y: f32) {
        self.pending_spawns.push(PendingSpawnRequest {
            kind: PendingSpawnKind::Template(template.into()),
            x,
            y,
            velocity: None,
            tags: Vec::new(),
            hp: None,
            anim: None,
            request_seq: None,
        });
    }

    pub fn queue_destroy_last_spawned(&mut self) -> bool {
        let Some(id) = self.last_spawned_entity_id.clone() else {
            return false;
        };
        self.queue_destroy(id);
        true
    }

    pub fn save_game(&mut self, project_root: &Path) -> Result<(), String> {
        if let Some(path) = &self.scene_manager.current_path {
            let persisted_path = path
                .strip_prefix(project_root)
                .unwrap_or(path.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            self.save_data.current_scene = Some(persisted_path.clone());
            self.save_data
                .set_text(SAVE_KEY_CHECKPOINT_SCENE, persisted_path);
        } else if let Some(scene) = &self.active_scene {
            self.save_data.current_scene = Some(scene.name.clone());
            self.save_data
                .set_text(SAVE_KEY_CHECKPOINT_SCENE, scene.name.clone());
        }

        if let Some(scene) = &self.active_scene {
            if let Some((player_id, player_name, x, y, vx, vy)) =
                capture_continue_player_snapshot(scene)
            {
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

    pub fn load_game(&mut self, project_root: &Path) -> bool {
        if let Some(data) = SaveData::load_from_project(project_root) {
            self.save_data = data;
            true
        } else {
            false
        }
    }

    pub fn stop_audio_by_name(&mut self, name: &str) -> usize {
        let stopped = self.audio_runtime.stop_by_name(name);
        if stopped > 0 {
            let lowered = name.trim().to_ascii_lowercase();
            self.started_audio
                .retain(|key| !key.to_ascii_lowercase().contains(&lowered));
        }
        stopped
    }

    pub fn update_frame(&mut self, project_root: &Path, ground_y: f32) {
        let now = Instant::now();
        let dt = if let Some(last) = self.last_frame_at {
            (now - last).as_secs_f32()
        } else {
            1.0 / 60.0
        };

        // Limita update do runtime a no máximo 120 FPS.
        // Se ainda não bateu o orçamento do frame, não simula neste tick.
        if self.last_frame_at.is_some() && dt < MIN_FRAME_DT {
            return;
        }
        self.last_frame_at = Some(now);
        self.frame_count = self.frame_count.saturating_add(1);

        // Evita salto gigante após travas, mas não força dt mínimo.
        // Forçar mínimo acelera o jogo em máquinas com FPS muito alto.
        self.delta_time = dt.min(0.1);
        // Atualiza ring buffer para FPS suavizado (média de 30 frames).
        let n = self.fps_samples.len();
        self.fps_samples[self.fps_sample_idx % n] = self.delta_time;
        self.fps_sample_idx = self.fps_sample_idx.wrapping_add(1);
        self.elapsed_time += self.delta_time;
        self.last_stage = RuntimeFrameStage::UpdateScriptsAndMovement;

        self.current_runtime_events = std::mem::take(&mut self.pending_runtime_events);

        self.previous_collision_contacts = self.collision_contacts.clone();
        self.previous_collision_contact_ids = self.collision_contact_ids.clone();
        self.previous_trigger_contacts = self.trigger_contacts.clone();
        self.previous_trigger_contact_ids = self.trigger_contact_ids.clone();

        self.collision_contacts.clear();
        self.collision_contact_ids.clear();
        self.trigger_contacts.clear();
        self.trigger_contact_ids.clear();

        let mut camera_follow_target: Option<(f32, f32)> = None;
        let mut camera_shake: Option<(f32, f32)> = None;
        let mut camera_zoom: Option<f32> = None;

        let command = if let Some(scene) = &mut self.active_scene {
            let scene_label_owned = scene.name.clone();
            systems::update_entities_runtime(
                &mut scene.entities,
                self.delta_time,
                self.elapsed_time,
                project_root,
                Some(scene_label_owned.as_str()),
                &mut self.started_scripts,
                &mut camera_follow_target,
                ground_y,
                &mut self.save_data,
                &mut self.session_state,
                &mut self.script_state,
                &self.input,
                self.frame_count,
                &mut self.lua_vms,
                &mut self.collision_contacts,
                &mut self.collision_contact_ids,
                &mut self.trigger_contacts,
                &mut self.trigger_contact_ids,
                &self.previous_collision_contacts,
                &self.previous_collision_contact_ids,
                &self.previous_trigger_contacts,
                &self.previous_trigger_contact_ids,
                &mut self.pending_destroys,
                &mut self.pending_spawns,
                &mut self.pending_particles,
                &mut self.emitters,
                &self.current_runtime_events,
                &mut self.pending_runtime_events,
                &mut camera_shake,
                &mut camera_zoom,
                &mut self.camera_controller,
                &mut self.audio_runtime,
            )
        } else {
            None
        };

        if let Some((time, intensity)) = camera_shake {
            self.camera_shake_time = time.max(0.0);
            self.camera_shake_intensity = intensity.max(0.0);
        }
        if let Some(zoom) = camera_zoom {
            self.camera_zoom_override = Some(zoom.max(0.01));
        }

        if !self.pending_destroys.is_empty() {
            let destroys = std::mem::take(&mut self.pending_destroys);
            let mut cleared_script_entities: Vec<String> = Vec::new();
            if let Some(scene) = &mut self.active_scene {
                for request in destroys {
                    if scene.remove_entity_by_id(&request.entity_id) {
                        cleared_script_entities.push(request.entity_id);
                    }
                }
            }
            for entity_id in cleared_script_entities {
                self.clear_script_state_for_entity(&entity_id);
            }
        }

        if !self.pending_spawns.is_empty() {
            let spawns = std::mem::take(&mut self.pending_spawns);
            if let Some(scene) = &mut self.active_scene {
                for request in spawns {
                    let mut entity = Entity::new(match &request.kind {
                        PendingSpawnKind::Template(name) => name.clone(),
                        PendingSpawnKind::PrefabPath(path) => Path::new(path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Spawned")
                            .to_string(),
                    });

                    if let Some(transform) = entity.transform_mut() {
                        transform.x = request.x;
                        transform.y = request.y;
                    }

                    entity.add_component(Component::Velocity(Velocity {
                        x: request.velocity.map(|v| v.0).unwrap_or(0.0),
                        y: request.velocity.map(|v| v.1).unwrap_or(0.0),
                    }));

                    for tag in request.tags {
                        entity.add_tag(tag);
                    }

                    let spawned_id = entity.id.clone();
                    scene.add_entity(entity);
                    self.last_spawned_entity_id = Some(spawned_id.clone());

                    if let Some(seq) = request.request_seq {
                        self.pending_runtime_events.push(RuntimeEvent {
                            name: format!("spawn_result:{}", seq),
                            data: Some(SaveValue::Text(spawned_id)),
                        });
                    }
                }
            }
        }

        self.rebuild_collision_events();

        if !self.pending_particles.is_empty() {
            let pending = std::mem::take(&mut self.pending_particles);
            self.particles.extend(pending);
        }
        self.update_emitters(self.delta_time);
        self.update_particles(self.delta_time);

        if let Some(command) = command {
            match command {
                RuntimeCommand::ChangeScene(path) => {
                    self.queue_scene_change_cached(PathBuf::from(path));
                }
                RuntimeCommand::ReloadScene => {
                    if self.scene_manager.reload_scene().is_ok() {
                        self.active_scene = self.scene_manager.current_scene.clone();
                        self.reset_timing_state();
                        self.game_state.flow = RuntimeGameFlow::Playing;
                        self.game_state.loading_label = None;
                    }
                }
            }
        }

        self.last_stage = RuntimeFrameStage::FinalizeFrame;
    }

    pub fn queue_destroy(&mut self, entity_id: impl Into<String>) {
        let entity_id = entity_id.into();
        if self
            .pending_destroys
            .iter()
            .any(|request| request.entity_id == entity_id)
        {
            return;
        }
        self.pending_destroys
            .push(PendingDestroyRequest { entity_id });
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

fn find_continue_player<'a>(
    scene: &'a Scene,
    preferred_id: Option<&str>,
    preferred_name: Option<&str>,
) -> Option<&'a Entity> {
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

fn find_continue_player_mut<'a>(
    scene: &'a mut Scene,
    preferred_id: Option<&str>,
    preferred_name: Option<&str>,
) -> Option<&'a mut Entity> {
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
