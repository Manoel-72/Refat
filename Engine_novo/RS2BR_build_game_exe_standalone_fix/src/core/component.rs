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
    #[serde(default)]
    pub screen_space: bool,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            texture_path: String::new(),
            color_r: 1.0,
            color_g: 1.0,
            color_b: 1.0,
            color_a: 1.0,
            screen_space: false,
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
    #[serde(default)]
    
    pub hit_ceiling: bool,
    #[serde(default)]
    
    pub hit_left: bool,
    #[serde(default)]
    
    pub hit_right: bool,
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self {
            gravity_scale: 1.0,
            is_static: false,
            grounded: false,
            hit_ceiling: false,
            hit_left: false,
            hit_right: false,
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyType {
    Static,
    Kinematic,
    Trigger,
}

impl Default for BodyType {
    fn default() -> Self {
        Self::Kinematic
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape2D {
    Box { width: f32, height: f32 },
    Circle { radius: f32 },
}

impl Default for Shape2D {
    fn default() -> Self {
        Self::Box { width: 32.0, height: 32.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxCollider {
    /// Campos legados preservados para compatibilidade com cenas antigas.
    #[serde(default = "default_collider_size")]
    pub width: f32,
    #[serde(default = "default_collider_size")]
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    #[serde(default)]
    pub is_trigger: bool,
    /// Se false, o collider existe na entidade mas é ignorado pelo runtime.
    #[serde(default = "default_collision_enabled")]
    pub collision_enabled: bool,
    /// Layer desta entidade. 0 = padrão/interage com tudo.
    #[serde(default)]
    pub layer: u32,
    /// Máscara de layers com quem esta entidade pode colidir. 0 = todos.
    #[serde(default)]
    pub mask: u32,
    #[serde(default)]
    pub body_type: BodyType,
    #[serde(default)]
    pub shape: Shape2D,
    #[serde(default)]
    pub one_way: bool,
    #[serde(default = "default_one_way_margin")]
    pub one_way_margin: f32,
}

impl BoxCollider {
    pub fn resolved_shape(&self) -> Shape2D {
        match &self.shape {
            Shape2D::Circle { radius } => Shape2D::Circle {
                radius: (*radius).max(0.0),
            },
            Shape2D::Box { .. } => Shape2D::Box {
                width: self.width.max(0.0),
                height: self.height.max(0.0),
            },
        }
    }

    pub fn resolved_body_type(&self, rigidbody: Option<&RigidBody2D>) -> BodyType {
        if self.is_trigger || matches!(self.body_type, BodyType::Trigger) {
            BodyType::Trigger
        } else if matches!(self.body_type, BodyType::Static)
            || rigidbody.map(|rb| rb.is_static).unwrap_or(false)
        {
            BodyType::Static
        } else {
            BodyType::Kinematic
        }
    }

    pub fn half_height_for_grounding(&self) -> f32 {
        match self.resolved_shape() {
            Shape2D::Box { height, .. } => height * 0.5,
            Shape2D::Circle { radius } => radius,
        }
    }

    pub fn radius(&self) -> f32 {
        match self.resolved_shape() {
            Shape2D::Circle { radius } => radius,
            Shape2D::Box { width, height } => (width.min(height) * 0.5).max(0.0),
        }
    }
}

impl Default for BoxCollider {
    fn default() -> Self {
        Self {
            width: 32.0,
            height: 32.0,
            offset_x: 0.0,
            offset_y: 0.0,
            is_trigger: false,
            collision_enabled: true,
            layer: 0,
            mask: 0,
            body_type: BodyType::Kinematic,
            shape: Shape2D::Box { width: 32.0, height: 32.0 },
            one_way: false,
            one_way_margin: default_one_way_margin(),
        }
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
fn default_collision_enabled() -> bool { true }
fn default_collider_size() -> f32 { 32.0 }
fn default_one_way_margin() -> f32 { 6.0 }

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

fn default_locomotion_threshold() -> f32 { 6.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationState {
    #[serde(default)]
    pub clip: String,
    #[serde(default)]
    pub looped: bool,
    #[serde(default)]
    pub interruptible: bool,
    #[serde(default)]
    pub next_state: String,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            clip: String::new(),
            looped: true,
            interruptible: true,
            next_state: String::new(),
        }
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
    /// Clip anterior — usado pelo runtime para detectar troca e resetar o timer.
    #[serde(default)]
    pub prev_clip: String,
    #[serde(default)]
    pub state_mode: bool,
    #[serde(default)]
    pub states: HashMap<String, AnimationState>,
    #[serde(default)]
    pub current_state: String,
    #[serde(default)]
    pub default_state: String,
    #[serde(default)]
    pub queued_state: String,
    #[serde(default = "default_locomotion_threshold")]
    pub locomotion_threshold: f32,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            clips: HashMap::new(),
            current: String::new(),
            timer: 0.0,
            playing: true,
            looped: true,
            prev_clip: String::new(),
            state_mode: true,
            states: HashMap::new(),
            current_state: String::new(),
            default_state: String::new(),
            queued_state: String::new(),
            locomotion_threshold: default_locomotion_threshold(),
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
            text: "Texto".to_string(),
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
#[serde(default)]
pub struct UIButton {
    pub label: String,
    pub action: String,
    pub width: f32,
    pub height: f32,

    pub text: String,
    pub font_size: f32,

    pub target_scene: String,
    pub close_runtime: bool,

    pub screen_space: bool,

    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub color_a: f32,

    pub text_r: f32,
    pub text_g: f32,
    pub text_b: f32,
    pub text_a: f32,
}


impl Default for UIButton {
    fn default() -> Self {
        Self {
            label: "Button".to_string(),
            action: "".to_string(),
            width: default_button_width(),
            height: default_button_height(),

            text: "Button".to_string(),
            font_size: 18.0,

            target_scene: "".to_string(),
            close_runtime: false,

            screen_space: true,

            color_r: 0.2,
            color_g: 0.6,
            color_b: 0.2,
            color_a: 1.0,

            text_r: 1.0,
            text_g: 1.0,
            text_b: 1.0,
            text_a: 1.0,
        }
    }
}
