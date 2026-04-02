// ============================================================
//  editor/menubar.rs
//  Barra de menu superior — Arquivo, Cena, Ajuda
// ============================================================

use eframe::egui;
use crate::engine::{scene::Scene, entity::Entity};
use super::{EditorApp, EditorPlayState};

pub fn show(app: &mut EditorApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
        ui.vertical(|ui| {
            egui::menu::bar(ui, |ui| {
                // ── Menu Arquivo ──
                ui.menu_button("📁 Arquivo", |ui| {
                                        if ui.button("🔄 Restaurar Layout").clicked() {
                                            app.layout = crate::editor::EditorLayout::default();
                                            app.save_layout_to_disk();
                                            ui.close_menu();
                                        }
                    if ui.button("🆕 Nova Cena").clicked() {
                        app.push_undo_state();
                        app.scene = Scene::new("Nova Cena");
                        app.clear_entity_selection();
                        app.status_msg = "Nova cena criada.".to_string();
                        ui.close_menu();
                    }

                    if ui.button("💾 Salvar Cena").clicked() {
                        save_scene(app);
                        ui.close_menu();
                    }

                    if ui.button("💾 Salvar Layout do Editor").clicked() {
                        app.save_layout_to_disk();
                        ui.close_menu();
                    }

                    if ui.button("📂 Carregar Cena").clicked() {
                        load_scene(app);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("🚪 Sair").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                // ── Menu Cena ──
                ui.menu_button("🎬 Cena", |ui| {
                    if ui.button("➕ Adicionar Entidade").clicked() {
                        app.new_entity_dialog = Some("Nova Entidade".to_string());
                        ui.close_menu();
                    }

                    ui.separator();

                    ui.label(egui::RichText::new("Cor de Fundo").weak());
                    let bg = &mut app.scene.background_color;
                    ui.horizontal(|ui| {
                        ui.label("R:"); ui.add(egui::Slider::new(&mut bg[0], 0.0..=1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("G:"); ui.add(egui::Slider::new(&mut bg[1], 0.0..=1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("B:"); ui.add(egui::Slider::new(&mut bg[2], 0.0..=1.0));
                    });
                });

                // ── Menu Entidades rápidas ──
                ui.menu_button("🎮 Criar", |ui| {
                    if ui.button("🔷 Entidade Vazia").clicked() {
                        app.push_undo_state();
                        let e = Entity::new("Entidade");
                        let id = e.id.clone();
                        app.scene.add_entity(e);
                        app.select_single_entity(Some(id));
                        ui.close_menu();
                    }

                    if ui.button("🖼 Sprite").clicked() {
                        app.push_undo_state();
                        let mut e = Entity::new("Sprite");
                        e.add_component(crate::engine::component::Component::Sprite(
                            crate::engine::component::Sprite::default(),
                        ));
                        let id = e.id.clone();
                        app.scene.add_entity(e);
                        app.select_single_entity(Some(id));
                        ui.close_menu();
                    }

                    if ui.button("📷 Câmera").clicked() {
                        app.push_undo_state();
                        let mut e = Entity::new("Camera");
                        e.add_component(crate::engine::component::Component::Camera2D(
                            crate::engine::component::Camera2D::default(),
                        ));
                        let id = e.id.clone();
                        app.scene.add_entity(e);
                        app.select_single_entity(Some(id));
                        ui.close_menu();
                    }
                });

                // ── Menu Ajuda ──
                ui.menu_button("❓ Ajuda", |ui| {
                    ui.label("Rust2D Engine v0.1.0");
                    ui.separator();
                    ui.label("Atalhos:");
                    ui.label("• Clique numa entidade na hierarquia para selecionar");
                    ui.label("• Ctrl + clique = seleção múltipla");
                    ui.label("• Delete = excluir com confirmação");
                    ui.label("• F2 = renomear entidade/asset");
                    ui.label("• Ctrl + Z / Ctrl + Y = desfazer / refazer");
                    ui.label("• Arraste entidades no viewport");
                    ui.label("• Clique direito na hierarquia para criar itens");
                    ui.label("• Use o Inspector para editar componentes");
                });

                // Play/Pause/Parar alinhados à direita
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⏹ Parar").clicked() {
                        app.play_state = EditorPlayState::Edit;
                        app.runtime.stop();
                        app.runtime.window_open = false;
                        app.status_msg = "⏹ Runtime parado. Retorno ao modo de edição.".to_string();
                    }
                    if ui.add_enabled(
                        app.play_state == EditorPlayState::Playing,
                        egui::Button::new("⏸ Pause"),
                    ).clicked() {
                        app.play_state = EditorPlayState::Paused;
                        app.status_msg = "⏸ Simulação pausada.".to_string();
                    }
                    if ui.button("▶ Play").clicked() {
                        app.play_state = EditorPlayState::Playing;
                        app.runtime.window_open = true;
                        app.runtime.start_from_scene(&app.scene);
                        app.status_msg = "▶ Modo Play iniciado em janela separada.".to_string();
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("🌐 {}", app.scene.name))
                            .color(egui::Color32::from_rgb(180, 200, 255)),
                    );
                });
            });

            // Linha separada para Desfazer/Refazer
            ui.add_space(2.0);
            ui.horizontal_centered(|ui| {
                if ui.button("↶ Desfazer").clicked() {
                    app.undo_scene();
                }
                if ui.button("↷ Refazer").clicked() {
                    app.redo_scene();
                }
            });
        });
    });
}

/// Salva a cena atual como JSON na pasta /scenes
fn save_scene(app: &mut EditorApp) {
    let scenes_dir = app.project_root.join("scenes");
    let _ = std::fs::create_dir_all(&scenes_dir);

    let filename = format!("{}.json", sanitize(&app.scene.name));
    let path = scenes_dir.join(&filename);

    match std::fs::write(&path, app.scene.to_json()) {
        Ok(_) => app.status_msg = format!("✅ Cena salva em: scenes/{}", filename),
        Err(e) => app.status_msg = format!("❌ Erro ao salvar: {}", e),
    }
}

/// Carrega a primeira cena JSON encontrada na pasta /scenes
fn load_scene(app: &mut EditorApp) {
    let scenes_dir = app.project_root.join("scenes");

    let entry = std::fs::read_dir(&scenes_dir)
        .ok()
        .and_then(|mut d| d.find(|e| {
            e.as_ref().map(|e| {
                e.path().extension().and_then(|x| x.to_str()) == Some("json")
            }).unwrap_or(false)
        }))
        .and_then(|e| e.ok());

    match entry {
        Some(e) => {
            let content = std::fs::read_to_string(e.path()).unwrap_or_default();
            match Scene::from_json(&content) {
                Some(scene) => {
                    app.scene = scene;
                    app.selected_entity_id = None;
                    app.status_msg = format!("✅ Cena carregada: {}", app.scene.name);
                }
                None => app.status_msg = "❌ Arquivo de cena inválido.".to_string(),
            }
        }
        None => {
            app.status_msg = "❌ Nenhuma cena encontrada em /scenes.".to_string();
        }
    }
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}
