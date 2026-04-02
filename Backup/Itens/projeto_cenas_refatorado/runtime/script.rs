// Funções e structs de scripts do runtime.
// Este módulo existe para receber o parser e o executor de scripts.

#[derive(Debug, Clone, Default)]
pub struct ScriptContext {
    pub delta_time: f32,
    pub elapsed_time: f32,
}

pub fn module_note() -> &'static str {
    "Mover gradualmente o parsing e a execução de scripts para este módulo."
}
