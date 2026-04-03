// ============================================================
//  editor/inspector.rs
//  Painel Inspector — exibe e edita componentes da entidade
//  selecionada na hierarquia
// ============================================================

use eframe::egui;
use crate::{
    assets::is_rs2_script_file,
    component::{BoxCollider, Camera2D, Component, RigidBody2D, Script, Sprite},
};
use crate::runtime::script::is_valid_rs2_script;
use super::{warnings::EditorWarningSeverity, EditorApp};

pub fn show(app: &mut EditorApp, ui: &mut egui::Ui) {
    ui.heading("🔍 Inspector");
    ui.separator();

    egui::ScrollArea::vertical()
        .id_source("inspector_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            show_inspector_contents(app, ui);
        });
}

fn show_inspector_contents(app: &mut EditorApp, ui: &mut egui::Ui) {
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

    let entity_data = {
        fn find_clone(
            entities: &[crate::core::entity::Entity],
            id: &str,
        ) -> Option<crate::core::entity::Entity> {
            for e in entities {
                if e.id == id {
                    return Some(e.clone());
                }
                if let Some(found) = find_clone(&e.children, id) {
                    return Some(found);
                }
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

    let runtime_warnings = app.collect_runtime_warnings();
    if !runtime_warnings.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("⚠ Diagnóstico da cena").strong());
            for warning in runtime_warnings.iter().take(6) {
                let (icon, color) = match warning.severity {
                    EditorWarningSeverity::Info => ("ℹ", egui::Color32::from_rgb(120, 180, 255)),
                    EditorWarningSeverity::Warning => ("⚠", egui::Color32::YELLOW),
                    EditorWarningSeverity::Error => ("❌", egui::Color32::from_rgb(255, 120, 120)),
                };
                ui.colored_label(color, format!("{} {}", icon, warning.label));
                ui.label(egui::RichText::new(&warning.details).small().weak());
                ui.add_space(4.0);
            }
            if runtime_warnings.len() > 6 {
                ui.label(egui::RichText::new(format!("+ {} aviso(s) adicional(is)", runtime_warnings.len() - 6)).small().weak());
            }
        });
        ui.add_space(6.0);
    }

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
            if ui.button("🧱 Exportar MATR").clicked() {
                match app.export_entity_as_matr(&entity) {
                    Ok(path) => {
                        app.status_msg = format!(
                            "🧱 MATR exportado: {}",
                            path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("arquivo.matr.json")
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
                                changed |= ui
                                    .add(egui::DragValue::new(&mut rot).speed(0.5).suffix("°"))
                                    .changed();
                                ui.end_row();
                                ui.label("Escala X:");
                                changed |= ui.add(egui::DragValue::new(&mut sx).speed(0.01)).changed();
                                ui.end_row();
                                ui.label("Escala Y:");
                                changed |= ui.add(egui::DragValue::new(&mut sy).speed(0.01)).changed();
                                ui.end_row();
                            });
                        if changed {
                            updated_components.push((
                                i,
                                Component::Transform(crate::core::component::Transform {
                                    x: cx,
                                    y: cy,
                                    rotation: rot,
                                    scale_x: sx,
                                    scale_y: sy,
                                }),
                            ));
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
                            updated_components.push((
                                i,
                                Component::Sprite(Sprite {
                                    texture_path: path,
                                    color_r: cr,
                                    color_g: cg,
                                    color_b: cb,
                                    color_a: ca,
                                }),
                            ));
                        }
                    }

                    Component::Camera2D(cam) => {
                        let mut zoom = cam.zoom;
                        let mut is_main = cam.is_main;
                        let mut changed = false;

                        egui::Grid::new(format!("cam_{}", i))
                            .num_columns(2)
                            .spacing([8.0, 4.0])
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
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Gravidade:");
                                changed |= ui.add(egui::DragValue::new(&mut gravity).speed(0.1)).changed();
                                ui.end_row();
                                ui.label("Estático:");
                                changed |= ui.checkbox(&mut is_static, "").changed();
                                ui.end_row();
                            });
                        if changed {
                            updated_components.push((
                                i,
                                Component::RigidBody2D(RigidBody2D {
                                    gravity_scale: gravity,
                                    is_static,
                                }),
                            ));
                        }
                    }

                    Component::BoxCollider(bc) => {
                        let mut w = bc.width;
                        let mut h = bc.height;
                        let mut ox = bc.offset_x;
                        let mut oy = bc.offset_y;
                        let mut is_trigger = bc.is_trigger;
                        let mut changed = false;

                        egui::Grid::new(format!("bc_{}", i))
                            .num_columns(2)
                            .spacing([8.0, 4.0])
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
                                ui.label("Trigger:");
                                changed |= ui.checkbox(&mut is_trigger, "").changed();
                                ui.end_row();
                            });
                        if changed {
                            updated_components.push((
                                i,
                                Component::BoxCollider(BoxCollider {
                                    width: w,
                                    height: h,
                                    offset_x: ox,
                                    offset_y: oy,
                                    is_trigger,
                                }),
                            ));
                        }
                    }

                    Component::Script(sc) => {
                        draw_script_component_ui(app, ui, &selected_id, i, sc, &mut updated_components);
                    }
                }

                if i > 0 && ui.small_button("🗑 Remover").clicked() {
                    to_remove = Some(i);
                }
            });
    }

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

fn draw_script_component_ui(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    selected_id: &str,
    index: usize,
    sc: &Script,
    updated_components: &mut Vec<(usize, Component)>,
) {
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
    let mut changed = false;
    let mut drop_error: Option<String> = None;

    if let Some(internal_drag_path) = app.dragging_asset_path.clone() {
        if is_rs2_script_file(&internal_drag_path) && ui.rect_contains_pointer(ui.max_rect()) {
            let released = ui.ctx().input(|i| i.pointer.any_released());
            if released {
                path = relative_to_project_or_full(&app.project_root, &internal_drag_path);
                changed = true;
                app.status_msg = format!("📜 Script RS2 vinculado à entidade '{}'.", selected_id);
            }
        }
    }

    let popup_id = egui::Id::new(format!("select_rs2_script_popup_{}_{}", selected_id, index));

    ui.horizontal(|ui| {
        let select_btn = ui.button("Selecionar Script RS2");
        if select_btn.clicked() {
            ui.memory_mut(|mem| mem.open_popup(popup_id));
        }

        if ui.button("Criar Script RS2").clicked() {
            match app.assets.create_rs2_script_file(&scripts_dir, "novo_script") {
                Ok(new_path) => {
                    path = format!(
                        "assets/scripts/{}",
                        new_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("novo_script.rs2")
                    );
                    changed = true;

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
            popup_id,
            &select_btn,
            egui::popup::PopupCloseBehavior::CloseOnClickOutside,
            |ui: &mut egui::Ui| {
                ui.label("Selecione um script RS2:");
                for script_name in &script_files {
                    if ui.button(script_name).clicked() {
                        path = format!("assets/scripts/{}", script_name);
                        changed = true;
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
            },
        );
    });

    let dropped_files = ui.ctx().input(|i| i.raw.dropped_files.clone());
    for file in &dropped_files {
        if let Some(path_buf) = &file.path {
            if is_rs2_script_file(path_buf) && ui.rect_contains_pointer(ui.max_rect()) {
                path = relative_to_project_or_full(&app.project_root, path_buf);
                changed = true;
            } else if ui.rect_contains_pointer(ui.max_rect()) {
                drop_error = Some(
                    "Só é permitido arrastar arquivos .rs2 para o componente Script RS2.".to_string(),
                );
            }
        }
    }

    let field_frame = egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(6.0))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(70)));

    let field_response = field_frame
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Arquivo RS2").strong());
            ui.add_space(4.0);

            let text_resp = ui.add(
                egui::TextEdit::singleline(&mut path)
                    .hint_text("assets/scripts/meu_script.rs2")
                    .desired_width(f32::INFINITY),
            );
            changed |= text_resp.changed();

            let hovering_internal_rs2 = app
                .dragging_asset_path
                .as_ref()
                .map(|p| is_rs2_script_file(p) && ui.rect_contains_pointer(ui.max_rect()))
                .unwrap_or(false);

            if hovering_internal_rs2 {
                ui.colored_label(
                    egui::Color32::from_rgb(120, 200, 255),
                    "Solte aqui para vincular o script RS2 do painel Assets.",
                );
            } else {
                ui.label(
                    egui::RichText::new(
                        "Arraste um .rs2 do painel Assets ou do sistema para este campo.",
                    )
                    .small()
                    .weak(),
                );
            }

            if !path.is_empty() && !is_valid_rs2_script(&path) {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "Use um arquivo com extensão .rs2.",
                );
            } else if let Some(msg) = &drop_error {
                ui.colored_label(egui::Color32::RED, msg);
            }
        })
        .response;

    let highlight_internal = app
        .dragging_asset_path
        .as_ref()
        .map(|p| is_rs2_script_file(p) && field_response.hovered())
        .unwrap_or(false);

    let highlight_external = !dropped_files.is_empty() && field_response.hovered();
    if highlight_internal || highlight_external {
        ui.painter().rect_stroke(
            field_response.rect.expand(1.0),
            6.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 200, 255)),
        );
    }

    if changed {
        updated_components.push((
            index,
            Component::Script(Script {
                file_path: path.clone(),
            }),
        ));
    }

    if !path.trim().is_empty() {
        if let Some(script_full_path) = crate::runtime::script::resolve_script_path(&app.project_root, &path) {
            match std::fs::read_to_string(&script_full_path) {
                Ok(source) => {
                    let script_errors = crate::runtime::script::validate_script(&source);
                    if script_errors.is_empty() {
                        ui.colored_label(egui::Color32::GREEN, "Script validado com sucesso.");
                    } else {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            format!("{} erro(s) de validação encontrados.", script_errors.len()),
                        );
                        for error in script_errors.iter().take(4) {
                            ui.label(egui::RichText::new(error.display()).small().weak());
                        }
                        if script_errors.len() > 4 {
                            ui.label(
                                egui::RichText::new(format!("+ {} erro(s) adicional(is)", script_errors.len() - 4))
                                    .small()
                                    .weak(),
                            );
                        }
                    }
                }
                Err(_) => {
                    ui.colored_label(
                        egui::Color32::RED,
                        format!("Falha ao ler o script em '{}'.", script_full_path.display()),
                    );
                }
            }
        }
    }

    ui.label(
        egui::RichText::new(
            "Diretivas suportadas: @move_x, @move_y, @rotate_speed, @player_controller, @camera_follow, @start_message, @on_update e @on_collision.",
        )
        .small()
        .weak(),
    );
}

fn relative_to_project_or_full(project_root: &std::path::Path, path: &std::path::Path) -> String {
    if let Ok(rel) = path.strip_prefix(project_root) {
        rel.to_string_lossy().replace('\\', "/")
    } else {
        path.to_string_lossy().replace('\\', "/")
    }
}
