// ============================================================
//  runtime/context.rs  —  V0.9
//  Trait RuntimeContext: contrato entre runtime e host.
//
//  O runtime não precisa saber se está dentro do editor,
//  num standalone build, ou num teste automatizado.
//  Ele só fala com quem implementar este trait.
// ============================================================

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use eframe::egui;

use crate::core::scene::Scene;

/// Tudo que o runtime precisa do "lado de fora" (editor ou standalone).
pub trait RuntimeContext {
    // ── estado de execução ───────────────────────────────────

    /// Estado atual: editando, rodando ou pausado.
    fn play_state(&self) -> RuntimePlayState;
    fn set_play_state(&mut self, state: RuntimePlayState);

    // ── cena ────────────────────────────────────────────────

    /// Pasta raiz do projeto (para resolver paths de assets).
    fn project_root(&self) -> &Path;

    /// Snapshot da cena atualmente aberta no editor / host.
    fn active_scene_snapshot(&self) -> &Scene;

    /// Lista de paths de arquivos de cena disponíveis no projeto.
    fn scene_file_candidates(&self) -> Vec<PathBuf>;

    /// Cena aberta por path, se existir em memória (evita reload do disco).
    fn scene_snapshot_by_path(&self, path: &Path) -> Option<(Scene, Option<PathBuf>)>;

    // ── feedback ────────────────────────────────────────────

    /// Mensagem de status exibida na UI do host.
    fn set_status(&mut self, msg: String);

    // ── texturas (para render inline no editor) ──────────────

    fn sprite_textures(&mut self) -> &mut HashMap<String, egui::TextureHandle>;
}

/// Estado de execução — espelho de `EditorPlayState`, mas sem depender do editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePlayState {
    Edit,
    Playing,
    Paused,
}

impl RuntimePlayState {
    pub fn is_playing(self) -> bool {
        self == RuntimePlayState::Playing
    }
    pub fn is_paused(self) -> bool {
        self == RuntimePlayState::Paused
    }
    pub fn is_edit(self) -> bool {
        self == RuntimePlayState::Edit
    }
}
