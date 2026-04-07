# Fix do teste de colisão V0.9.1

O problema não estava no binding Lua desse teste separado. O script de validação de colisão estava anexado ao HUD, uma entidade sem BoxCollider. Assim, `game.get_current_collision_ids()` sempre consultava a entidade errada e retornava vazio.

Correções aplicadas:
- `validation_collision_only.lua` foi movido para o Player.
- O HUD virou apenas instrução estática.
- O Player recebeu um `TextLabel` local para mostrar o estado de colisão.
- `systems.rs` foi mantido na versão estável do ZIP `Atual.zip`, sem a mudança que deslocava a ordem da física/colisão.
