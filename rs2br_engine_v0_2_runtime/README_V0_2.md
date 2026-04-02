RS2BR-Engine V0.2

Mudanças aplicadas:
- runtime modularizado de verdade
- RuntimeState usa SceneManager
- play/pause/stop/reload organizados
- troca de cena agendada durante o runtime
- renderer do runtime movido para runtime/renderer.rs
- parser de script implementado em runtime/script.rs
- input do runtime atualizado para recarregar cena corretamente

Arquivos principais alterados:
- src/runtime/mod.rs
- src/runtime/state.rs
- src/runtime/input.rs
- src/runtime/renderer.rs
- src/runtime/script.rs
- src/runtime/scene_manager.rs

Observação:
- não removi arquivos antigos fora do fluxo principal para evitar quebrar seu projeto.
- se aparecer warning de arquivo não usado, ele tende a vir de módulos antigos ou helpers ainda não chamados.
