// ============================================================
//  engine/component.rs
//  Componentes built-in da engine.
//  Um componente é um "pedaço de dados/comportamento" que você
//  cola numa entidade (igual ao Unity/Godot).
// ============================================================

use serde::{Deserialize, Serialize};

/// Todos os tipos de componente suportados
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Component {
    /// Posição, rotação e escala no mundo 2D
    Transform(Transform),
    /// Renderiza um sprite (imagem)
    Sprite(Sprite),
    /// Câmera 2D
    Camera2D(Camera2D),
    /// Corpo de física simples
    RigidBody2D(RigidBody2D),
    /// Colisão em caixa
    BoxCollider(BoxCollider),
    /// Script RS2 interpretado (caminho do arquivo .rs2)
    Script(Script),
    /// Áudio simples reproduzido pelo runtime
    Audio(Audio),
}

impl Component {
    /// Nome legível do componente (exibido no Inspector)
    pub fn display_name(&self) -> &str {
        match self {
            Component::Transform(_)    => "Transform",
            Component::Sprite(_)       => "Sprite",
            Component::Camera2D(_)     => "Camera 2D",
            Component::RigidBody2D(_)  => "RigidBody 2D",
            Component::BoxCollider(_)  => "Box Collider",
            Component::Script(_)       => "Script RS2",
            Component::Audio(_)        => "Audio",
        }
    }

    /// Cria um Transform com valores padrão
    pub fn transform_default() -> Self {
        Component::Transform(Transform {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        })
    }
}

// ── Definições dos structs de componente ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,   // graus
    pub scale_x: f32,
    pub scale_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprite {
    /// Caminho relativo ao asset dentro da pasta /assets
    pub texture_path: String,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub color_a: f32,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            texture_path: String::new(),
            color_r: 1.0, color_g: 1.0, color_b: 1.0, color_a: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera2D {
    pub zoom: f32,
    pub is_main: bool,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self { zoom: 1.0, is_main: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigidBody2D {
    pub gravity_scale: f32,
    pub is_static: bool,
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self { gravity_scale: 1.0, is_static: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxCollider {
    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    #[serde(default)]
    pub is_trigger: bool,
}

impl Default for BoxCollider {
    fn default() -> Self {
        Self { width: 32.0, height: 32.0, offset_x: 0.0, offset_y: 0.0, is_trigger: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    /// Caminho do arquivo .rs2 relativo ao projeto
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audio {
    /// Caminho relativo ao arquivo de áudio no projeto
    pub file_path: String,
    #[serde(default)]
    pub play_on_start: bool,
    #[serde(default)]
    pub looped: bool,
    #[serde(default = "default_audio_volume")]
    pub volume: f32,
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            file_path: String::new(),
            play_on_start: true,
            looped: false,
            volume: 1.0,
        }
    }
}

fn default_audio_volume() -> f32 {
    1.0
}
