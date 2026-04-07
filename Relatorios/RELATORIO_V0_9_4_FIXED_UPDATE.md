# RS2BR Engine V0.9.4 - Fixed Update

## Corrigido
- script HUD com strings Lua válidas
- patrulha do inimigo com clamp por faixa e dt limitado
- plataforma intermediária mais baixa para teste de pulo
- moeda reposicionada e oscilando menos
- prefab no Enter com spawn mais visível e partícula de confirmação
- UICanvas visual separado e mais legível
- entidade Emitter renomeada para ParticleSource para ficar clara
- cena principal atualizada para validar gameplay, UI e partículas

## Como testar
- Enter: spawnar inimigo prefab à frente do player
- encoste no inimigo: perde HP e ativa camera shake
- pegue a moeda: sobe score e dispara partículas
- observe o bloco ciano "Particle Source": emissor automático de partículas
- use Reset Scene para resetar tudo
