# RS2BR-Editor UI — Módulo egui

Implementação Rust do novo layout do Editor Principal usando `egui 0.29` + `eframe 0.29`.

## Estrutura dos arquivos

```
src/
├── main.rs                  ← Ponto de entrada (NativeOptions, janela 1280x800)
└── editor/
    ├── mod.rs               ← EditorApp + layout geral (todos os painéis)
    ├── state.rs             ← Tipos de dados: Entity, Transform, Sprite, Collider...
    ├── theme.rs             ← Paleta de cores + apply() no egui::Context
    ├── hierarchy.rs         ← Painel esquerdo: lista de entidades
    ├── inspector.rs         ← Painel direito: componentes da entidade selecionada
    ├── scene_view.rs        ← Painel central: grade + viewport + zoom/pan
    └── asset_browser.rs     ← Painel inferior: abas (Assets, Animator, Console)
```

## Layout dos painéis egui

```
TopBottomPanel::top("title_bar")        28px  — nome + versão + avisos
TopBottomPanel::top("menu_bar")         24px  — File/Create/Scene/Editor/Help + Undo/Redo
TopBottomPanel::top("play_toolbar")     38px  — ▶ Play / ⏸ Pause / ⏹ Stop
TopBottomPanel::bottom("status_bar")    22px  — modo edição + contagem de avisos
TopBottomPanel::bottom("bottom_panel") ~180px — Asset Browser / Animator / Console
SidePanel::left("hierarchy")           180px  — hierarquia de entidades (redimensionável)
SidePanel::right("inspector")          220px  — inspetor de componentes (redimensionável)
CentralPanel                            —      — viewport da cena (grade + pan + zoom)
```

## Como integrar ao seu projeto RS2BR

### Opção 1 — Usar como binário separado (recomendado para teste)

```toml
# Cargo.toml da sua workspace
[workspace]
members = ["seu_engine", "rs2br-editor-ui"]
```

### Opção 2 — Incorporar os módulos ao seu engine

1. Copie a pasta `src/editor/` para dentro do seu crate de editor.
2. Substitua `EditorState` pelos seus tipos reais (Entity, Scene, etc.).
3. Conecte `EditorApp::update` ao seu loop `eframe::App::update`.

### Substituindo os dados mock

Em `state.rs`, o `EditorState::default()` cria dados de exemplo.
Para usar seus dados reais:

```rust
impl EditorState {
    pub fn from_scene(scene: &YourScene) -> Self {
        let entities = scene.entities().iter().enumerate().map(|(i, e)| {
            Entity {
                id: i,
                name: e.name.clone(),
                kind: entity_kind_from_components(&e.components),
                depth: e.parent.map(|_| 1).unwrap_or(0),
                // ...
            }
        }).collect();
        // ...
    }
}
```

## Dependências

```toml
[dependencies]
eframe  = { version = "0.29", features = ["default_fonts"] }
egui    = "0.29"
```

## Funcionalidades implementadas

- [x] Grid da cena com zoom (scroll) e pan (arrastar)
- [x] Clique na entidade no viewport para selecioná-la
- [x] Hierarquia rolável com destaque da entidade selecionada
- [x] Menu de contexto na hierarquia (botão direito)
- [x] Inspector com DragValue editável nos campos de Transform
- [x] Colliders visíveis na cena (toggle)
- [x] Nomes visíveis na cena (toggle)
- [x] Abas: Asset Browser / Animator / Console
- [x] Filtros de asset e grade de pastas
- [x] Botões Play / Pause / Stop com estado visual
- [x] Paleta de cores centralizada em `theme.rs`
