// ============================================================
//  editor/menubar.rs
//  Barra de menu superior — Arquivo, Cena, Ajuda
// ============================================================

use eframe::egui;

use crate::engine::{entity::Entity, scene::Scene};

use super::{EditorApp, EditorPlayState};

pub fn show(app: &mut EditorApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
        ui.vertical(|ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("📁 Arquivo", |ui| {
                    if ui.button("🔄 Restaurar Layout").clicked() {
                        app.layout = crate::editor::EditorLayout::default();
                        app.save_layout_to_disk();
                        ui.close_menu();
                    }

                    if ui.button("🆕 Nova Cena").clicked() {
                        app.push_undo_state();
                        let next_name = format!("Cena {}", app.open_scenes.len() + 1);
                        app.create_new_scene_tab(next_name);
                        ui.close_menu();
                    }

                    if ui.button("💾 Salvar Cena Atual").clicked() {
                        match app.save_active_scene() {
                            Ok(path) => {
                                app.status_msg = format!(
                                    "✅ Cena salva em assets/scenes/{}",
                                    path.file_name().and_then(|n| n.to_str()).unwrap_or("cena.scene.json")
                                );
                            }
                            Err(error) => app.status_msg = format!("❌ {}", error),
                        }
                        ui.close_menu();
                    }

                    if ui.button("💾 Salvar Layout do Editor").clicked() {
                        app.save_layout_to_disk();
                        ui.close_menu();
                    }

                    ui.separator();
                    ui.label(egui::RichText::new("Carregar cena de assets/scenes").small().weak());
                    let scene_files = app.scene_file_candidates();
                    if scene_files.is_empty() {
                        ui.label("Nenhuma cena salva encontrada.");
                    } else {
                        for path in scene_files {
                            let label = path
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .map(|name| name.replace(".scene", ""))
                                .unwrap_or_else(|| "Cena".to_string());
                            if ui.button(format!("🎬 {}", label)).clicked() {
                                if let Err(error) = app.open_scene_from_path(path.clone()) {
                                    app.status_msg = format!("❌ {}", error);
                                }
                                ui.close_menu();
                            }
                        }
                    }

                    ui.separator();

                    if ui.button("🚪 Sair").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("🎬 Cena", |ui| {
                    ui.label(format!("Cenas abertas: {}", app.open_scenes.len()));
                    ui.label(format!("Cena ativa: {}", app.scene.name));
                    ui.separator();

                    if ui.button("➕ Adicionar Entidade").clicked() {
                        app.new_entity_dialog = Some("Nova Entidade".to_string());
                        ui.close_menu();
                    }

                    ui.separator();
                    ui.label(egui::RichText::new("Cor de Fundo").weak());
                    let bg = &mut app.scene.background_color;
                    ui.horizontal(|ui| {
                        ui.label("R:");
                        ui.add(egui::Slider::new(&mut bg[0], 0.0..=1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("G:");
                        ui.add(egui::Slider::new(&mut bg[1], 0.0..=1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("B:");
                        ui.add(egui::Slider::new(&mut bg[2], 0.0..=1.0));
                    });
                });

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

                ui.menu_button("❓ Ajuda", |ui| {
                    ui.label("Rust2D Engine v0.2.0");
                    ui.separator();
                    ui.label("Atalhos:");
                    ui.label("• Ctrl + Z / Ctrl + Y = desfazer / refazer");
                    ui.label("• Delete = excluir com confirmação");
                    ui.label("• F2 = renomear entidade/asset");
                    ui.label("• Scenes ficam em assets/scenes");
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⏹ Parar").clicked() {
                        app.play_state = EditorPlayState::Edit;
                        app.runtime.stop();
                        app.runtime.window_open = false;
                        app.status_msg = "⏹ Runtime parado. Retorno ao modo de edição.".to_string();
                    }
                    if ui
                        .add_enabled(
                            app.play_state == EditorPlayState::Playing,
                            egui::Button::new("⏸ Pause"),
                        )
                        .clicked()
                    {
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
                        egui::RichText::new(format!(
                            "🌐 {} | abertas: {}",
                            app.scene.name,
                            app.open_scenes.len()
                        ))
                        .color(egui::Color32::from_rgb(180, 200, 255)),
                    );
                });
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Cenas abertas:").small().weak());
                let tabs: Vec<(usize, String)> = app
                    .open_scenes
                    .iter()
                    .enumerate()
                    .map(|(index, doc)| (index, doc.display_name()))
                    .collect();

                for (index, label) in tabs {
                    let selected = app.active_scene_index == index;
                    if ui
                        .selectable_label(selected, format!("🎬 {}", label))
                        .clicked()
                    {
                        app.activate_scene_tab(index);
                    }

                    if app.open_scenes.len() > 1 && ui.small_button("✕").clicked() {
                        app.close_scene_tab(index);
                        break;
                    }
                }
            });

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

#[allow(dead_code)]
fn _load_scene_from_content(content: &str) -> Option<Scene> {
    Scene::from_json(content)
}
