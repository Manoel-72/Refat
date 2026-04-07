# Hotfix de compilação — V0.9.4

Correções aplicadas neste pacote:

- adicionada passagem de `audio_runtime` em `systems::update_entities_runtime` e na chamada em `state.rs`
- adicionados `request_seq: None` nos caminhos de `queue_spawn` e `queue_spawn_prefab`
- corrigido conflito de empréstimo mutável em `state.rs` usando `std::mem::take` para `pending_runtime_events` antes de chamar `apply_pending_entity_commands`

Observação:
- não foi possível executar `cargo check` neste ambiente porque `cargo`/`rustc` não estão instalados no container desta sessão.
