# RS2BR-Engine V0.5.5

## O que foi feito

- limpeza estrutural do runtime e do editor
- separação mais clara dos systems
- redução de lógica espalhada em `runtime/systems.rs`
- criação de `editor/warnings.rs`
- warnings visuais para:
  - cena sem câmera principal
  - sprite sem textura
  - textura ausente
  - script RS2 vazio
  - script RS2 inválido
  - prefab quebrado
- validação mais forte de scripts RS2
- validação mais forte no carregamento de cenas
- tratamento mais seguro para caminhos inválidos
- leitura de input centralizada em `input_system`
- `script_system` agora retorna uma struct clara em vez de uma tupla grande
- contador de warnings no rodapé do editor
- warnings resumidos no Inspector

## Direção técnica

Esta versão foca em estabilização da V0.5, sem trocar ícone nem expandir features grandes.
