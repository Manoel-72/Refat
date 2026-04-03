# 🎮 RS2BR-Engine

Engine de jogo 2D simples com editor visual, escrita em **R2S**.

---

## 📦 Dependências e Instalação

### 1. Instale o Rust (se ainda não tiver)

```bash
# No Linux/macOS:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# No Windows:
# Baixe o instalador em: https://rustup.rs
```

Após instalar, reinicie o terminal e confirme:
```bash
rustc --version   # ex: rustc 1.77.0
cargo --version   # ex: cargo 1.77.0
```

### 2. Dependências do sistema (Linux)

No Ubuntu/Debian:
```bash
sudo apt install -y \
  libx11-dev libxrandr-dev libxcursor-dev libxi-dev \
  libgl1-mesa-dev libgles2-mesa-dev libasound2-dev \
  pkg-config build-essential
```

No Arch Linux:
```bash
sudo pacman -S libx11 libxrandr libxcursor libxi mesa alsa-lib
```

No macOS: sem deps extras (usa Metal nativamente via eframe).

No Windows: sem deps extras (usa D3D12/WGL via eframe).

### 3. Clone e rode o editor

```bash
git clone <seu-repo>
cd rust2d_engine

# Modo debug (mais rápido de compilar):
cargo run

# Modo release (mais performance):
cargo run --release
```

A primeira compilação pode levar **2-5 minutos** (baixa e compila as deps).
As próximas compilações são muito mais rápidas.

---

## 🏗️ Arquitetura do Projeto

```
rust2d_engine/
├── Cargo.toml          ← Dependências do projeto
├── README.md           ← Este arquivo
├── assets/             ← Seus arquivos de jogo (criado automaticamente)
│   ├── sprites/
│   ├── scripts/
│   └── sounds/
├── scenes/             ← Cenas salvas como JSON
└── src/
    ├── main.rs         ← Ponto de entrada — inicia o editor
    ├── engine/         ← Núcleo da engine (dados e lógica)
    │   ├── mod.rs      ← Exporta os módulos da engine
    │   ├── entity.rs   ← Entidade = objeto do jogo (ID + nome + componentes)
    │   ├── component.rs← Tipos de componente (Transform, Sprite, Camera2D…)
    │   ├── scene.rs    ← Cena = container de entidades, salva/carrega JSON
    │   └── assets.rs   ← Lê o disco, cria scripts/pastas
    └── editor/         ← Interface visual (painéis egui)
        ├── mod.rs      ← EditorApp — estado global + janelas modais
        ├── menubar.rs  ← Menu superior (Arquivo, Cena, Criar, Ajuda)
        ├── hierarchy.rs← Painel esquerdo: árvore de entidades
        ├── inspector.rs← Painel direito: editar componentes
        ├── asset_browser.rs ← Painel inferior: assets + clique direito
        └── scene_view.rs    ← Painel central: viewport 2D com grid
```

---

## 🧠 Como Funciona a Arquitetura (ECS Simplificado)

### Entidade (`entity.rs`)
Representa um "objeto" no jogo. Cada entidade tem:
- `id` — UUID único gerado automaticamente
- `name` — nome visível no editor
- `components` — lista de comportamentos/dados
- `children` — entidades filhas (hierarquia pai→filho)

### Componente (`component.rs`)
Um componente é um "pedaço de dado" colado numa entidade.
É um `enum` Rust com variantes:

| Componente   | O que faz |
|-------------|-----------|
| `Transform` | Posição (X,Y), rotação e escala |
| `Sprite`    | Qual textura renderizar + cor |
| `Camera2D`  | Câmera com zoom |
| `RigidBody2D` | Física: gravidade, estático/dinâmico |
| `BoxCollider` | Colisão em caixa retangular |
| `Script`    | Caminho de um script `.rs` |

### Cena (`scene.rs`)
Uma cena é um container de entidades. Pode ser:
- Salva como JSON (`Arquivo → Salvar Cena`)
- Carregada de volta (`Arquivo → Carregar Cena`)

### Gerenciador de Assets (`assets.rs`)
- Lê a pasta `assets/` e constrói uma árvore de arquivos
- Cria scripts `.rs` com template automático
- Cria novas pastas

---

## 🖥️ Painéis do Editor

### 🌳 Hierarquia (esquerda)
- Lista todas as entidades da cena em árvore
- Clique para selecionar
- Botão "➕ Nova Entidade" ou Menu → Criar
- Clique direito na entidade → Deletar

### 🔍 Inspector (direita)
- Exibe todos os componentes da entidade selecionada
- Edite valores diretamente (drag nos números, sliders para cores)
- Adicione novos componentes com os botões na parte inferior

### 🎬 Cena 2D (centro)
- Viewport com grid de 32px
- Eixo X (vermelho) e Y (verde)
- Entidades aparecem como gizmos azuis/amarelos
- **Arraste** para mover entidades
- Clique no fundo para desselecionar

### 📂 Assets (inferior)
- Navega pelos arquivos em `assets/`
- **Clique direito numa pasta** → opções:
  - `⚙ Criar Script (.rs)` — cria um arquivo `.rs` com template
  - `📁 Criar Pasta` — cria uma subpasta
  - `🔄 Recarregar` — relê o disco
- Clique direito num arquivo → copiar caminho

### 📋 Menu Superior
- **Arquivo**: Nova Cena, Salvar (JSON), Carregar, Sair
- **Cena**: Adicionar entidade, mudar cor de fundo
- **Criar**: Entidade vazia, Sprite, Câmera
- **Ajuda**: Atalhos e versão

---

## 📝 Criando um Script

1. No painel Assets, clique direito numa pasta (ex: `scripts`)
2. Clique em `⚙ Criar Script (.rs)`
3. Digite o nome (ex: `jogador`)
4. Um arquivo `jogador.rs` é criado com este template:

```rust
pub struct Jogador {
    // Campos do script aqui
}

impl Jogador {
    pub fn start(&mut self) {
        println!("Script 'jogador' iniciado!");
    }

    pub fn update(&mut self, delta_time: f32) {
        // Chamado todo frame
        let _ = delta_time;
    }
}
```

5. Para associar ao objeto: selecione a entidade → Inspector → `⚙ Script` → coloque o caminho do arquivo

---

## 🔧 Crates Usadas e Por Quê

| Crate | Versão | Uso |
|-------|--------|-----|
| `eframe` | 0.27 | Framework de janela nativa com egui integrado |
| `egui` | 0.27 | UI imediata (immediate mode) — toda a interface do editor |
| `serde` + `serde_json` | 1.x | Serializar/deserializar cenas para JSON |
| `uuid` | 1.x | Gerar IDs únicos para entidades |
| `log` + `env_logger` | — | Logging para debug |

> **Por que egui?** É 100% Rust, sem deps externas complicadas, funciona em Linux/Windows/macOS sem configuração, e é perfeito para editors/tooling.

---

## 🚀 Próximos Passos (Extensões Possíveis)

- [ ] Renderização real com `wgpu` ou `macroquad`
- [ ] Sistema de física com `rapier2d`
- [ ] Hot-reload de scripts com `libloading`
- [ ] Undo/Redo com histórico de comandos
- [ ] Prefabs (entidades reutilizáveis)
- [ ] Exportar projeto para executável
