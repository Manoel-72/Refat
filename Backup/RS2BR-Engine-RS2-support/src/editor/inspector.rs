// ============================================================
//  editor/inspector.rs
//  Painel Inspector — exibe e edita componentes da entidade
//  selecionada na hierarquia
// ============================================================

use eframe::egui;
use crate::engine::{
    assets::is_rs2_script_file,
    component::{BoxCollider, Camera2D, Component, RigidBody2D, Script, Sprite},
};
use crate::runtime::script::is_valid_rs2_script;
use super::EditorApp;

pub fn show(app: &mut EditorApp, ui: &mut egui::Ui) {
    ui.heading("🔍 Inspector");
    ui.separator();

    if app.selected_entity_ids.len() > 1 {
        ui.label(
            egui::RichText::new(format!(
                "{} entidades selecionadas — o Inspector edita a principal.",
                app.selected_entity_ids.len()
            ))
            .small()
            .weak(),
        );
        ui.separator();
    }

    let selected_id = match &app.selected_entity_id {
        Some(id) => id.clone(),
        None => {
            ui.label("Nenhuma entidade selecionada.");
            return;
        }
    };

    // Coletar dados para edição (evitar borrow duplo)
    let entity_data = {
        fn find_clone(entities: &[crate::engine::entity::Entity], id: &str)
            -> Option<crate::engine::entity::Entity>
        {
            for e in entities {
                if e.id == id { return Some(e.clone()); }
                if let Some(found) = find_clone(&e.children, id) { return Some(found); }
            }
            None
        }
        find_clone(&app.scene.entities, &selected_id)
    };

    let entity = match entity_data {
        Some(e) => e,
        None => {
            ui.label("Entidade não encontrada.");
            return;
        }
    };

    // ── Nome / propriedades básicas da entidade ──
    ui.group(|ui| {
        ui.label("Nome:");
        let mut name = entity.name.clone();
        if ui.text_edit_singleline(&mut name).changed() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.name = name;
            }
        }

        ui.add_space(4.0);
        ui.label("ID:");
        ui.monospace(&entity.id);

        let mut visible = entity.visible;
        if ui.checkbox(&mut visible, "👁 Visível na cena").changed() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.visible = visible;
            }
        }

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if ui.button("� Exportar MATR").clicked() {
                match app.export_entity_as_matr(&entity) {
                    Ok(path) => {
                        app.status_msg = format!(
                            "🧱 MATR exportado: {}",
                            path.file_name().and_then(|n| n.to_str()).unwrap_or("arquivo.matr.json")
                        );
                    }
                    Err(e) => {
                        app.status_msg = format!("❌ Erro ao exportar MATR: {}", e);
                    }
                }
            }

            if ui.button("🗑 Excluir Entidade").clicked() {
                app.request_delete_entity(selected_id.clone(), entity.name.clone());
            }
        });
    });

    ui.add_space(4.0);

    // ── Componentes ──
    ui.label(egui::RichText::new("Componentes").strong());

    let components = entity.components.clone();
    let mut to_remove: Option<usize> = None;
    let mut updated_components: Vec<(usize, Component)> = Vec::new();

    for (i, component) in components.iter().enumerate() {
        let header_text = format!("⚙ {}", component.display_name());

        egui::CollapsingHeader::new(&header_text)
            .id_source(format!("comp_{}_{}_{}", selected_id, i, component.display_name()))
            .default_open(true)
            .show(ui, |ui| {
                match component {
                    Component::Transform(t) => {
                        let mut cx = t.x;
                        let mut cy = t.y;
                        let mut rot = t.rotation;
                        let mut sx = t.scale_x;
                        let mut sy = t.scale_y;

                        let mut changed = false;
                        egui::Grid::new(format!("transform_{}", i))
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("X:");
                                changed |= ui.add(egui::DragValue::new(&mut cx).speed(0.5)).changed();
                                ui.end_row();
                                ui.label("Y:");
                                changed |= ui.add(egui::DragValue::new(&mut cy).speed(0.5)).changed();
                                ui.end_row();
                                ui.label("Rotação:");
                                changed |= ui.add(egui::DragValue::new(&mut rot).speed(0.5).suffix("°")).changed();
                                ui.end_row();
                                ui.label("Escala X:");
                                changed |= ui.add(egui::DragValue::new(&mut sx).speed(0.01)).changed();
                                ui.end_row();
                                ui.label("Escala Y:");
                                changed |= ui.add(egui::DragValue::new(&mut sy).speed(0.01)).changed();
                                ui.end_row();
                            });
                        if changed {
                            updated_components.push((i, Component::Transform(
                                crate::engine::component::Transform {
                                    x: cx, y: cy, rotation: rot,
                                    scale_x: sx, scale_y: sy,
                                }
                            )));
                        }
                    }

                    Component::Sprite(s) => {
                        let mut path = s.texture_path.clone();
                        let mut cr = s.color_r;
                        let mut cg = s.color_g;
                        let mut cb = s.color_b;
                        let mut ca = s.color_a;
                        let mut changed = false;

                        egui::Grid::new(format!("sprite_{}", i))
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Textura:");
                                changed |= ui.text_edit_singleline(&mut path).changed();
                                ui.end_row();
                                ui.label("Cor R:");
                                changed |= ui.add(egui::Slider::new(&mut cr, 0.0..=1.0)).changed();
                                ui.end_row();
                                ui.label("Cor G:");
                                changed |= ui.add(egui::Slider::new(&mut cg, 0.0..=1.0)).changed();
                                ui.end_row();
                                ui.label("Cor B:");
                                changed |= ui.add(egui::Slider::new(&mut cb, 0.0..=1.0)).changed();
                                ui.end_row();
                                ui.label("Alfa:");
                                changed |= ui.add(egui::Slider::new(&mut ca, 0.0..=1.0)).changed();
                                ui.end_row();
                            });
                        if changed {
                            updated_components.push((i, Component::Sprite(Sprite {
                                texture_path: path,
                                color_r: cr, color_g: cg, color_b: cb, color_a: ca,
                            })));
                        }
                    }

                    Component::Camera2D(cam) => {
                        let mut zoom = cam.zoom;
                        let mut is_main = cam.is_main;
                        let mut changed = false;

                        egui::Grid::new(format!("cam_{}", i))
                            .num_columns(2).spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Zoom:");
                                changed |= ui.add(egui::DragValue::new(&mut zoom).speed(0.01)).changed();
                                ui.end_row();
                                ui.label("Câmera principal:");
                                changed |= ui.checkbox(&mut is_main, "").changed();
                                ui.end_row();
                            });
                        if changed {
                            updated_components.push((i, Component::Camera2D(Camera2D { zoom, is_main })));
                        }
                    }

                    Component::RigidBody2D(rb) => {
                        let mut gravity = rb.gravity_scale;
                        let mut is_static = rb.is_static;
                        let mut changed = false;

                        egui::Grid::new(format!("rb_{}", i))
                            .num_columns(2).spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Gravidade:");
                                changed |= ui.add(egui::DragValue::new(&mut gravity).speed(0.1)).changed();
                                ui.end_row();
                                ui.label("Estático:");
                                changed |= ui.checkbox(&mut is_static, "").changed();
                                ui.end_row();
                            });
                        if changed {
                            updated_components.push((i, Component::RigidBody2D(RigidBody2D {
                                gravity_scale: gravity,
                                is_static,
                            })));
                        }
                    }

                    Component::BoxCollider(bc) => {
                        let mut w = bc.width;
                        let mut h = bc.height;
                        let mut ox = bc.offset_x;
                        let mut oy = bc.offset_y;
                        let mut changed = false;

                        egui::Grid::new(format!("bc_{}", i))
                            .num_columns(2).spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Largura:");
                                changed |= ui.add(egui::DragValue::new(&mut w).speed(0.5)).changed();
                                ui.end_row();
                                ui.label("Altura:");
                                changed |= ui.add(egui::DragValue::new(&mut h).speed(0.5)).changed();
                                ui.end_row();
                                ui.label("Offset X:");
                                changed |= ui.add(egui::DragValue::new(&mut ox).speed(0.5)).changed();
                                ui.end_row();
                                ui.label("Offset Y:");
                                changed |= ui.add(egui::DragValue::new(&mut oy).speed(0.5)).changed();
                                ui.end_row();
                            });
                        if changed {
                            updated_components.push((i, Component::BoxCollider(BoxCollider {
                                width: w, height: h, offset_x: ox, offset_y: oy,
                            })));
                        }
                    }
                    Component::Script(sc) => {
                        use std::fs;

                        let scripts_dir = app.project_root.join("assets/scripts");
                        let mut script_files: Vec<String> = Vec::new();
                        if let Ok(entries) = fs::read_dir(&scripts_dir) {
                            for entry in entries.flatten() {
                                let script_path = entry.path();
                                if is_rs2_script_file(&script_path) {
                                    if let Some(name) = script_path.file_name().and_then(|n| n.to_str()) {
                                        script_files.push(name.to_string());
                                    }
                                }
                            }
                        }
                        script_files.sort();

                        let mut path = sc.file_path.clone();
                        let mut dropped = false;
                        let mut drop_error: Option<String> = None;
                        let ctx = ui.ctx();
                        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());

                        ui.horizontal(|ui| {
                            let select_btn = ui.button("Selecionar Script RS2");
                            if select_btn.clicked() {
                                ui.memory_mut(|mem| mem.open_popup("select_rs2_script_popup".into()));
                            }

                            if ui.button("Criar Script RS2").clicked() {
                                match app.assets.create_rs2_script_file(&scripts_dir, "novo_script") {
                                    Ok(new_path) => {
                                        path = format!(
                                            "assets/scripts/{}",
                                            new_path.file_name().and_then(|n| n.to_str()).unwrap_or("novo_script.rs2")
                                        );
                                        updated_components.push((
                                            i,
                                            Component::Script(Script {
                                                file_path: path.clone(),
                                            }),
                                        ));

                                        #[cfg(target_os = "windows")]
                                        {
                                            let _ = std::process::Command::new("cmd")
                                                .args(["/C", "code", &new_path.to_string_lossy()])
                                                .spawn();
                                        }
                                    }
                                    Err(err) => {
                                        drop_error = Some(format!("Erro ao criar script RS2: {}", err));
                                    }
                                }
                            }

                            egui::popup::popup_below_widget(
                                ui,
                                egui::Id::new("select_rs2_script_popup"),
                                &select_btn,
                                egui::popup::PopupCloseBehavior::CloseOnClickOutside,
                                |ui: &mut egui::Ui| {
                                    ui.label("Selecione um script RS2:");
                                    for script_name in &script_files {
                                        if ui.button(script_name).clicked() {
                                            path = format!("assets/scripts/{}", script_name);
                                            updated_components.push((
                                                i,
                                                Component::Script(Script {
                                                    file_path: path.clone(),
                                                }),
                                            ));
                                            ui.memory_mut(|mem| mem.close_popup());
                                        }
                                    }
                                },
                            );
                        });

                        for file in &dropped_files {
                            if let Some(path_buf) = &file.path {
                                if is_rs2_script_file(path_buf) {
                                    let rel_path = if let Ok(rel) = path_buf.strip_prefix(&app.project_root) {
                                        rel.to_string_lossy().replace('\\', "/")
                                    } else {
                                        path_buf.to_string_lossy().replace('\\', "/")
                                    };
                                    path = rel_path;
                                    dropped = true;
                                } else {
                                    drop_error = Some(
                                        "Só é permitido arrastar arquivos .rs2 para o componente Script RS2."
                                            .to_string(),
                                    );
                                }
                            }
                        }

                        let response = egui::Grid::new(format!("script_{}", i))
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Arquivo RS2:");
                                let text_resp = ui.text_edit_singleline(&mut path);
                                if text_resp.changed() || dropped {
                                    updated_components.push((
                                        i,
                                        Component::Script(Script {
                                            file_path: path.clone(),
                                        }),
                                    ));
                                }

                                if !path.is_empty() && !is_valid_rs2_script(&path) {
                                    ui.colored_label(
                                        egui::Color32::YELLOW,
                                        "Use um arquivo com extensão .rs2",
                                    );
                                } else if let Some(msg) = &drop_error {
                                    ui.colored_label(egui::Color32::RED, msg);
                                }
                                ui.end_row();
                            });

                        if !dropped_files.is_empty() {
                            ui.painter().rect_stroke(
                                response.response.rect,
                                4.0,
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 200, 255)),
                            );
                        }

                        ui.label(
                            egui::RichText::new(
                                "Dica: arraste um arquivo .rs2 para este campo ou digite o caminho. Use diretivas como @move_x, @move_y, @rotate_speed, @player_controller, @camera_follow e @on_collision.",
                            )
                            .small()
                            .weak(),
                        );
                    }
                }

                // Botão remover componente (não o Transform)
                if i > 0 {
                    if ui.small_button("🗑 Remover").clicked() {
                        to_remove = Some(i);
                    }
                }
            });
    }

    // Aplicar remoção / updates
    if let Some(idx) = to_remove {
        if let Some(e) = app.find_entity_mut(&selected_id) {
            e.remove_component(idx);
        }
    }
    for (idx, comp) in updated_components {
        if let Some(e) = app.find_entity_mut(&selected_id) {
            if idx < e.components.len() {
                e.components[idx] = comp;
            }
        }
    }

    ui.add_space(8.0);
    ui.separator();

    // ── Adicionar componente ──
    ui.label(egui::RichText::new("Adicionar Componente").strong());

    ui.horizontal_wrapped(|ui| {
        if ui.button("🖼 Sprite").clicked() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.add_component(Component::Sprite(Sprite::default()));
            }
        }
        if ui.button("📷 Camera2D").clicked() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.add_component(Component::Camera2D(Camera2D::default()));
            }
        }
        if ui.button("⚽ RigidBody2D").clicked() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.add_component(Component::RigidBody2D(RigidBody2D::default()));
            }
        }
        if ui.button("📐 BoxCollider").clicked() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.add_component(Component::BoxCollider(BoxCollider::default()));
            }
        }
        if ui.button("📜 Script RS2").clicked() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.add_component(Component::Script(Script {
                    file_path: String::new(),
                }));
            }
        }
    });
}
