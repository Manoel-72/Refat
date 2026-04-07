# Patch cirúrgico — runtime Lua V0.9.1

## O que foi corrigido
- scripts Lua carregados apenas uma vez por VM cacheada
- variáveis locais e closures agora persistem entre frames
- `event.listen(name, fn)` registra listeners persistentes na VM
- `event.emit(name, data)` entra em fila de eventos do runtime para o próximo frame
- `game.after(seconds, fn)` e `game.every(seconds, fn)` agora executam callbacks por frame
- callbacks de timer e eventos são protegidos com log de erro, sem derrubar o runtime inteiro
- limpeza de duplicação de `event.emit` por frame

## Escopo
Mudanças pequenas e locais, sem reescrever a engine.

## Limitação conhecida
- o bus de eventos continua leve: emissão neste frame é entregue no próximo frame
- a validação principal foi feita por inspeção de código; não consegui compilar no container porque `cargo` não está disponível neste ambiente
