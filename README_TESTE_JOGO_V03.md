# Teste do jogo V0.3

## O que foi montado
- `assets/scenes/fase1.scene.json`
- `assets/scenes/fase2.scene.json`
- `assets/scripts/player.rs2`
- `assets/scripts/portal_fase2.rs2`
- `assets/scripts/portal_menu.rs2`
- suporte a `@on_collision change_scene ...`

## Como testar
1. Abra o projeto.
2. Rode com `cargo run`.
3. No editor, carregue `assets/scenes/fase1.scene.json`.
4. Clique em **Play**.
5. Use **WASD** para mover o player azul.
6. Encoste no portal laranja da direita.
7. A engine deve trocar para a **Fase 2**.
8. Na Fase 2, encoste no portal amarelo para voltar à Fase 1.

## Resultado esperado
- Player move normalmente.
- Câmera segue o player.
- Ao colidir com o portal da Fase 1, troca para a Fase 2.
- Ao colidir com o portal da Fase 2, volta para a Fase 1.
- O console mostra mensagens de `@start_message` quando os scripts iniciam.

## Se não trocar de fase
Verifique:
- se a cena aberta é a `fase1.scene.json`
- se os scripts estão em `assets/scripts`
- se os paths dos scripts estão corretos no JSON
- se o runtime foi iniciado em **Play**
