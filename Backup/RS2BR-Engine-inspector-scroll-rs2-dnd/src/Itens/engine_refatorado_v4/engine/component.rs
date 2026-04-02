use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentKind {
    Transform,
    Sprite,
    Camera2D,
    RigidBody2D,
    BoxCollider,
    Script,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Component {
    Transform(Transform),
    Sprite(Sprite),
    Camera2D(Camera2D),
    RigidBody2D(RigidBody2D),
    BoxCollider(BoxCollider),
    Script(Script),
}

impl Component {
    pub fn kind(&self) -> ComponentKind {
        match self {
            Component::Transform(_) => ComponentKind::Transform,
            Component::Sprite(_) => ComponentKind::Sprite,
            Component::Camera2D(_) => ComponentKind::Camera2D,
            Component::RigidBody2D(_) => ComponentKind::RigidBody2D,
            Component::BoxCollider(_) => ComponentKind::BoxCollider,
            Component::Script(_) => ComponentKind::Script,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Component::Transform(_) => "Transform",
            Component::Sprite(_) => "Sprite",
            Component::Camera2D(_) => "Camera 2D",
            Component::RigidBody2D(_) => "RigidBody 2D",
            Component::BoxCollider(_) => "Box Collider",
            Component::Script(_) => "Script",
        }
    }

    pub fn is_unique(&self) -> bool {
        !matches!(self, Component::Script(_))
    }

    pub fn transform_default() -> Self {
        Component::Transform(Transform::default())
    }

    pub fn camera_default() -> Self {
        Component::Camera2D(Camera2D::default())
    }

    pub fn sprite_default() -> Self {
        Component::Sprite(Sprite::default())
    }

    pub fn rigidbody_default() -> Self {
        Component::RigidBody2D(RigidBody2D::default())
    }

    pub fn collider_default() -> Self {
        Component::BoxCollider(BoxCollider::default())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Camera2D {
    pub zoom: f32,
    pub is_main: bool,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            is_main: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RigidBody2D {
    pub gravity_scale: f32,
    pub is_static: bool,
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self {
            gravity_scale: 1.0,
            is_static: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoxCollider {
    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl Default for BoxCollider {
    fn default() -> Self {
        Self {
            width: 32.0,
            height: 32.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Script {
    pub file_path: String,
}
