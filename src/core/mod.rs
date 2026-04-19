//! Núcleo do modelo de dados RS2BR: entidades, componentes, cenas, prefabs e projeto.
//!
//! Este é o módulo **canônico** para tipos de jogo e serialização. O módulo [`crate::engine`]
//! existe só como reexportação para compatibilidade — prefira `crate::core` em código novo.

pub mod component;
pub mod entity;
pub mod prefab;
pub mod project;
pub mod scene;

pub mod version;
