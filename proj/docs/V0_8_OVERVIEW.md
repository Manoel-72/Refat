# RS2BR-Engine V0.8

## O que entrou
- `project.json` para definir a cena inicial do projeto.
- `Animator` frame-based com `AnimationClip`.
- Cache em memória para `ScriptBehavior` de arquivos `.rs2`.
- Componentes de UI de jogo: `TextLabel` e `UIButton`.
- Validações iniciais para animação e UI no editor.

## Objetivo da V0.8
Tornar a engine mais usável para projeto real, removendo hardcode de cena inicial, reduzindo custo de leitura de scripts e permitindo HUD/menu simples dentro do runtime.

## Ordem recomendada de uso
1. Definir `project.json`.
2. Criar a cena inicial em `assets/scenes`.
3. Anexar `Sprite + Animator` em entidades visuais.
4. Anexar `TextLabel` e `UIButton` em entidades de HUD.
5. Usar `.rs2` para lógica simples e troca de cena.
