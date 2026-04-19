//! Fachada de compatibilidade com código antigo (`crate::engine::…`).
//!
//! **Fonte canônica do modelo de dados do jogo:** [`crate::core`]. Use `core` em código novo
//! (`crate::core::entity`, `crate::core::component`, …). Este módulo apenas reexporta os
//! mesmos submódulos para não quebrar imports legados.
//!
//! **Assets** (arquivos do disco, catálogo no editor) permanecem em [`crate::assets`].
//!
//! Não adicione lógica aqui — só reexportações.

pub use crate::assets;
pub use crate::core::{component, entity, prefab, project, scene, version};
