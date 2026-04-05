// ============================================================
//  editor/inspector.rs
//  Painel Inspector — exibe e edita componentes da entidade
//  selecionada na hierarquia
// ============================================================

use eframe::egui;
use crate::{
    assets::is_rs2_script_file,
    component::{AnimationClip, Animator, Audio, BoxCollider, UIButton, Camera2D, Component, LuaScript, RigidBody2D, Script, Sprite, TextLabel, Velocity},
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
                        let grounded = rb.grounded;
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
                                ui.label("Grounded:");
                                ui.label(if grounded { "Sim" } else { "Não" });
                                ui.end_row();
                            });
                        if changed {
                            updated_components.push((
                                i,
                                Component::RigidBody2D(RigidBody2D {
                                    gravity_scale: gravity,
                                    is_static,
                                    grounded,
                                }),
                            ));
                        }
                    }

                    Component::Velocity(vel) => {
                        let mut vx = vel.x;
                        let mut vy = vel.y;
                        let mut changed = false;

                        egui::Grid::new(format!("vel_{}", i))
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Vel X:");
                                changed |= ui.add(egui::DragValue::new(&mut vx).speed(1.0)).changed();
                                ui.end_row();
                                ui.label("Vel Y:");
                                changed |= ui.add(egui::DragValue::new(&mut vy).speed(1.0)).changed();
                                ui.end_row();
                            });
                        if changed {
                            updated_components.push((i, Component::Velocity(Velocity { x: vx, y: vy })));
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
                                    layer: 0,
                                    mask: 0,
                                }),
                            ));
                        }
                    }

                    Component::Script(sc) => {
                        draw_script_component_ui(app, ui, &selected_id, i, sc, &mut updated_components);
                    }

                    Component::LuaScript(sc) => {
                        draw_lua_script_component_ui(app, ui, i, sc, &mut updated_components);
                    }

                    Component::Animator(animator) => {
                        draw_animator_component_ui(ui, i, animator, &mut updated_components);
                    }

                    Component::TextLabel(label) => {
                        draw_text_label_component_ui(ui, i, label, &mut updated_components);
                    }

                    Component::UIButton(button) => {
                        draw_ui_button_component_ui(ui, i, button, &mut updated_components);
                    }

                    Component::Audio(audio) => {
                        draw_audio_component_ui(app, ui, i, audio, &mut updated_components);
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
        if ui.button("🌙 LuaScript").clicked() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.add_component(Component::LuaScript(LuaScript {
                    file_path: String::new(),
                }));
            }
        }
        if ui.button("🎞 Animator").clicked() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.add_component(Component::Animator(Animator::default()));
            }
        }
        if ui.button("🔤 TextLabel").clicked() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.add_component(Component::TextLabel(TextLabel::default()));
            }
        }
        if ui.button("🔘 UIButton").clicked() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.add_component(Component::UIButton(UIButton::default()));
            }
        }
        if ui.button("🔊 Audio").clicked() {
            if let Some(e) = app.find_entity_mut(&selected_id) {
                e.add_component(Component::Audio(Audio::default()));
            }
        }
    });
}


fn draw_audio_component_ui(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    index: usize,
    audio: &Audio,
    updated_components: &mut Vec<(usize, Component)>,
) {
    use std::fs;

    let candidate_dirs = [app.project_root.join("sounds"), app.project_root.join("assets/sounds")];
    let mut audio_files: Vec<String> = Vec::new();
    for dir in candidate_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase());
                let is_audio = matches!(ext.as_deref(), Some("wav") | Some("ogg") | Some("mp3"));
                if is_audio {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let prefix = path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("sounds");
                        audio_files.push(format!("{}/{}", prefix, name));
                    }
                }
            }
        }
    }
    audio_files.sort();
    audio_files.dedup();

    let mut path = audio.file_path.clone();
    let mut play_on_start = audio.play_on_start;
    let mut looped = audio.looped;
    let mut volume = audio.volume;
    let mut changed = false;

    let popup_id = egui::Id::new(format!("select_audio_popup_{}", index));

    ui.horizontal(|ui| {
        let select_btn = ui.button("Selecionar áudio");
        if select_btn.clicked() {
            ui.memory_mut(|mem| mem.open_popup(popup_id));
        }

        egui::popup::popup_below_widget(
            ui,
            popup_id,
            &select_btn,
            egui::popup::PopupCloseBehavior::CloseOnClickOutside,
            |ui: &mut egui::Ui| {
                ui.label("Selecione um arquivo de áudio:");
                for audio_name in &audio_files {
                    if ui.button(audio_name).clicked() {
                        path = audio_name.clone();
                        changed = true;
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
            },
        );
    });

    egui::Grid::new(format!("audio_{}", index))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Arquivo:");
            changed |= ui.text_edit_singleline(&mut path).changed();
            ui.end_row();
            ui.label("Play on start:");
            changed |= ui.checkbox(&mut play_on_start, "").changed();
            ui.end_row();
            ui.label("Loop:");
            changed |= ui.checkbox(&mut looped, "").changed();
            ui.end_row();
            ui.label("Volume:");
            changed |= ui.add(egui::Slider::new(&mut volume, 0.0..=1.5)).changed();
            ui.end_row();
        });

    if !path.trim().is_empty() && !crate::runtime::systems::audio_system::validate_audio_path(&app.project_root, &path) {
        ui.colored_label(egui::Color32::YELLOW, "Arquivo de áudio não encontrado no projeto.");
    } else if !path.trim().is_empty() {
        ui.colored_label(egui::Color32::GREEN, "Áudio válido para o runtime.");
    }

    if changed {
        updated_components.push((
            index,
            Component::Audio(Audio {
                file_path: path,
                play_on_start,
                looped,
                volume,
            }),
        ));
    }
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

fn draw_lua_script_component_ui(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    index: usize,
    sc: &LuaScript,
    updated_components: &mut Vec<(usize, Component)>,
) {
    let mut path = sc.file_path.clone();
    let mut changed = false;

    ui.horizontal(|ui| {
        if ui.button("Criar LuaScript").clicked() {
            let scripts_dir = app.project_root.join("assets/scripts");
            match app.assets.create_lua_script_file(&scripts_dir, "novo_script_lua") {
                Ok(new_path) => {
                    path = format!(
                        "assets/scripts/{}",
                        new_path.file_name().and_then(|n| n.to_str()).unwrap_or("novo_script_lua.lua")
                    );
                    changed = true;
                }
                Err(err) => {
                    app.status_msg = format!("❌ Erro ao criar LuaScript: {}", err);
                }
            }
        }
    });

    egui::Grid::new(format!("lua_script_{}", index))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Arquivo Lua:");
            changed |= ui
                .add(egui::TextEdit::singleline(&mut path).hint_text("assets/scripts/meu_script.lua").desired_width(f32::INFINITY))
                .changed();
            ui.end_row();
        });

    if changed {
        updated_components.push((index, Component::LuaScript(LuaScript { file_path: path.clone() })));
    }

    if path.trim().is_empty() {
        ui.label(egui::RichText::new("Preparação V0.8.5: componente pronto para a integração Lua da V0.9.").small().weak());
    } else if !crate::runtime::script::is_valid_lua_script(&path) {
        ui.colored_label(egui::Color32::YELLOW, "Use um arquivo com extensão .lua.");
    } else if let Err(error) = crate::runtime::script::validate_lua_script_reference(&app.project_root, &path) {
        ui.colored_label(egui::Color32::RED, error);
    } else {
        ui.colored_label(egui::Color32::GREEN, "LuaScript localizado. Execução Lua entra na V0.9.");
    }
}

fn draw_animator_component_ui(
    ui: &mut egui::Ui,
    index: usize,
    animator: &Animator,
    updated_components: &mut Vec<(usize, Component)>,
) {
    let mut current = animator.current.clone();
    let mut fps = animator
        .clips
        .get(animator.current.as_str())
        .map(|clip| clip.fps)
        .unwrap_or(8.0);
    let mut frames_text = animator
        .clips
        .get(animator.current.as_str())
        .map(|clip| clip.frames.join("
"))
        .unwrap_or_default();
    let mut playing = animator.playing;
    let mut looped = animator.looped;
    let mut changed = false;

    egui::Grid::new(format!("animator_{}", index))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Clip atual:");
            changed |= ui.text_edit_singleline(&mut current).changed();
            ui.end_row();
            ui.label("FPS:");
            changed |= ui.add(egui::DragValue::new(&mut fps).speed(0.25).range(1.0..=60.0)).changed();
            ui.end_row();
            ui.label("Playing:");
            changed |= ui.checkbox(&mut playing, "").changed();
            ui.end_row();
            ui.label("Loop:");
            changed |= ui.checkbox(&mut looped, "").changed();
            ui.end_row();
        });

    ui.label("Frames (1 por linha):");
    changed |= ui.add(egui::TextEdit::multiline(&mut frames_text).desired_rows(4)).changed();

    if changed {
        let frames = frames_text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect::<Vec<_>>();

        let mut clips = std::collections::HashMap::new();
        clips.insert(current.clone(), AnimationClip { frames, fps });

        updated_components.push((
            index,
            Component::Animator(Animator {
                clips,
                current,
                timer: 0.0,
                playing,
                looped,
            }),
        ));
    }
}

fn draw_text_label_component_ui(
    ui: &mut egui::Ui,
    index: usize,
    label: &TextLabel,
    updated_components: &mut Vec<(usize, Component)>,
) {
    let mut text = label.text.clone();
    let mut font_size = label.font_size;
    let mut screen_space = label.screen_space;
    let mut color = [label.color_r, label.color_g, label.color_b, label.color_a];
    let mut changed = false;

    egui::Grid::new(format!("text_label_{}", index))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Texto:");
            changed |= ui.text_edit_singleline(&mut text).changed();
            ui.end_row();
            ui.label("Font size:");
            changed |= ui.add(egui::DragValue::new(&mut font_size).speed(0.5).range(8.0..=96.0)).changed();
            ui.end_row();
            ui.label("Screen space:");
            changed |= ui.checkbox(&mut screen_space, "").changed();
            ui.end_row();
        });

    changed |= ui.color_edit_button_rgba_unmultiplied(&mut color).changed();

    if changed {
        updated_components.push((
            index,
            Component::TextLabel(TextLabel {
                text,
                font_size,
                color_r: color[0],
                color_g: color[1],
                color_b: color[2],
                color_a: color[3],
                screen_space,
            }),
        ));
    }
}

fn draw_ui_button_component_ui(
    ui: &mut egui::Ui,
    index: usize,
    button: &UIButton,
    updated_components: &mut Vec<(usize, Component)>,
) {
    let mut text = button.text.clone();
    let mut width = button.width;
    let mut height = button.height;
    let mut font_size = button.font_size;
    let mut target_scene = button.target_scene.clone();
    let mut close_runtime = button.close_runtime;
    let mut screen_space = button.screen_space;
    let mut color = [button.color_r, button.color_g, button.color_b, button.color_a];
    let mut text_color = [button.text_r, button.text_g, button.text_b, button.text_a];
    let mut changed = false;

    egui::Grid::new(format!("ui_button_{}", index))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Texto:");
            changed |= ui.text_edit_singleline(&mut text).changed();
            ui.end_row();
            ui.label("Largura:");
            changed |= ui.add(egui::DragValue::new(&mut width).speed(1.0).range(72.0..=800.0)).changed();
            ui.end_row();
            ui.label("Altura:");
            changed |= ui.add(egui::DragValue::new(&mut height).speed(1.0).range(28.0..=200.0)).changed();
            ui.end_row();
            ui.label("Font size:");
            changed |= ui.add(egui::DragValue::new(&mut font_size).speed(0.5).range(8.0..=72.0)).changed();
            ui.end_row();
            ui.label("Target scene:");
            changed |= ui.text_edit_singleline(&mut target_scene).changed();
            ui.end_row();
            ui.label("Fechar runtime:");
            changed |= ui.checkbox(&mut close_runtime, "").changed();
            ui.end_row();
            ui.label("Screen space:");
            changed |= ui.checkbox(&mut screen_space, "").changed();
            ui.end_row();
        });

    ui.label("Cor do botão:");
    changed |= ui.color_edit_button_rgba_unmultiplied(&mut color).changed();
    ui.label("Cor do texto:");
    changed |= ui.color_edit_button_rgba_unmultiplied(&mut text_color).changed();

    ui.add_space(6.0);
    ui.label("Uso rápido:");
    ui.small("• target_scene: assets/scenes/main.scene.json");
    ui.small("• close_runtime: fecha o Play e volta para o editor");
    ui.small("• screen_space: mantém o botão preso na tela, ideal para menu e HUD");

    if changed {
        updated_components.push((
            index,
            Component::UIButton(UIButton {
                text,
                width,
                height,
                font_size,
                target_scene,
                close_runtime,
                color_r: color[0],
                color_g: color[1],
                color_b: color[2],
                color_a: color[3],
                text_r: text_color[0],
                text_g: text_color[1],
                text_b: text_color[2],
                text_a: text_color[3],
                screen_space,
            }),
        ));
    }
}

fn relative_to_project_or_full(project_root: &std::path::Path, path: &std::path::Path) -> String {
    if let Ok(rel) = path.strip_prefix(project_root) {
        rel.to_string_lossy().replace('\\', "/")
    } else {
        path.to_string_lossy().replace('\\', "/")
    }
}
