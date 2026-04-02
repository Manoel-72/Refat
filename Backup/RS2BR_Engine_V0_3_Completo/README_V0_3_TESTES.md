# RS2BR-Engine V0.3 — Como testar

## 1. Testar Play / Pause / Stop
1. Abra o projeto e rode `cargo run`.
2. No editor, crie ou abra uma cena.
3. Clique em **Play**.
4. Clique em **Pause**.
5. Clique em **Stop**.

### Esperado
- Play inicia o runtime.
- Pause congela a simulação.
- Stop volta para o editor sem sujar a cena original.

## 2. Testar troca de cena pelo Scene Manager
1. Salve uma cena chamada `fase1.scene.json` em `assets/scenes`.
2. Crie um script em `assets/scripts/TesteInput.rs` com:

```rs
@on_key H change_scene fase1.scene.json
```
3. Adicione esse script a uma entidade da cena atual.
4. Clique em **Play**.
5. Aperte **H**.

### Esperado
- O runtime agenda a troca de cena.
- A cena muda para `fase1.scene.json`.
- O runtime não trava.

## 3. Testar recarregar cena
Use este script:

```rs
@on_key H reload_scene
```

### Esperado
- Ao apertar `H`, a cena atual reinicia.

## 4. Testar script de mensagem inicial
Use este script:

```rs
@start_message Teste da V0.3
```

### Esperado
- A mensagem aparece no console quando o runtime começa.

## 5. Testar movimentação por script
Use este script:

```rs
@move_x 40
@move_y 20
```

### Esperado
- A entidade se move durante o Play.

## 6. Testar camera follow
Use este script:

```rs
@camera_follow
@move_x 40
```

### Esperado
- A câmera principal acompanha a entidade.

## 7. Testar física básica
1. Adicione `RigidBody2D` a uma entidade.
2. Dê Play.

### Esperado
- A entidade cai.
- Ela para no chão definido pelo runtime.

## 8. Testar colisão básica
1. Crie duas entidades com `BoxCollider`.
2. Faça uma encostar na outra.

### Esperado
- Uma entidade não atravessa a outra livremente.

## Se der erro
Mande o log completo do `cargo run`.
