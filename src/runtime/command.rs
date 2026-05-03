/// Comandos emitidos pelo gameplay (Lua, colisões RS2, etc.) para o `RuntimeState`.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeCommand {
    ChangeScene(String),
    ReloadScene,
    /// Fade overlay 1 → 0 (revela).
    ScreenFadeIn { duration: f32 },
    /// Fade overlay 0 → 1 (escurece).
    ScreenFadeOut { duration: f32 },
    /// Flash de cor em `duration` segundos.
    ScreenFlash {
        r: f32,
        g: f32,
        b: f32,
        duration: f32,
    },
}
