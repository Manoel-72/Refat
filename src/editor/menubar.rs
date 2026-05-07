// ============================================================
//  editor/menubar.rs
//  Barra de menu superior — redesenhada v0.9 (inspirada no print)
//  Layout: [menus] [tabs de cena] [centro: Play/Pause/Stop] [info direita]
// ============================================================

use eframe::egui;

use crate::{entity::Entity, scene::Scene};

use super::{EditorApp, EditorPlayState};

// Cores do tema
const ACCENT: egui::Color32 = egui::Color32::from_rgb(88, 166, 255);
const WARN: egui::Color32 = egui::Color32::from_rgb(255, 180, 50);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(146, 158, 176);
const PLAY_BG: egui::Color32 = egui::Color32::from_rgb(35, 134, 54);
const PAUSE_BG: egui::Color32 = egui::Color32::from_rgb(88, 100, 50);
const STOP_BG: egui::Color32 = egui::Color32::from_rgb(110, 40, 40);

pub fn show(app: &mut EditorApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menubar")
        .frame(
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(22, 27, 34))
                .inner_margin(egui::Margin {
                    left: 4.0,
                    right: 4.0,
                    top: 2.0,
                    bottom: 0.0,
                }),
        )
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // ══════════════════════════════════════════════════
                // LINHA 1: Menus + Play Controls centrados + Info
                // ══════════════════════════════════════════════════
                egui::menu::bar(ui, |ui| {
                    // ── Menus esquerda ──
                    ui.menu_button("Arquivo", |ui| {
                        ui.set_min_width(240.0);

                        section_label(ui, "PROJETO");
                        if menu_item(
                            ui,
                            "🆕",
                            "Novo Projeto...",
                            "Cria uma pasta de projeto nova",
                        )
                        .clicked()
                        {
                            let base = std::env::current_dir()
                                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                .to_string_lossy()
                                .to_string();
                            app.new_project_dialog = Some(("MeuProjeto".to_string(), base));
                            ui.close_menu();
                        }
                        if menu_item(
                            ui,
                            "📂",
                            "Abrir Projeto...",
                            "Abre um projeto existente pelo caminho",
                        )
                        .clicked()
                        {
                            app.open_project_dialog = Some(String::new());
                            ui.close_menu();
                        }
                        if menu_item(
                            ui,
                            "📦",
                            "Criar Template Básico",
                            "Gera estrutura inicial de pastas e arquivos",
                        )
                        .clicked()
                        {
                            app.new_template_dialog = Some("MeuProjeto".to_string());
                            ui.close_menu();
                        }
                        if menu_item(
                            ui,
                            "🏠",
                            "Voltar ao Hub",
                            "Fecha o projeto e volta à tela inicial",
                        )
                        .clicked()
                        {
                            app.close_current_project_to_hub(ctx);
                            ui.close_menu();
                        }

                        ui.separator();
                        section_label(ui, "CENA");
                        if menu_item(
                            ui,
                            "🎬",
                            "Nova Cena",
                            "Cria uma cena vazia e abre em nova aba",
                        )
                        .clicked()
                        {
                            app.push_undo_state();
                            let next_name = format!("Cena {}", app.open_scenes.len() + 1);
                            app.create_new_scene_tab(next_name);
                            ui.close_menu();
                        }
                        if menu_item(
                            ui,
                            "💾",
                            "Salvar Cena  Ctrl+S",
                            "Salva a cena ativa em assets/scenes/",
                        )
                        .clicked()
                        {
                            match app.save_active_scene() {
                                Ok(path) => {
                                    app.status_msg = format!(
                                        "✅ Cena salva: {}",
                                        path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("cena.scene.json")
                                    );
                                }
                                Err(error) => app.status_msg = format!("❌ {}", error),
                            }
                            ui.close_menu();
                        }

                        let scene_files = app.scene_file_candidates();
                        if !scene_files.is_empty() {
                            ui.separator();
                            section_label(ui, "ABRIR CENA SALVA");
                            for path in scene_files {
                                let label = path
                                    .file_stem()
                                    .and_then(|n| n.to_str())
                                    .map(|name| name.replace(".scene", ""))
                                    .unwrap_or_else(|| "Cena".to_string());
                                if ui.button(format!("  🎬 {}", label)).clicked() {
                                    if let Err(error) = app.open_scene_from_path(path.clone()) {
                                        app.status_msg = format!("❌ {}", error);
                                    }
                                    ui.close_menu();
                                }
                            }
                        }

                        ui.separator();
                        if menu_item(
                            ui,
                            "🔄",
                            "Restaurar Layout",
                            "Volta o layout padrão dos painéis",
                        )
                        .clicked()
                        {
                            app.layout = crate::editor::EditorLayout::default();
                            app.save_layout_to_disk();
                            ui.close_menu();
                        }
                        if menu_item(
                            ui,
                            "💾",
                            "Salvar Layout",
                            "Salva o tamanho atual dos painéis",
                        )
                        .clicked()
                        {
                            app.save_layout_to_disk();
                            ui.close_menu();
                        }

                        ui.separator();
                        if menu_item(ui, "🚪", "Sair", "Fecha o editor").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });

                    ui.menu_button("Criar", |ui| {
                        ui.set_min_width(210.0);
                        section_label(ui, "ENTIDADES");
                        if menu_item(ui, "🔷", "Entidade Vazia", "Objeto básico sem componentes")
                            .clicked()
                        {
                            app.push_undo_state();
                            let e = Entity::new("Entidade");
                            let id = e.id.clone();
                            app.scene.add_entity(e);
                            app.select_single_entity(Some(id));
                            ui.close_menu();
                        }
                        if menu_item(
                            ui,
                            "🖼",
                            "Sprite",
                            "Entidade com componente Sprite para exibir imagens",
                        )
                        .clicked()
                        {
                            app.push_undo_state();
                            let mut e = Entity::new("Sprite");
                            e.add_component(crate::core::component::Component::Sprite(
                                crate::core::component::Sprite::default(),
                            ));
                            let id = e.id.clone();
                            app.scene.add_entity(e);
                            app.select_single_entity(Some(id));
                            ui.close_menu();
                        }
                        if menu_item(
                            ui,
                            "📷",
                            "Câmera",
                            "Define o ponto de visão do jogador na cena",
                        )
                        .clicked()
                        {
                            app.push_undo_state();
                            let mut e = Entity::new("Camera");
                            e.add_component(crate::core::component::Component::Camera2D(
                                crate::core::component::Camera2D::default(),
                            ));
                            let id = e.id.clone();
                            app.scene.add_entity(e);
                            app.select_single_entity(Some(id));
                            ui.close_menu();
                        }
                        if menu_item(ui, "🔤", "Texto UI", "Rótulo de texto na interface do jogo")
                            .clicked()
                        {
                            app.push_undo_state();
                            let mut e = Entity::new("Texto");
                            e.add_component(crate::core::component::Component::TextLabel(
                                crate::core::component::TextLabel::default(),
                            ));
                            let id = e.id.clone();
                            app.scene.add_entity(e);
                            app.select_single_entity(Some(id));
                            ui.close_menu();
                        }
                        if menu_item(
                            ui,
                            "🔘",
                            "Botão UI",
                            "Botão clicável para menus e interfaces",
                        )
                        .clicked()
                        {
                            app.push_undo_state();
                            let mut e = Entity::new("Botão");
                            e.add_component(crate::core::component::Component::UIButton(
                                crate::core::component::UIButton::default(),
                            ));
                            let id = e.id.clone();
                            app.scene.add_entity(e);
                            app.select_single_entity(Some(id));
                            ui.close_menu();
                        }

                        ui.separator();
                        section_label(ui, "CENA");
                        if menu_item(
                            ui,
                            "🗺",
                            "Tilemap (Tiled JSON)…",
                            "Adiciona referência na cena (Tiled); não cria entidade",
                        )
                        .clicked()
                        {
                            app.add_scene_tilemap_from_file_dialog();
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Cena", |ui| {
                        ui.set_min_width(220.0);
                        ui.label(
                            egui::RichText::new(format!("Cena ativa: {}", app.scene.name)).strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{} entidade(s) | {} aba(s) aberta(s)",
                                app.scene.entities.len(),
                                app.open_scenes.len()
                            ))
                            .small()
                            .color(TEXT_DIM),
                        );
                        ui.separator();

                        section_label(ui, "COR DE FUNDO");
                        let bg = &mut app.scene.background_color;
                        egui::Grid::new("menu_bg_grid")
                            .num_columns(2)
                            .spacing([6.0, 3.0])
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("R")
                                        .color(egui::Color32::from_rgb(255, 100, 100)),
                                );
                                ui.add(egui::Slider::new(&mut bg[0], 0.0..=1.0).show_value(true));
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("G")
                                        .color(egui::Color32::from_rgb(100, 220, 100)),
                                );
                                ui.add(egui::Slider::new(&mut bg[1], 0.0..=1.0).show_value(true));
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("B")
                                        .color(egui::Color32::from_rgb(100, 140, 255)),
                                );
                                ui.add(egui::Slider::new(&mut bg[2], 0.0..=1.0).show_value(true));
                                ui.end_row();
                            });
                        let preview = egui::Color32::from_rgb(
                            (bg[0] * 255.0) as u8,
                            (bg[1] * 255.0) as u8,
                            (bg[2] * 255.0) as u8,
                        );
                        let (r, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 20.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(r, 3.0, preview);

                        ui.separator();
                        section_label(ui, "TILEMAPS");
                        if menu_item(
                            ui,
                            "🗺",
                            "Adicionar tilemap à cena…",
                            "Escolhe um JSON do Tiled; lista em Inspector → cena e pré-visualização na viewport",
                        )
                        .clicked()
                        {
                            app.add_scene_tilemap_from_file_dialog();
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Editor", |ui| {
                        ui.set_min_width(210.0);
                        section_label(ui, "HISTÓRICO");
                        if menu_item(ui, "↶", "Desfazer  Ctrl+Z", "Desfaz a última ação").clicked()
                        {
                            app.undo_scene();
                            ui.close_menu();
                        }
                        if menu_item(ui, "↷", "Refazer  Ctrl+Y", "Refaz a ação desfeita").clicked()
                        {
                            app.redo_scene();
                            ui.close_menu();
                        }
                        ui.separator();
                        section_label(ui, "SELEÇÃO");
                        if menu_item(
                            ui,
                            "📋",
                            "Copiar Entidade  Ctrl+C",
                            "Copia a entidade selecionada",
                        )
                        .clicked()
                        {
                            if let Some(id) = &app.selected_entity_id.clone() {
                                if let Some(entity) = app.scene.find_entity(id).cloned() {
                                    app.entity_clipboard = Some(entity);
                                    app.status_msg = "📋 Entidade copiada.".to_string();
                                }
                            }
                            ui.close_menu();
                        }
                        if menu_item(
                            ui,
                            "📌",
                            "Colar Entidade  Ctrl+V",
                            "Cola a entidade copiada com novo ID",
                        )
                        .clicked()
                        {
                            if let Some(template) = app.entity_clipboard.clone() {
                                app.push_undo_state();
                                let mut cloned = template.clone();
                                cloned.id = uuid::Uuid::new_v4().to_string();
                                cloned.name = format!("{} (cópia)", template.name);
                                if let Some(t) = cloned.transform_mut() {
                                    t.x += 32.0;
                                    t.y -= 32.0;
                                }
                                let new_id = cloned.id.clone();
                                app.scene.add_entity(cloned);
                                app.select_single_entity(Some(new_id));
                                app.status_msg = format!("📋 '{}' colada.", template.name);
                            }
                            ui.close_menu();
                        }
                        if menu_item(
                            ui,
                            "🗑",
                            "Deletar Seleção  Del",
                            "Remove a entidade selecionada",
                        )
                        .clicked()
                        {
                            app.request_delete_selected();
                            ui.close_menu();
                        }

                        ui.separator();
                        section_label(ui, "CENA");
                        if menu_item(
                            ui,
                            "🗺",
                            "Adicionar tilemap à cena…",
                            "JSON do Tiled; mesma ação do menu Scene e do clique direito na viewport",
                        )
                        .clicked()
                        {
                            app.add_scene_tilemap_from_file_dialog();
                            ui.close_menu();
                        }

                        ui.separator();
                        section_label(ui, "BUILD");
                        if menu_item(
                            ui,
                            "🖥",
                            "Build Standalone PC",
                            "Compila a engine em release e empacota o projeto atual",
                        )
                        .clicked()
                        {
                            if let Err(error) = app.start_build_standalone_pc() {
                                app.status_msg = format!("❌ {}", error);
                            }
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Ajuda", |ui| {
                        ui.set_min_width(300.0);

                        ui.label(
                            egui::RichText::new(format!(
                                "{} {}",
                                crate::core::version::ENGINE_TITLE,
                                crate::core::version::ENGINE_VERSION
                            ))
                            .strong()
                            .color(ACCENT),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Status: {}",
                                crate::core::version::ENGINE_STATUS
                            ))
                            .small()
                            .color(TEXT_DIM),
                        );
                        ui.separator();

                        section_label(ui, "⌨ ATALHOS DE TECLADO");
                        hotkey_row(ui, "Ctrl + Z", "Desfazer última ação");
                        hotkey_row(ui, "Ctrl + Y", "Refazer ação desfeita");
                        hotkey_row(ui, "Ctrl + C", "Copiar entidade selecionada");
                        hotkey_row(ui, "Ctrl + V", "Colar entidade copiada");
                        hotkey_row(ui, "Delete", "Deletar seleção");
                        hotkey_row(ui, "F2", "Renomear entidade ou asset");
                        hotkey_row(ui, "Scroll", "Zoom na viewport da cena");
                        hotkey_row(ui, "Arraste", "Mover entidade na cena");
                        hotkey_row(ui, "Ctrl+Clique", "Seleção múltipla");

                        ui.separator();
                        section_label(ui, "🗂 ORGANIZAÇÃO DO PROJETO");
                        help_row(ui, "assets/scenes/", "Arquivos .scene.json — cenas do jogo");
                        help_row(ui, "assets/sprites/", "Imagens .png/.jpg para os sprites");
                        help_row(ui, "assets/scripts/", "Scripts .rs2 e .lua com a lógica");
                        help_row(ui, "assets/prefabs/", "Templates .prefab.json (MATR)");
                        help_row(ui, "assets/audio/", "Sons e músicas do jogo");

                        ui.separator();
                        section_label(ui, "🔰 PRIMEIROS PASSOS");
                        ui.label(
                            egui::RichText::new("  1. Crie ou abra um projeto no Hub").small(),
                        );
                        ui.label(
                            egui::RichText::new("  2. Use 'Criar' para adicionar entidades à cena")
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new("  3. Selecione uma entidade e veja o Inspector →")
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new("  4. Adicione componentes (Sprite, Script, etc.)")
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(
                                "  5. Salve a cena e pressione ▶ Executar para testar",
                            )
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new("  6. Duplo clique em script abre no VS Code")
                                .small(),
                        );

                        ui.separator();
                        section_label(ui, "💡 DICAS RÁPIDAS");
                        ui.label(
                            egui::RichText::new("  • Arraste imagens do Asset Browser para a cena")
                                .small()
                                .color(TEXT_DIM),
                        );
                        ui.label(
                            egui::RichText::new(
                                "  • Clique direito na Hierarquia para criar filhos",
                            )
                            .small()
                            .color(TEXT_DIM),
                        );
                        ui.label(
                            egui::RichText::new(
                                "  • MATR = prefab reutilizável em múltiplas cenas",
                            )
                            .small()
                            .color(TEXT_DIM),
                        );
                        ui.label(
                            egui::RichText::new(
                                "  • Inspector mostra as props da cena se nada selecionado",
                            )
                            .small()
                            .color(TEXT_DIM),
                        );
                        ui.label(
                            egui::RichText::new(
                                "  • Use Snap 32px para alinhar entidades na grade",
                            )
                            .small()
                            .color(TEXT_DIM),
                        );
                    });

                    // ── Centro: Executar / Pausar / Parar ──
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add_space((ui.available_width() * 0.34).clamp(80.0, 420.0));
                        ui.horizontal(|ui| {
                            let is_playing = app.play_state == EditorPlayState::Playing;
                            let is_paused = app.play_state == EditorPlayState::Paused;

                            // Executar
                            let play_btn = egui::Button::new(
                                egui::RichText::new("Executar")
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .fill(if is_playing {
                                PLAY_BG
                            } else {
                                egui::Color32::from_rgb(46, 52, 63)
                            })
                            .min_size(egui::vec2(72.0, 24.0));
                            if ui
                                .add(play_btn)
                                .on_hover_text("Inicia o jogo em janela separada (F5)")
                                .clicked()
                            {
                                app.play_state = EditorPlayState::Playing;
                                app.runtime.window_open = true;
                                app.sync_active_scene_document();
                                let active_doc =
                                    app.open_scenes.get(app.active_scene_index).cloned();
                                if let Some(doc) = active_doc {
                                    let label = doc.display_name();
                                    let source_path = doc.file_path.clone();
                                    app.runtime.start_from_document(
                                        &doc.scene,
                                        source_path,
                                        &app.project_root,
                                    );
                                    app.status_msg = format!("▶ Executando: '{}'", label);
                                } else {
                                    app.runtime.start_from_scene(&app.scene, &app.project_root);
                                    app.status_msg = "▶ Execucao iniciada.".to_string();
                                }
                            }

                            // Pausar
                            let pause_btn = egui::Button::new(
                                egui::RichText::new("Pausar").color(egui::Color32::WHITE),
                            )
                            .fill(if is_paused {
                                PAUSE_BG
                            } else {
                                egui::Color32::from_rgb(46, 52, 63)
                            })
                            .min_size(egui::vec2(72.0, 24.0));
                            if ui
                                .add_enabled(is_playing, pause_btn)
                                .on_hover_text("Pausa a simulação sem fechar o jogo")
                                .clicked()
                            {
                                app.play_state = EditorPlayState::Paused;
                                app.status_msg = "⏸ Pausado.".to_string();
                            }

                            // Parar
                            let stop_btn = egui::Button::new(
                                egui::RichText::new("Parar").color(egui::Color32::WHITE),
                            )
                            .fill(if !is_playing && !is_paused {
                                egui::Color32::from_rgb(46, 52, 63)
                            } else {
                                STOP_BG
                            })
                            .min_size(egui::vec2(72.0, 24.0));
                            if ui
                                .add(stop_btn)
                                .on_hover_text("Para o jogo e volta ao modo de edição")
                                .clicked()
                            {
                                app.play_state = EditorPlayState::Edit;
                                app.runtime.stop();
                                app.runtime.window_open = false;
                                app.status_msg = "⏹ Runtime parado.".to_string();
                            }
                        });
                        ui.add_space(14.0);
                    });

                    // ── Direita: Info da cena ──
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        let warn_count = app.warning_count();
                        if warn_count > 0 {
                            ui.label(
                                egui::RichText::new(format!("⚠ {}", warn_count))
                                    .color(WARN)
                                    .small(),
                            );
                            ui.separator();
                        }
                        ui.label(
                            egui::RichText::new(format!(
                                "{} | {} abas",
                                app.scene.name,
                                app.open_scenes.len()
                            ))
                            .color(TEXT_DIM)
                            .small(),
                        );
                    });
                });

                // ══════════════════════════════════════════════════
                // LINHA 2: Tabs de cenas abertas + Desfazer/Refazer
                // ══════════════════════════════════════════════════
                ui.horizontal(|ui| {
                    ui.add_space(4.0);

                    // Desfazer / Refazer compactos
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("< Desfazer").size(12.0))
                                .min_size(egui::vec2(52.0, 20.0)),
                        )
                        .on_hover_text("Desfazer (Ctrl+Z)")
                        .clicked()
                    {
                        app.undo_scene();
                    }
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("Refazer >").size(12.0))
                                .min_size(egui::vec2(52.0, 20.0)),
                        )
                        .on_hover_text("Refazer (Ctrl+Y)")
                        .clicked()
                    {
                        app.redo_scene();
                    }

                    ui.separator();

                    // Tabs
                    let tabs: Vec<(usize, String)> = app
                        .open_scenes
                        .iter()
                        .enumerate()
                        .map(|(index, doc)| (index, doc.display_name()))
                        .collect();

                    for (index, label) in tabs {
                        let selected = app.active_scene_index == index;
                        let tab_color = if selected { ACCENT } else { TEXT_DIM };
                        let tab_text = egui::RichText::new(label).color(tab_color).small();

                        if ui.selectable_label(selected, tab_text).clicked() {
                            app.activate_scene_tab(index);
                        }

                        if app.open_scenes.len() > 1 {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("[x]").color(TEXT_DIM).small(),
                                    )
                                    .min_size(egui::vec2(24.0, 16.0))
                                    .frame(false),
                                )
                                .on_hover_text("Fechar aba")
                                .clicked()
                            {
                                app.close_scene_tab(index);
                                break;
                            }
                        }
                        ui.add_space(2.0);
                    }

                    // Botão + para nova cena
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("[+]").color(TEXT_DIM).small())
                                .min_size(egui::vec2(28.0, 20.0))
                                .frame(false),
                        )
                        .on_hover_text("Nova Cena")
                        .clicked()
                    {
                        let next_name = format!("Cena {}", app.open_scenes.len() + 1);
                        app.create_new_scene_tab(next_name);
                    }
                });

                ui.add_space(2.0);
            });
        });
}

// ── Helpers de UI ────────────────────────────────────────────

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(egui::RichText::new(text).small().color(TEXT_DIM));
    ui.add_space(1.0);
}

fn menu_item(ui: &mut egui::Ui, icon: &str, label: &str, tooltip: &str) -> egui::Response {
    let resp = ui.button(format!("{}  {}", icon, label));
    resp.on_hover_text(tooltip)
}

fn hotkey_row(ui: &mut egui::Ui, key: &str, desc: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("  {:15}", key))
                .small()
                .color(ACCENT)
                .monospace(),
        );
        ui.label(egui::RichText::new(desc).small().color(TEXT_DIM));
    });
}

fn help_row(ui: &mut egui::Ui, path: &str, desc: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("  {:22}", path))
                .small()
                .color(egui::Color32::from_rgb(180, 210, 255))
                .monospace(),
        );
        ui.label(egui::RichText::new(desc).small().color(TEXT_DIM));
    });
}

#[allow(dead_code)]
fn _load_scene_from_content(content: &str) -> Option<Scene> {
    Scene::from_json(content)
}
