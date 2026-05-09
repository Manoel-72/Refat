# RS2 Runtime Icon Panic Fix

## Erro corrigido
O jogo compilava, mas ao abrir fechava com panic:

```text
Falha ao carregar ícone PNG da engine: Decoding(...)
```

## Causa real
O build estava criando/copiendo um `rs2br_engine_icon.png` inválido/corrompido.
O jogo tentava decodificar esse PNG na inicialização e derrubava o runtime antes da janela abrir.

## Correções aplicadas
- Substituído `assets/icon/rs2br_engine_icon.png` por um PNG 1x1 transparente válido.
- Corrigido o placeholder em `src/standalone_export.rs`.
- Removido uso perigoso de `include_bytes!` para esse ícone quando encontrado.
- Onde havia possibilidade de panic por ícone, foi convertido para fallback seguro.

## Arquivos Rust alterados
- src/standalone_export.rs

## Ícones corrigidos
- assets/icon/rs2br_engine_icon.png

## Importante
Depois de extrair esta versão, apague builds antigos/cache antigos antes de testar:

```bat
rmdir /s /q build
rmdir /s /q .rs2br_build_cache_Demo_Gameplay_*
cargo clean
cargo run
```

Ou gere o build novamente pelo editor.
