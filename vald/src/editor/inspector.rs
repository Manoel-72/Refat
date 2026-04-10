// ============================================================
//  editor/inspector.rs
//  Painel Inspector — exibe e edita componentes da entidade
//  selecionada na hierarquia
// ============================================================

use eframe::egui;
use crate::{
    assets::is_rs2_script_file,
    component::{AnimationClip, Animator, Audio, BodyType, BoxCollider, Shape2D, UIButton, Camera2D, Component, LuaScript, RigidBody2D, Script, Sprite, TextLabel, Velocity},
};
use crate::runtime::script::is_valid_rs2_script;
use super::{warnings::EditorWarningSeverity, EditorApp};

pub fn show(app: &mut EditorApp, ui: &mut egui::Ui) {
    // ── Cabeçalho do Inspector ──
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(22, 27, 34))
        .inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 6.0, bottom: 4.0 })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔍 Inspector").strong().size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.selected_entity_id.is_some() {
                        ui.label(
                            egui::RichText::new("entidade selecionada")
                                .small()
                                .color(egui::Color32::from_rgb(63, 185, 80)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("cena")
                                .small()
                                .color(egui::Color32::from_rgb(88, 166, 255)),
                        );
                    }
                });
            });
        });

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
            // ── Propriedades da Cena (nenhuma entidade selecionada) ──
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(28, 34, 44))
                .rounding(6.0)
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("🎬 Propriedades da Cena").strong().size(13.0));
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(format!("📄 {}", app.scene.name))
                            .color(egui::Color32::from_rgb(88, 166, 255))
                    );
                });

            ui.add_space(8.0);

            ui.label(egui::RichText::new("🎨 Cor de Fundo").strong());
            ui.label(
                egui::RichText::new("Cor que aparece atrás de todos os objetos na cena")
                    .small()
                    .color(egui::Color32::from_rgb(100, 115, 135)),
            );
            ui.add_space(4.0);

            let bg = &mut app.scene.background_color;
            egui::Grid::new("scene_bg_color_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("R").color(egui::Color32::from_rgb(255, 100, 100)));
                    ui.add(egui::Slider::new(&mut bg[0], 0.0..=1.0).show_value(true));
                    ui.end_row();
                    ui.label(egui::RichText::new("G").color(egui::Color32::from_rgb(100, 220, 100)));
                    ui.add(egui::Slider::new(&mut bg[1], 0.0..=1.0).show_value(true));
                    ui.end_row();
                    ui.label(egui::RichText::new("B").color(egui::Color32::from_rgb(100, 140, 255)));
                    ui.add(egui::Slider::new(&mut bg[2], 0.0..=1.0).show_value(true));
                    ui.end_row();
                });

            // Preview da cor
            let preview_color = egui::Color32::from_rgb(
                (bg[0].clamp(0.0,1.0)*255.0) as u8,
                (bg[1].clamp(0.0,1.0)*255.0) as u8,
                (bg[2].clamp(0.0,1.0)*255.0) as u8,
            );
            let (preview_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 28.0), egui::Sense::hover()
            );
            ui.painter().rect_filled(preview_rect, 4.0, preview_color);

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(4.0);

            ui.label(
                egui::RichText::new(format!("📦 {} entidade(s) na cena", app.scene.entities.len()))
                    .color(egui::Color32::from_rgb(88, 166, 255))
            );

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(4.0);

            // Dica para iniciantes
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(20, 30, 46))
                .rounding(5.0)
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("💡 Como usar o Inspector").strong().small());
                    ui.add_space(3.0);
                    ui.label(egui::RichText::new("• Clique em uma entidade na Hierarquia (esquerda) para ver e editar suas propriedades aqui.").small().color(egui::Color32::from_rgb(139, 148, 158)));
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new("• Você pode adicionar componentes como Sprite, Script, Collider e muito mais.").small().color(egui::Color32::from_rgb(139, 148, 158)));
                });

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
                                    screen_space: false,
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
                                    hit_ceiling: false,
                                    hit_left: false,
                                    hit_right: false,
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
                        let mut radius = match &bc.shape {
                            Shape2D::Circle { radius } => *radius,
                            Shape2D::Box { .. } => (bc.width.min(bc.height) * 0.5).max(1.0),
                        };
                        let mut ox = bc.offset_x;
                        let mut oy = bc.offset_y;
                        let mut is_trigger = bc.is_trigger;
                        let mut collision_enabled = bc.collision_enabled;
                        let mut layer = bc.layer;
                        let mut mask = bc.mask;
                        let mut body_type = bc.body_type;
                        let mut one_way = bc.one_way;
                        let mut one_way_margin = bc.one_way_margin;
                        let mut use_circle = matches!(bc.shape, Shape2D::Circle { .. });
                        let mut changed = false;

                        egui::Grid::new(format!("bc_{}", i))
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Tipo de corpo:");
                                egui::ComboBox::from_id_source(format!("bc_body_type_{}", i))
                                    .selected_text(match body_type {
                                        BodyType::Static => "Static",
                                        BodyType::Kinematic => "Kinematic",
                                        BodyType::Trigger => "Trigger",
                                    })
                                    .show_ui(ui, |ui| {
                                        changed |= ui.selectable_value(&mut body_type, BodyType::Static, "Static").changed();
                                        changed |= ui.selectable_value(&mut body_type, BodyType::Kinematic, "Kinematic").changed();
                                        changed |= ui.selectable_value(&mut body_type, BodyType::Trigger, "Trigger").changed();
                                    });
                                ui.end_row();
                                ui.label("Shape:");
                                changed |= ui.checkbox(&mut use_circle, "Circle").changed();
                                ui.end_row();
                                if use_circle {
                                    ui.label("Raio:");
                                    changed |= ui.add(egui::DragValue::new(&mut radius).speed(0.5).range(1.0..=4096.0)).changed();
                                    ui.end_row();
                                } else {
                                    ui.label("Largura:");
                                    changed |= ui.add(egui::DragValue::new(&mut w).speed(0.5).range(1.0..=4096.0)).changed();
                                    ui.end_row();
                                    ui.label("Altura:");
                                    changed |= ui.add(egui::DragValue::new(&mut h).speed(0.5).range(1.0..=4096.0)).changed();
                                    ui.end_row();
                                }
                                ui.label("Offset X:");
                                changed |= ui.add(egui::DragValue::new(&mut ox).speed(0.5)).changed();
                                ui.end_row();
                                ui.label("Offset Y:");
                                changed |= ui.add(egui::DragValue::new(&mut oy).speed(0.5)).changed();
                                ui.end_row();
                                ui.label("Trigger legado:");
                                changed |= ui.checkbox(&mut is_trigger, "").changed();
                                ui.end_row();
                                ui.label("Colisão habilitada:");
                                changed |= ui.checkbox(&mut collision_enabled, "").changed();
                                ui.end_row();
                                ui.label("Layer:");
                                changed |= ui.add(egui::DragValue::new(&mut layer).speed(1.0).range(0..=u32::MAX)).changed();
                                ui.end_row();
                                ui.label("Mask:");
                                changed |= ui.add(egui::DragValue::new(&mut mask).speed(1.0).range(0..=u32::MAX)).changed();
                                ui.end_row();
                                ui.label("One-way platform:");
                                changed |= ui.checkbox(&mut one_way, "").changed();
                                ui.end_row();
                                ui.label("Margem one-way:");
                                changed |= ui.add(egui::DragValue::new(&mut one_way_margin).speed(0.25).range(0.0..=64.0)).changed();
                                ui.end_row();
                            });
                        if changed {
                            let shape = if use_circle {
                                Shape2D::Circle { radius: radius.max(1.0) }
                            } else {
                                Shape2D::Box { width: w.max(1.0), height: h.max(1.0) }
                            };
                            updated_components.push((
                                i,
                                Component::BoxCollider(BoxCollider {
                                    width: if use_circle { radius.max(1.0) * 2.0 } else { w.max(1.0) },
                                    height: if use_circle { radius.max(1.0) * 2.0 } else { h.max(1.0) },
                                    offset_x: ox,
                                    offset_y: oy,
                                    is_trigger,
                                    collision_enabled,
                                    layer,
                                    mask,
                                    body_type,
                                    shape,
                                    one_way,
                                    one_way_margin,
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

    ui.horizontal_wrapped(|ui| {
        let runtime_ativo = app.runtime.window_open;
        let stop_btn = ui.add_enabled(runtime_ativo && !path.trim().is_empty(), egui::Button::new("⏹ Stop por nome"));
        if stop_btn.clicked() {
            let stopped = app.runtime.stop_audio_by_name(&path);
            if stopped > 0 {
                app.status_msg = format!("⏹ {} áudio(s) parados para '{}'.", stopped, path);
            } else {
                app.status_msg = format!("ℹ Nenhum áudio ativo encontrado para '{}'.", path);
            }
        }

        if !runtime_ativo {
            ui.label(egui::RichText::new("Disponível durante o Play do runtime.").small().weak());
        }
    });

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
    use std::fs;

    // Lista todos os .lua em assets/scripts (igual ao RS2 faz para .rs2)
    let scripts_dir = app.project_root.join("assets/scripts");
    let mut lua_files: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&scripts_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("lua") {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    lua_files.push(name.to_string());
                }
            }
        }
    }
    lua_files.sort();

    let mut path = sc.file_path.clone();
    let mut changed = false;

    let popup_id = egui::Id::new(format!("select_lua_script_popup_{}_{}", index, scripts_dir.display()));

    ui.horizontal(|ui| {
        // ── botão Selecionar (popup igual ao RS2) ──
        let select_btn = ui.button("Selecionar LuaScript");
        if select_btn.clicked() {
            ui.memory_mut(|mem| mem.open_popup(popup_id));
        }

        egui::popup::popup_below_widget(
            ui,
            popup_id,
            &select_btn,
            egui::popup::PopupCloseBehavior::CloseOnClickOutside,
            |ui: &mut egui::Ui| {
                ui.set_min_width(220.0);
                ui.label("Selecione um script Lua:");
                if lua_files.is_empty() {
                    ui.label(egui::RichText::new("Nenhum .lua encontrado em assets/scripts").small().weak());
                }
                for script_name in &lua_files {
                    if ui.button(script_name).clicked() {
                        path = format!("assets/scripts/{}", script_name);
                        changed = true;
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
            },
        );

        // ── botão Criar ──
        if ui.button("Criar LuaScript").clicked() {
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
        ui.label(egui::RichText::new("Preparação V0.8.7: componente pronto para validação local de Lua e template base.").small().weak());
    } else if !crate::runtime::script::is_valid_lua_script(&path) {
        ui.colored_label(egui::Color32::YELLOW, "Use um arquivo com extensão .lua.");
    } else if let Err(error) = crate::runtime::script::validate_lua_script_reference(&app.project_root, &path) {
        ui.colored_label(egui::Color32::RED, error);
    } else {
        ui.colored_label(egui::Color32::GREEN, "LuaScript localizado.");
        let full_path = app.project_root.join(&path);
        match std::fs::read_to_string(&full_path) {
            Ok(source) => {
                let lua_errors = crate::runtime::script::validate_lua_source(&source);
                if lua_errors.is_empty() {
                    ui.label(egui::RichText::new("Sintaxe Lua OK via mlua.").small().weak());
                } else {
                    for error in lua_errors.iter().take(3) {
                        ui.colored_label(egui::Color32::RED, error);
                    }
                    if lua_errors.len() > 3 {
                        ui.label(egui::RichText::new(format!("+ {} erro(s) adicional(is)", lua_errors.len() - 3)).small().weak());
                    }
                }
            }
            Err(error) => {
                ui.colored_label(egui::Color32::RED, format!("Falha ao ler LuaScript: {}", error));
            }
        }
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
    let mut frames: Vec<String> = animator
        .clips
        .get(animator.current.as_str())
        .map(|clip| clip.frames.clone())
        .unwrap_or_default();
    let mut playing = animator.playing;
    let mut looped = animator.looped;
    let mut changed = false;

    // ── Configurações básicas ──────────────────────────────────
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(26, 32, 42))
        .rounding(5.0)
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            egui::Grid::new(format!("animator_cfg_{}", index))
                .num_columns(2)
                .spacing([12.0, 5.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Clip:").strong());
                    changed |= ui.text_edit_singleline(&mut current)
                        .on_hover_text("Nome do clip de animação atual")
                        .changed();
                    ui.end_row();

                    ui.label(egui::RichText::new("FPS:").strong());
                    ui.horizontal(|ui| {
                        changed |= ui.add(
                            egui::DragValue::new(&mut fps)
                                .speed(0.5)
                                .range(1.0..=60.0)
                                .suffix(" fps"),
                        ).on_hover_text("Quadros por segundo da animação").changed();
                        let duration = if fps > 0.0 { frames.len() as f32 / fps } else { 0.0 };
                        ui.label(
                            egui::RichText::new(format!("= {:.2}s", duration))
                                .small()
                                .color(egui::Color32::from_rgb(139, 148, 158)),
                        );
                    });
                    ui.end_row();

                    ui.label(egui::RichText::new("Playing:").strong());
                    changed |= ui.checkbox(&mut playing, "")
                        .on_hover_text("Animação tocando no runtime").changed();
                    ui.end_row();

                    ui.label(egui::RichText::new("Loop:").strong());
                    changed |= ui.checkbox(&mut looped, "")
                        .on_hover_text("Repete a animação ao chegar no fim").changed();
                    ui.end_row();
                });
        });

    ui.add_space(6.0);

    // ── Cabeçalho da lista de frames ──────────────────────────
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("Frames  ({})", frames.len()))
                .strong()
                .size(12.0),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(
                egui::Button::new(
                    egui::RichText::new("Ordenar A-Z").small()
                )
                .min_size(egui::vec2(80.0, 18.0))
            ).on_hover_text("Ordena os frames em ordem alfabética (útil quando os sprites têm numeração no nome)").clicked() {
                frames.sort();
                changed = true;
            }
            if ui.add(
                egui::Button::new(egui::RichText::new("Limpar").small())
                    .min_size(egui::vec2(54.0, 18.0))
            ).on_hover_text("Remove todos os frames do clip").clicked() {
                frames.clear();
                changed = true;
            }
        });
    });

    ui.label(
        egui::RichText::new("Cole paths de sprites abaixo, um por linha. Arraste sprites do Asset Browser para cá.")
            .small()
            .color(egui::Color32::from_rgb(100, 115, 135)),
    );

    // ── Lista visual de frames ─────────────────────────────────
    let frame_count = frames.len();
    let mut to_delete: Option<usize> = None;
    let mut to_move_up: Option<usize> = None;
    let mut to_move_down: Option<usize> = None;

    if frame_count == 0 {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(20, 26, 35))
            .rounding(4.0)
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Nenhum frame. Cole paths abaixo ou use o campo de texto.")
                            .small()
                            .color(egui::Color32::from_rgb(80, 95, 115)),
                    );
                });
            });
    } else {
        egui::ScrollArea::vertical()
            .id_source(format!("anim_frames_{}", index))
            .max_height(180.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (i, frame) in frames.iter().enumerate() {
                    egui::Frame::none()
                        .fill(if i % 2 == 0 {
                            egui::Color32::from_rgb(24, 30, 40)
                        } else {
                            egui::Color32::from_rgb(28, 35, 46)
                        })
                        .rounding(3.0)
                        .inner_margin(egui::Margin { left: 8.0, right: 4.0, top: 2.0, bottom: 2.0 })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Número do frame
                                ui.label(
                                    egui::RichText::new(format!("{:>2}.", i + 1))
                                        .small()
                                        .monospace()
                                        .color(egui::Color32::from_rgb(88, 166, 255)),
                                );
                                // Nome do arquivo (só o final do path)
                                let display = frame
                                    .split(['/', '\\'])
                                    .last()
                                    .unwrap_or(frame.as_str());
                                ui.label(
                                    egui::RichText::new(display)
                                        .small()
                                        .color(egui::Color32::from_rgb(210, 220, 235)),
                                )
                                .on_hover_text(frame.as_str());

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.add(
                                        egui::Button::new(egui::RichText::new("X").small().color(egui::Color32::from_rgb(200, 80, 80)))
                                            .min_size(egui::vec2(18.0, 16.0))
                                            .frame(false)
                                    ).on_hover_text("Remover este frame").clicked() {
                                        to_delete = Some(i);
                                    }
                                    if i + 1 < frame_count {
                                        if ui.add(
                                            egui::Button::new(egui::RichText::new("v").small())
                                                .min_size(egui::vec2(16.0, 16.0))
                                                .frame(false)
                                        ).on_hover_text("Mover para baixo").clicked() {
                                            to_move_down = Some(i);
                                        }
                                    }
                                    if i > 0 {
                                        if ui.add(
                                            egui::Button::new(egui::RichText::new("^").small())
                                                .min_size(egui::vec2(16.0, 16.0))
                                                .frame(false)
                                        ).on_hover_text("Mover para cima").clicked() {
                                            to_move_up = Some(i);
                                        }
                                    }
                                });
                            });
                        });
                }
            });
    }

    // Aplica ações da lista
    if let Some(i) = to_delete {
        frames.remove(i);
        changed = true;
    }
    if let Some(i) = to_move_up {
        frames.swap(i, i - 1);
        changed = true;
    }
    if let Some(i) = to_move_down {
        frames.swap(i, i + 1);
        changed = true;
    }

    // ── Campo de texto para adicionar/colar frames ─────────────
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Adicionar frames (cole paths, um por linha):").small().strong());

    let paste_id = egui::Id::new(format!("anim_paste_{}", index));
    let mut paste_buf = ui.data(|d| d.get_temp::<String>(paste_id).unwrap_or_default());

    let resp = ui.add(
        egui::TextEdit::multiline(&mut paste_buf)
            .desired_rows(3)
            .hint_text("sprites/player/run_01.png\nsprites/player/run_02.png")
            .font(egui::TextStyle::Monospace),
    );

    if resp.changed() {
        ui.data_mut(|d| d.insert_temp(paste_id, paste_buf.clone()));
    }

    ui.horizontal(|ui| {
        if ui.button("Adicionar").on_hover_text("Adiciona os paths digitados acima à lista de frames").clicked() {
            let new_frames = normalize_animator_frames(&paste_buf);
            if !new_frames.is_empty() {
                frames.extend(new_frames);
                paste_buf.clear();
                ui.data_mut(|d| d.insert_temp(paste_id, String::new()));
                changed = true;
            }
        }
        if ui.button("Substituir tudo").on_hover_text("Substitui toda a lista pelos paths digitados acima").clicked() {
            let new_frames = normalize_animator_frames(&paste_buf);
            frames = new_frames;
            paste_buf.clear();
            ui.data_mut(|d| d.insert_temp(paste_id, String::new()));
            changed = true;
        }
    });

    // ── Aplica mudanças ────────────────────────────────────────
    if changed {
        let mut clips = std::collections::HashMap::new();
        clips.insert(current.clone(), AnimationClip { frames, fps });
        let preserved_timer = animator.timer;
        let preserved_prev_clip = animator.prev_clip.clone();
        updated_components.push((
            index,
            Component::Animator(Animator {
                clips,
                current,
                timer: preserved_timer,
                playing,
                looped,
                prev_clip: preserved_prev_clip,
            }),
        ));
    }
}

fn normalize_animator_frames(raw: &str) -> Vec<String> {
    raw.replace(['\r', ';', ','], "\n")
        .lines()
        .flat_map(|line| {
            let trimmed = line.trim();
            if trimmed.contains("assets/") {
                trimmed.split_whitespace().map(str::to_string).collect::<Vec<_>>()
            } else {
                vec![trimmed.to_string()]
            }
        })
        .map(|line| line.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|line| !line.is_empty())
        .collect()
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
                label: text.clone(),
                action: target_scene.clone(),
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
