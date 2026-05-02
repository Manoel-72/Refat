// ============================================================
//  runtime/log_queue.rs  —  Fila thread-local de logs de debug
//
//  Permite que script_system, lua_runtime e outros módulos
//  internos emitam logs sem precisar carregar &mut RuntimeState
//  em todas as assinaturas.
//
//  Uso:
//    log_queue::push_error("Lua", "mensagem de erro");
//    log_queue::push_warn("Collision", "aviso");
//    log_queue::push_info("Script", "informação");
//
//  No loop principal (state.rs update_frame), após cada frame:
//    log_queue::drain_into(&mut self.log_entries, self.frame_count, MAX);
// ============================================================

use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct PendingEntry {
    pub level: Level,
    pub tag: String,
    pub message: String,
}

thread_local! {
    static QUEUE: RefCell<Vec<PendingEntry>> = RefCell::new(Vec::new());
}

pub fn push(level: Level, tag: impl Into<String>, message: impl Into<String>) {
    QUEUE.with(|q| {
        q.borrow_mut().push(PendingEntry {
            level,
            tag: tag.into(),
            message: message.into(),
        });
    });
}

pub fn push_error(tag: impl Into<String>, message: impl Into<String>) {
    push(Level::Error, tag, message);
}

pub fn push_warn(tag: impl Into<String>, message: impl Into<String>) {
    push(Level::Warn, tag, message);
}

pub fn push_info(tag: impl Into<String>, message: impl Into<String>) {
    push(Level::Info, tag, message);
}

/// Drena a fila para o buffer do RuntimeState.
/// Deve ser chamado uma vez por frame, após update_entities_runtime.
pub fn drain_into(
    entries: &mut Vec<crate::runtime::state::RuntimeLogEntry>,
    frame: u64,
    max: usize,
) {
    QUEUE.with(|q| {
        let mut q = q.borrow_mut();
        for pending in q.drain(..) {
            let entry = match pending.level {
                Level::Info  => crate::runtime::state::RuntimeLogEntry::info(frame, pending.tag, pending.message),
                Level::Warn  => crate::runtime::state::RuntimeLogEntry::warn(frame, pending.tag, pending.message),
                Level::Error => crate::runtime::state::RuntimeLogEntry::error(frame, pending.tag, pending.message),
            };
            entries.push(entry);
        }
        // Mantém somente as últimas `max` entradas
        if entries.len() > max {
            let remove = entries.len() - max;
            entries.drain(..remove);
        }
    });
}

/// Descarta todos os logs pendentes sem processar (usado no stop()).
pub fn clear() {
    QUEUE.with(|q| q.borrow_mut().clear());
}
