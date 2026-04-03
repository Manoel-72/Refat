# RS2BR-Engine V0.5

## Arquitetura oficial

- `core/`: dados puros compartilhados entre editor e runtime
- `assets/`: sistema de assets e tipos de asset
- `serialization/`: save/load de cenas e prefabs
- `editor/`: ferramenta de criação
- `runtime/`: execução do jogo

## Regras

- `Scene`, `Entity`, `Transform` e `Component` ficam no `core`
- `Scene` representa qualquer tela (menu, loading, fase)
- UI pertence à scene, mas é processada pelo runtime
- Runtime continua no mesmo projeto, separado por responsabilidade
- `.scene.json` é carregado direto pelo runtime
- `Prefab` é uma entidade reutilizável com componentes
- `Script .rs2` continua suportado nesta versão
