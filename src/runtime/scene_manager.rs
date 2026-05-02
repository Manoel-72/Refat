use std::path::PathBuf;

use crate::{core::scene::Scene, serialization::scene_serializer};

#[derive(Debug, Clone)]
pub enum PendingSceneChange {
    Path(PathBuf),
    Snapshot {
        scene: Scene,
        source_path: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct SceneManager {
    pub current_scene: Option<Scene>,
    pub current_path: Option<PathBuf>,
    pub pending_scene_change: Option<PendingSceneChange>,
}

impl SceneManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_editor_scene(&mut self, scene: Scene) {
        self.current_scene = Some(scene);
        self.current_path = None;
        self.pending_scene_change = None;
    }

    pub fn clear_runtime_scene(&mut self) {
        self.current_scene = None;
        self.current_path = None;
        self.pending_scene_change = None;
    }

    pub fn load_scene(&mut self, path: PathBuf) -> Result<(), String> {
        if !path.exists() {
            return Err(format!("Cena não encontrada: {}", path.display()));
        }
        let scene = scene_serializer::load_scene_from_path(&path)
            .ok_or_else(|| format!("Não foi possível carregar a cena em {}", path.display()))?;
        self.current_scene = Some(scene);
        self.current_path = Some(path);
        self.pending_scene_change = None;
        Ok(())
    }

    pub fn change_scene(&mut self, path: PathBuf) {
        self.pending_scene_change = Some(PendingSceneChange::Path(path));
    }

    pub fn change_scene_snapshot(&mut self, scene: Scene, source_path: Option<PathBuf>) {
        self.pending_scene_change = Some(PendingSceneChange::Snapshot { scene, source_path });
    }

    pub fn apply_pending_change(&mut self) -> Result<bool, String> {
        let Some(change) = self.pending_scene_change.take() else {
            return Ok(false);
        };

        match change {
            PendingSceneChange::Path(path) => {
                self.load_scene(path)?;
            }
            PendingSceneChange::Snapshot { scene, source_path } => {
                self.current_scene = Some(scene);
                self.current_path = source_path;
            }
        }

        Ok(true)
    }

    pub fn reload_scene(&mut self) -> Result<(), String> {
        let Some(path) = self.current_path.clone() else {
            return Err("Cena atual não veio de arquivo salvo".to_string());
        };
        self.load_scene(path)
    }
}
