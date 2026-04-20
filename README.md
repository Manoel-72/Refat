# RS2BR-Engine

> **Engine 2D brasileira feita em Rust — do zero, com propósito.**

[![Versão](https://img.shields.io/badge/versão-V0.9.0%20MVP-orange?style=flat-square)](https://github.com/)
[![Status](https://img.shields.io/badge/status-Em%20Desenvolvimento%20Ativo-brightgreen?style=flat-square)](https://github.com/)
[![Linguagem](https://img.shields.io/badge/linguagem-Rust%202021-b7410e?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Licença](https://img.shields.io/badge/licença-MIT-blue?style=flat-square)](LICENSE)

---

## O que é a RS2BR-Engine?

A **RS2BR-Engine** é uma game engine 2D nativa desenvolvida inteiramente em Rust, com editor visual integrado e runtime próprio. O objetivo é oferecer uma ferramenta **simples, direta e eficiente** para criação de jogos 2D — sem a complexidade de engines industriais, mas com estrutura sólida o suficiente para projetos reais.

O editor roda como uma aplicação desktop nativa. Não é um wrapper de Unity, Godot ou qualquer outra engine — cada sistema foi construído do zero com foco em aprendizado, controle total do código e performance nativa em Rust.

A engine é voltada para desenvolvedores brasileiros que querem criar jogos 2D sem depender de engines estrangeiras ou pagas, tendo controle total sobre o código e a arquitetura.

---

## Para que tipo de jogos serve?

A RS2BR-Engine foi desenhada para jogos 2D com foco em:

- **Plataformas 2D** — player com física, colisões, câmera seguindo o personagem
- **Jogos de aventura / puzzle** — múltiplas cenas, troca de fases, portais, triggers
- **Protótipos rápidos** — sistema de cenas e prefabs facilita testar ideias

### Próximos gêneros planejados

| Gênero | O que vai precisar |
|---|---|
| **Card Game** | Sistema de mão, baralho, turno, efeitos de carta via Lua |
| **Roguelite** | Geração procedural de salas, itens randômicos, progressão por run |
| **Survivor-like** | Spawn em massa, sistema de XP/level-up, hordas de inimigos |

Estes gêneros guiam o desenvolvimento dos próximos sistemas da engine — o que significa que cada feature nova tem um objetivo concreto de jogo.

---

## Arquitetura

A engine é dividida em módulos bem definidos:

```
rs2br-engine/
├── src/
│   ├── core/           # Componentes, entidades, cenas, prefabs, versão
│   ├── editor/         # UI do editor (hierarquia, inspetor, asset browser, scene view)
│   ├── runtime/        # Loop de jogo, sistemas, câmera, input, Lua, save/load
│   │   └── systems/    # Audio, colisão, física, movimento, render, scripts, UI
│   ├── assets/         # Gerenciamento e validação de assets
│   ├── renderer/       # Abstração de renderização 2D (Macroquad)
│   ├── serialization/  # Serialização de cenas e prefabs (JSON)
│   └── engine/         # Traits e interfaces públicas da engine
├── assets/
│   ├── scenes/         # Arquivos .scene.json
│   ├── scripts/        # Scripts .rs2 e .lua
│   ├── prefabs/        # Prefabs .prefab.json
│   ├── sprites/        # Texturas PNG/JPG
│   └── sounds/         # Áudio WAV/OGG
```

---

## Funcionalidades da V0.9 (MVP atual)

### Editor Visual
- Hierarquia de entidades com múltiplos documentos abertos simultâneos
- Inspetor de componentes com edição em tempo real
- Asset Browser com filtros por tipo (cenas, scripts, imagens, áudio, prefabs, fontes)
- Scene View para posicionamento visual de entidades
- Menu bar com ações de arquivo, edição e configuração de projeto
- Sistema de warnings e validação de assets

### Componentes Built-in
| Componente | Descrição |
|---|---|
| `Transform` | Posição, rotação e escala |
| `Sprite` | Renderização de textura com cor |
| `Camera2D` | Câmera 2D com follow automático |
| `RigidBody2D` | Corpo físico com gravidade |
| `Velocity` | Velocidade linear da entidade |
| `BoxCollider` | Colisão AABB com layers e masks |
| `Script (RS2)` | Script proprietário simples via anotações |
| `LuaScript` | Script Lua 5.4 com API completa |
| `Audio` | Reprodução de sons via Rodio |
| `Animator` | Controle de animações por clipes |
| `TextLabel` | Texto na cena |
| `UIButton` | Botão interativo para UI |

### Runtime
- Loop de jogo com estágios definidos: input → scripts → física → colisões → câmera → frame
- Estados de jogo: `Editing`, `Playing`, `Paused`, `GameOver`, `Loading`
- Transição de cenas com estado pendente e tela de loading
- Spawn e destroy de entidades via fila no fim do frame
- Pause/Resume por botão e pela tecla ESC
- Sistema de save/load com persistência em JSON (`save/save.json`)
- HUD mínimo no runtime
- Overlay de menu principal quando a cena ativa é de menu

### Física e Colisão
- Detecção AABB (Axis-Aligned Bounding Box)
- Resolução por MTV (Minimum Translation Vector)
- Sistema de layers e masks por bits (8 layers)
- Raycast simples

### Scripting
Dois sistemas de script disponíveis:

**RS2 (script proprietário)** — simples, baseado em anotações:
```
@start_message "Player iniciado"
@player_controller 220
@camera_follow
```

**Lua 5.4** — completo e flexível:
```lua
function on_start()
    game.log("Entidade iniciada: " .. entity.name)
end

function on_update(dt)
    if input.key_held("right") then
        entity.set_velocity(150, entity.vy)
    end

    if input.key_pressed("space") and entity.grounded then
        entity.set_velocity(entity.vx, -400)
    end

    game.change_scene("assets/scenes/fase2.scene.json")
end
```

**API Lua disponível:**
- `entity.x/y`, `entity.vx/vy`, `entity.grounded`, `entity.visible`, `entity.name`
- `entity.set_position(x, y)`, `entity.set_velocity(vx, vy)`, `entity.set_rotation(r)`
- `entity.play_anim(clip_name)`, `entity.set_visible(bool)`
- `input.key_held(name)`, `input.key_pressed(name)`, `input.mouse_pos()`
- `game.delta_time`, `game.elapsed_time`, `game.log(msg)`, `game.change_scene(path)`
- `save.set(key, value)`, `save.get(key)`, `save.has(key)`, `save.remove(key)`

### Assets Suportados
| Tipo | Extensões |
|---|---|
| Cenas | `.scene.json` |
| Prefabs | `.prefab.json` |
| Scripts | `.rs2`, `.lua` |
| Texturas | `.png`, `.jpg`, `.jpeg`, `.webp` |
| Áudio | `.wav`, `.ogg`, `.mp3` |
| Fontes | `.ttf`, `.otf` |

---

## Como rodar

### Pré-requisitos
- [Rust](https://rustup.rs/) (edição 2021, stable)
- Cargo (incluso com o Rust)

### Compilar e executar
```bash
git clone https://github.com/seu-usuario/rs2br-engine.git
cd rs2br-engine
cargo run
```

Para build de release:
```bash
cargo build --release
./target/release/rs2br-engine
```

### Dependências principais (Cargo.toml)
| Crate | Uso |
|---|---|
| `eframe` + `egui` | UI do editor (nativo, puro Rust) |
| `macroquad` | Renderização 2D no runtime |
| `mlua` (Lua 5.4) | Runtime de scripts Lua |
| `serde` + `serde_json` | Serialização de cenas e saves |
| `rodio` | Reprodução de áudio |
| `uuid` | IDs únicos para entidades |
| `image` | Carregamento de texturas |

---

## Como testar

### Menu principal
1. Abra `assets/scenes/menu.scene.json`
2. Clique em **Play** no editor
3. O overlay **Menu Principal** deve aparecer
4. Clique em **Play** para carregar a primeira cena jogável
5. Clique em **Exit** para encerrar o runtime

### Fase jogável
1. Abra `assets/scenes/fase1.scene.json`
2. Clique em **Play**
3. O player aparece na cena com câmera seguindo

### Pause
- Durante o runtime: botão **Pause/Resume** ou tecla **ESC**
- O gameplay congela; os sistemas de input param de processar

### Troca de cena
- Use os botões de troca no runtime
- O estado entra em `Loading` antes de voltar para `Playing`

### Spawn e Destroy
- Clique em **Spawn Enemy** para criar uma entidade via fila
- Clique em **Destroy Last Spawn** para remover a última entidade criada

---

## Exportar Standalone (Windows)

O menu **Build Standalone PC** gera uma pasta pronta para distribuir.

### Resultado para o jogador final

- O jogador executa apenas o `.exe` exportado.
- **Nao precisa instalar Rust, Cargo ou SDKs de desenvolvimento**.
- O jogo abre em modo standalone usando `project.json` + `assets` da pasta exportada.

### Modos de build no editor

1. **Runtime pre-compilado (recomendado para pipeline profissional)**
   - Coloque o executavel base da engine em:
     - `standalone_runtime/windows/rs2br-engine.exe` (raiz da engine), ou
     - configure `RS2BR_STANDALONE_RUNTIME_DIR` apontando para a pasta do runtime.
   - Nesse modo, o export **nao depende de Rust instalado** na maquina que exporta.

2. **Compilacao local via Cargo (fallback)**
   - Se nao houver runtime pre-compilado, o editor usa `cargo build --release`.
   - Exige Rust/Cargo instalados na maquina de export.

### Erros comuns e como resolver

- **"Rust/Cargo nao encontrado e nao ha runtime pre-compilado disponivel"**
  - Solucao: configurar runtime pre-compilado (`standalone_runtime/windows/`) ou instalar Rust.
- **Windows pedindo runtime C++ no PC do jogador**
  - Instalar [Microsoft Visual C++ Redistributable 2015-2022 (x64)](https://aka.ms/vs/17/release/vc_redist.x64.exe).

---

## Roadmap

### V0.9 — MVP (atual)
- [x] Editor visual completo com hierarquia, inspetor e asset browser
- [x] Runtime com loop de jogo, física básica e colisão AABB
- [x] Scripts RS2 e Lua 5.4
- [x] Sistema de save/load persistente
- [x] Múltiplas cenas com transição e loading
- [x] Spawn/Destroy por fila
- [x] Câmera 2D com follow
- [x] Áudio básico
- [x] Prefabs simples

### V1.0 — Estabilização
- [ ] Validação completa de build
- [ ] Prefab system completo (instâncias, override de propriedades)
- [ ] Animações por spritesheet com editor de clipes
- [ ] Tilemap simples
- [ ] Exportação standalone do jogo

### Pós V1.0 — Gêneros
- [ ] **Card Game**: sistema de mão, baralho, turnos e efeitos via Lua
- [ ] **Roguelite**: geração procedural de salas, itens aleatórios, progressão por run
- [ ] **Survivor-like**: spawn em horda, XP/level-up, pausa de seleção de habilidade
- [ ] Pathfinding básico (A*)
- [ ] UI system com canvas e âncoras
- [ ] Particle system

---

## Estrutura de um projeto de jogo

```
meu_jogo/
├── assets/
│   ├── scenes/
│   │   ├── menu.scene.json
│   │   ├── fase1.scene.json
│   │   └── gameover.scene.json
│   ├── scripts/
│   │   ├── player.lua
│   │   └── enemy.lua
│   ├── prefabs/
│   │   └── inimigo.prefab.json
│   ├── sprites/
│   └── sounds/
└── save/
    └── save.json   ← gerado automaticamente em runtime
```

---

## Contribuindo

O projeto está em desenvolvimento ativo. Contribuições são bem-vindas.

1. Fork o repositório
2. Crie sua branch: `git checkout -b feature/minha-feature`
3. Commit suas mudanças: `git commit -m 'feat: descrição'`
4. Push: `git push origin feature/minha-feature`
5. Abra um Pull Request

---

## Limitações conhecidas (V0.9)

- Spawn usa template interno simples, não prefab completo
- HUD e menu são overlays mínimos sem sistema de UI com âncoras
- Loading é textual (sem tela de loading com arte)
- Sem tilemap

---

## Licença

MIT — veja [LICENSE](LICENSE) para detalhes.

---

> Feito com Rust 🦀 no Brasil 🇧🇷
