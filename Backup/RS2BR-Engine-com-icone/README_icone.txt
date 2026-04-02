Ícone aplicado na janela da engine e no executável Windows.

Arquivos adicionados:
- assets/icon/rs2br_engine_icon.png
- assets/icon/rs2br_engine_icon.ico
- build.rs

Como gerar o .exe com ícone:
1. cargo build --release
2. O executável será gerado em target/release/rs2br-engine.exe

Se o Windows não mostrar o ícone na hora, feche o Explorer ou reconstrua com cargo clean && cargo build --release.
