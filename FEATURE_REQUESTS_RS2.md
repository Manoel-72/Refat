# RS2 Engine - Feature Gap Analysis and Planned Implementations

Este documento resume as funcionalidades solicitadas para a engine RS2 e serve como especificação técnica para implementação.

## 1. Prefab System com Override de Propriedades
### Situação Atual
- `src/core/prefab.rs` contém apenas:
  - `name`
  - `root_entity`

### Implementação Proposta
Adicionar:

```rust
pub struct PrefabOverride {
    pub component: String,
    pub property: String,
    pub value: serde_json::Value,
}

pub struct PrefabInstance {
    pub prefab_name: String,
    pub overrides: Vec<PrefabOverride>,
}
```

### Benefício
Permite customizar cada instância de um prefab sem alterar o original.

---

## 2. UI System com Anchors
### Situação Atual
- `screen_space: bool`
- Transform em coordenadas absolutas.

### Implementação Proposta
```rust
pub enum UIAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

pub struct UITransform {
    pub anchor: UIAnchor,
    pub offset: Vec2,
    pub normalized: Option<Vec2>, // 0.0..1.0
}
```

### Benefício
HUDs e menus responsivos.

---

## 3. Backend Real de Gamepad
### Situação Atual
- API Lua existe.
- Backend não lê botões reais.

### Implementação Proposta
Usar `macroquad::input`:
- `is_gamepad_button_down`
- `get_gamepads`

### Benefício
Suporte real a controles Xbox, PlayStation e genéricos.

---

## 4. Script RS2 com Código Rust Real
### Situação Atual
- Apenas diretivas (`@move_x`, `@on_update`).

### Implementação Proposta
Criar crate `rs2_script_api` e carregar plugins Rust compilados dinamicamente.

### Benefício
Scripts de alta performance em Rust.

---

## 5. Tilemap Editor Visual
### Situação Atual
- Importa `.tmj` do Tiled.
- Sem pintura visual interna.

### Implementação Proposta
Adicionar:
- Palette de tiles
- Ferramenta Pencil
- Ferramenta Fill
- Salvar `.tmj`

### Benefício
Editor integrado semelhante ao Unity/Godot.

---

## 6. ParticleEmitter como Component
### Situação Atual
- Partículas apenas via Lua runtime.

### Implementação Proposta
Adicionar ao enum `Component`:
```rust
ParticleEmitter(ParticleEmitterComponent)
```

### Benefício
Permite configurar partículas diretamente no editor.

---

## Prioridade Recomendada
1. Gamepad real
2. ParticleEmitter no editor
3. Prefab overrides
4. UI Anchors
5. Tilemap editor visual
6. Rust scripting

---

## Estimativa Realista (1 desenvolvedor + IA)
| Feature | Tempo |
|------:|:------|
| Gamepad | 1-2 dias |
| ParticleEmitter | 1-2 dias |
| Prefab Overrides | 3-5 dias |
| UI Anchors | 3-5 dias |
| Tilemap Editor | 2-4 semanas |
| Rust Scripting | 4-8 semanas |

