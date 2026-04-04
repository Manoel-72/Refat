// ============================================================
//  engine/component.rs
//  Componentes built-in da engine.
// ============================================================

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Todos os tipos de componente suportados
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Component {
    Transform(Transform),
    Sprite(Sprite),
    Camera2D(Camera2D),
    RigidBody2D(RigidBody2D),
    Velocity(Velocity),
    BoxCollider(BoxCollider),
    Script(Script),
    LuaScript(LuaScript),
    Audio(Audio),
    Animator(Animator),
    TextLabel(TextLabel),
    UIButton(UIButton),
}

impl Component {
    pub fn display_name(&self) -> &str {
        match self {
            Component::Transform(_) => "Transform",
            Component::Sprite(_) => "Sprite",
            Component::Camera2D(_) => "Camera 2D",
            Component::RigidBody2D(_) => "RigidBody 2D",
            Component::Velocity(_) => "Velocity",
            Component::BoxCollider(_) => "Box Collider",
            Component::Script(_) => "Script RS2",
            Component::LuaScript(_) => "Script Lua",
            Component::Audio(_) => "Audio",
            Component::Animator(_) => "Animator",
            Component::TextLabel(_) => "Text Label",
            Component::UIButton(_) => "UI Button",
        }
    }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprite {
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
            color_r: 1.0,
            color_g: 1.0,
            color_b: 1.0,
            color_a: 1.0,
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
    #[serde(default)]
    pub grounded: bool,
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self { gravity_scale: 1.0, is_static: false, grounded: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Velocity {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
}

impl Default for Velocity {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
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
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuaScript {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audio {
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

fn default_audio_volume() -> f32 { 1.0 }
fn default_animation_fps() -> f32 { 8.0 }
fn default_ui_alpha() -> f32 { 1.0 }
fn default_text_size() -> f32 { 24.0 }
fn default_button_width() -> f32 { 220.0 }
fn default_button_height() -> f32 { 48.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClip {
    #[serde(default)]
    pub frames: Vec<String>,
    #[serde(default = "default_animation_fps")]
    pub fps: f32,
}

impl Default for AnimationClip {
    fn default() -> Self {
        Self { frames: Vec::new(), fps: default_animation_fps() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Animator {
    #[serde(default)]
    pub clips: HashMap<String, AnimationClip>,
    #[serde(default)]
    pub current: String,
    #[serde(default)]
    pub timer: f32,
    #[serde(default)]
    pub playing: bool,
    #[serde(default)]
    pub looped: bool,
}

impl Default for Animator {
    fn default() -> Self {
        let mut clips = HashMap::new();
        clips.insert("idle".to_string(), AnimationClip::default());
        Self {
            clips,
            current: "idle".to_string(),
            timer: 0.0,
            playing: true,
            looped: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLabel {
    #[serde(default)]
    pub text: String,
    #[serde(default = "default_text_size")]
    pub font_size: f32,
    #[serde(default)]
    pub color_r: f32,
    #[serde(default)]
    pub color_g: f32,
    #[serde(default)]
    pub color_b: f32,
    #[serde(default = "default_ui_alpha")]
    pub color_a: f32,
    #[serde(default)]
    pub screen_space: bool,
}

impl Default for TextLabel {
    fn default() -> Self {
        Self {
            text: "Novo texto".to_string(),
            font_size: default_text_size(),
            color_r: 1.0,
            color_g: 1.0,
            color_b: 1.0,
            color_a: 1.0,
            screen_space: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIButton {
    #[serde(default)]
    pub text: String,
    #[serde(default = "default_button_width")]
    pub width: f32,
    #[serde(default = "default_button_height")]
    pub height: f32,
    #[serde(default = "default_text_size")]
    pub font_size: f32,
    #[serde(default)]
    pub target_scene: String,
    #[serde(default)]
    pub close_runtime: bool,
    #[serde(default)]
    pub color_r: f32,
    #[serde(default)]
    pub color_g: f32,
    #[serde(default)]
    pub color_b: f32,
    #[serde(default = "default_ui_alpha")]
    pub color_a: f32,
    #[serde(default)]
    pub text_r: f32,
    #[serde(default)]
    pub text_g: f32,
    #[serde(default)]
    pub text_b: f32,
    #[serde(default = "default_ui_alpha")]
    pub text_a: f32,
    #[serde(default)]
    pub screen_space: bool,
}

impl Default for UIButton {
    fn default() -> Self {
        Self {
            text: "Button".to_string(),
            width: default_button_width(),
            height: default_button_height(),
            font_size: 20.0,
            target_scene: String::new(),
            close_runtime: false,
            color_r: 0.18,
            color_g: 0.58,
            color_b: 0.32,
            color_a: 1.0,
            text_r: 1.0,
            text_g: 1.0,
            text_b: 1.0,
            text_a: 1.0,
            screen_space: true,
        }
    }
}
