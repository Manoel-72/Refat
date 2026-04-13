use eframe::egui;

use crate::{
    component::{AnimationClip, AnimationState, Animator, Component},
    editor::EditorApp,
};


pub fn show(app: &mut EditorApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_source("animator_root_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let Some(entity_id) = app.selected_entity_id.clone() else {
                show_empty_state(ui, "Selecione uma entidade com Animator para abrir esta aba.");
                return;
            };

            let Some(entity) = app.scene.find_entity(&entity_id).cloned() else {
                show_empty_state(ui, "Entidade selecionada não foi encontrada.");
                return;
            };

            let Some(animator_index) = entity
                .components
                .iter()
                .position(|component| matches!(component, Component::Animator(_)))
            else {
                show_empty_state(ui, "A entidade atual não possui componente Animator.");
                return;
            };

            let animator_snapshot = match entity.components.get(animator_index) {
                Some(Component::Animator(animator)) => animator.clone(),
                _ => {
                    show_empty_state(ui, "Não foi possível ler o Animator da entidade.");
                    return;
                }
            };

            let state_names = sorted_state_names(&animator_snapshot);
            ensure_node_layout(app, &state_names);

            if state_names.is_empty() {
                draw_header(ui, &entity.name, &animator_snapshot, "<nenhum>", "<nenhum>", app.animator_detached);
                ui.add_space(6.0);
                draw_help_bar(ui);
                ui.add_space(8.0);
                draw_empty_animator_setup(app, ui, &entity_id, animator_index);
                return;
            }

            if app.animator_selected_state.trim().is_empty()
                || !state_names.iter().any(|name| name == &app.animator_selected_state)
            {
                app.animator_selected_state = preferred_state_name(&animator_snapshot, &state_names);
            }

            let selected_state_name = app.animator_selected_state.clone();
            if app.animator_state_rename_buffer.trim().is_empty() {
                app.animator_state_rename_buffer = selected_state_name.clone();
            }
            let selected_state = animator_snapshot
                .states
                .get(&selected_state_name)
                .cloned()
                .unwrap_or_else(|| AnimationState {
                    clip: selected_state_name.clone(),
                    ..AnimationState::default()
                });
            let selected_clip_name = if selected_state.clip.trim().is_empty() {
                selected_state_name.clone()
            } else {
                selected_state.clip.clone()
            };
            let selected_clip = animator_snapshot
                .clips
                .get(&selected_clip_name)
                .cloned()
                .unwrap_or_default();

            draw_header(
                ui,
                &entity.name,
                &animator_snapshot,
                &selected_state_name,
                &selected_clip_name,
                app.animator_detached,
            );
            ui.add_space(6.0);
            draw_help_bar(ui);
            ui.add_space(8.0);

            let compact_layout = ui.available_width() < 980.0;
            if compact_layout {
                draw_state_sidebar(app, ui, &entity_id, animator_index, &animator_snapshot, &state_names);
                ui.add_space(8.0);
                draw_simple_workspace(
                    app,
                    ui,
                    &entity_id,
                    animator_index,
                    &animator_snapshot,
                    &state_names,
                    &selected_state_name,
                    &selected_state,
                    &selected_clip_name,
                    &selected_clip,
                );
            } else {
                ui.columns(2, |columns| {
                    draw_state_sidebar(app, &mut columns[0], &entity_id, animator_index, &animator_snapshot, &state_names);
                    draw_simple_workspace(
                        app,
                        &mut columns[1],
                        &entity_id,
                        animator_index,
                        &animator_snapshot,
                        &state_names,
                        &selected_state_name,
                        &selected_state,
                        &selected_clip_name,
                        &selected_clip,
                    );
                });
            }

        });
}

fn preferred_state_name(animator: &Animator, state_names: &[String]) -> String {
    if !animator.current_state.trim().is_empty() {
        return animator.current_state.clone();
    }
    if !animator.default_state.trim().is_empty() {
        return animator.default_state.clone();
    }
    state_names.first().cloned().unwrap_or_default()
}

fn draw_header(
    ui: &mut egui::Ui,
    entity_name: &str,
    _animator: &Animator,
    selected_state_name: &str,
    selected_clip_name: &str,
    detached: bool,
) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(21, 28, 38))
        .rounding(6.0)
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("🎞 Animator").strong().size(15.0));
                ui.separator();
                ui.label(format!("Entidade: {}", entity_name));
                ui.separator();
                ui.label(format!("Estado: {}", selected_state_name));
                ui.separator();
                ui.label(format!("Clip: {}", selected_clip_name));
                ui.separator();
                ui.label("Modo: Simples");
                ui.separator();
                ui.label(if detached { "Janela destacada" } else { "Na aba inferior" });
            });
        });
}

fn draw_help_bar(ui: &mut egui::Ui) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(18, 24, 32))
        .rounding(6.0)
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Modo simples").strong());
                ui.separator();
                ui.label("1. Crie um estado com o nome que quiser.");
                ui.separator();
                ui.label("2. Arraste sprites para 'Solte frames aqui'.");
                ui.separator();
                ui.label("3. Ajuste FPS, Loop e Próximo estado.");
                ui.separator();
                ui.label("4. Use 'Próximo estado' para fazer a transição de forma fácil.");
            });
        });
}

fn draw_state_sidebar(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    entity_id: &str,
    animator_index: usize,
    animator: &Animator,
    state_names: &[String],
) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(18, 24, 32))
        .rounding(6.0)
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.set_min_width(210.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Estados").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("+ Estado").clicked() {
                        let mut new_name = unique_state_name(animator, "novo");
                        update_animator(app, entity_id, animator_index, |animator| {
                            animator.states.insert(
                                new_name.clone(),
                                AnimationState {
                                    clip: new_name.clone(),
                                    looped: true,
                                    interruptible: true,
                                    next_state: String::new(),
                                },
                            );
                            animator.clips.entry(new_name.clone()).or_insert_with(AnimationClip::default);
                            animator.current_state = new_name.clone();
                            animator.state_mode = true;
                        });
                        let count = state_names.len().max(1) as f32;
                        let y = (0.18 + count * 0.12).min(0.82);
                        app.animator_node_positions.insert(new_name.clone(), [0.40, y]);
                        app.animator_selected_state = std::mem::take(&mut new_name);
                        app.animator_state_rename_buffer = app.animator_selected_state.clone();
                        app.status_msg = format!("✅ Estado '{}' criado.", app.animator_selected_state);
                    }
                });
            });
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .id_source("animator_state_sidebar")
                .max_height(260.0)
                .show(ui, |ui| {
                    for state_name in state_names {
                        ui.horizontal(|ui| {
                            let selected = app.animator_selected_state == *state_name;
                            let response = ui.add_sized(
                                [ui.available_width() - 30.0, 24.0],
                                egui::SelectableLabel::new(selected, state_name),
                            );
                            if response.clicked() {
                                app.animator_selected_state = state_name.clone();
                                app.animator_state_rename_buffer = state_name.clone();
                            }
                            if response.double_clicked() {
                                let target = state_name.clone();
                                update_animator(app, entity_id, animator_index, move |animator| {
                                    animator.current_state = target.clone();
                                    if let Some(state) = animator.states.get(&target) {
                                        animator.current = state.clip.clone();
                                        animator.looped = state.looped;
                                    }
                                });
                            }

                            let delete_clicked = ui
                                .add_sized(
                                    [24.0, 24.0],
                                    egui::Button::new(egui::RichText::new("🗑").size(12.0)),
                                )
                                .on_hover_text("Deletar estado")
                                .clicked();
                            if delete_clicked {
                                delete_state(app, entity_id, animator_index, state_name);
                            }

                            let source_name = state_name.clone();
                            response.context_menu(|ui| {
                                ui.label(format!("Estado: {}", source_name));
                                ui.separator();
                                if ui.button("Limpar ligação → saída").clicked() {
                                    let source = source_name.clone();
                                    update_animator(app, entity_id, animator_index, move |animator| {
                                        if let Some(state) = animator.states.get_mut(&source) {
                                            state.next_state.clear();
                                        }
                                    });
                                    ui.close_menu();
                                }
                                if ui.button("🗑 Deletar estado").clicked() {
                                    delete_state(app, entity_id, animator_index, &source_name);
                                    ui.close_menu();
                                }
                                ui.separator();
                                ui.label("Ligar com:");
                                for target_name in state_names {
                                    if target_name != &source_name
                                        && ui.button(format!("→ {}", target_name)).clicked()
                                    {
                                        let source = source_name.clone();
                                        let target = target_name.clone();
                                        update_animator(app, entity_id, animator_index, move |animator| {
                                            if let Some(state) = animator.states.get_mut(&source) {
                                                state.next_state = target.clone();
                                            }
                                        });
                                        ui.close_menu();
                                    }
                                }
                            });
                        });
                    }
                });

            ui.separator();
            ui.label(egui::RichText::new("Ferramentas rápidas").small().strong());
            if ui.small_button("Sincronizar estados pelos clips").clicked() {
                update_animator(app, entity_id, animator_index, |animator| {
                    let mut clip_names: Vec<String> = animator.clips.keys().cloned().collect();
                    clip_names.sort();
                    for clip_name in clip_names {
                        animator.states.entry(clip_name.clone()).or_insert_with(|| AnimationState {
                            clip: clip_name.clone(),
                            looped: clip_name != "attack",
                            interruptible: clip_name != "attack",
                            next_state: if clip_name == "attack" { "idle".to_string() } else { String::new() },
                        });
                    }
                    animator.state_mode = true;
                });
                ensure_node_layout(app, &sorted_state_names(animator));
            }

            ui.horizontal_wrapped(|ui| {
                ui.label("Zoom:");
                ui.add(egui::Slider::new(&mut app.animator_graph_zoom, 0.55..=2.4).clamp_to_range(true));
            });

            if ui.small_button("Reorganizar nós").clicked() {
                app.animator_node_positions.clear();
                ensure_node_layout(app, state_names);
            }
            if ui.small_button("Abrir em janela").clicked() {
                app.animator_detached = true;
            }
            if ui.small_button("Ativar state mode").clicked() {
                update_animator(app, entity_id, animator_index, |animator| {
                    animator.state_mode = true;
                });
            }

            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "Dica: arraste a bolinha amarela da direita até outro nó ou até a Saída.",
                )
                .small()
                .weak(),
            );
        });
}


fn draw_empty_animator_setup(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    entity_id: &str,
    animator_index: usize,
) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(16, 21, 30))
        .rounding(6.0)
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Nenhum estado criado ainda").strong());
            ui.add_space(6.0);
            ui.label("Clique no botão abaixo para criar seu primeiro estado vazio. Depois arraste os sprites para a área de frames e ajuste FPS.");
            ui.add_space(10.0);
            if ui.button("+ Criar primeiro estado").clicked() {
                let new_name = "idle".to_string();
                let selected_name = new_name.clone();
                update_animator(app, entity_id, animator_index, move |animator| {
                    animator.states.insert(
                        new_name.clone(),
                        AnimationState {
                            clip: new_name.clone(),
                            looped: true,
                            interruptible: true,
                            next_state: String::new(),
                        },
                    );
                    animator.clips.entry(new_name.clone()).or_insert_with(AnimationClip::default);
                    animator.current_state = new_name.clone();
                    animator.default_state = new_name.clone();
                    animator.state_mode = true;
                });
                app.animator_selected_state = selected_name.clone();
                app.animator_state_rename_buffer = selected_name;
                app.status_msg = "✅ Primeiro estado criado.".to_string();
            }
        });
}

fn draw_simple_workspace(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    entity_id: &str,
    animator_index: usize,
    animator: &Animator,
    state_names: &[String],
    selected_name: &str,
    selected_state: &AnimationState,
    selected_clip_name: &str,
    selected_clip: &AnimationClip,
) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(16, 21, 30))
        .rounding(6.0)
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(format!("Editar estado: {}", selected_name)).strong());
            ui.add_space(6.0);
            draw_state_rename_row(app, ui, entity_id, animator_index, selected_name);
            ui.add_space(8.0);

            let compact = ui.available_width() < 720.0;
            if compact {
                draw_clip_drop_and_frames(
                    app,
                    ui,
                    entity_id,
                    animator_index,
                    animator,
                    state_names,
                    selected_name,
                    selected_state,
                    selected_clip_name,
                    selected_clip,
                );
                ui.add_space(10.0);
                draw_preview_and_settings(
                    app,
                    ui,
                    entity_id,
                    animator_index,
                    animator,
                    state_names,
                    selected_name,
                    selected_state,
                    selected_clip_name,
                    selected_clip,
                );
            } else {
                ui.columns(2, |columns| {
                    draw_clip_drop_and_frames(
                        app,
                        &mut columns[0],
                        entity_id,
                        animator_index,
                        animator,
                        state_names,
                        selected_name,
                        selected_state,
                        selected_clip_name,
                        selected_clip,
                    );
                    draw_preview_and_settings(
                        app,
                        &mut columns[1],
                        entity_id,
                        animator_index,
                        animator,
                        state_names,
                        selected_name,
                        selected_state,
                        selected_clip_name,
                        selected_clip,
                    );
                });
            }
        });
}

fn draw_clip_drop_and_frames(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    entity_id: &str,
    animator_index: usize,
    animator: &Animator,
    _state_names: &[String],
    selected_name: &str,
    selected_state: &AnimationState,
    selected_clip_name: &str,
    selected_clip: &AnimationClip,
) {
    let mut clip_name = if selected_state.clip.trim().is_empty() {
        selected_name.to_string()
    } else {
        selected_state.clip.clone()
    };

    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Frames do estado").strong());
                ui.separator();
                ui.monospace(selected_name);
                ui.separator();
                ui.label(format!("Clip: {}", selected_clip_name));
            });
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                ui.label("Clip usado:");
                let clip_names = sorted_clip_names(animator);
                let before = clip_name.clone();
                egui::ComboBox::from_id_source(format!("simple_workspace_clip_{}_{}", entity_id, selected_name))
                    .selected_text(if clip_name.is_empty() { "<vazio>" } else { clip_name.as_str() })
                    .show_ui(ui, |ui| {
                        for clip in &clip_names {
                            ui.selectable_value(&mut clip_name, clip.clone(), clip);
                        }
                    });
                if clip_name != before {
                    let target = selected_name.to_string();
                    let clip = clip_name.clone();
                    update_animator(app, entity_id, animator_index, move |animator| {
                        animator.clips.entry(clip.clone()).or_insert_with(AnimationClip::default);
                        animator.states.entry(target.clone()).or_default().clip = clip.clone();
                        animator.current = clip.clone();
                    });
                    app.status_msg = format!("✅ Clip '{}' vinculado ao estado '{}'.", clip_name, selected_name);
                }
                if ui.button("Salvar clip no estado").clicked() {
                    let target = selected_name.to_string();
                    let clip = clip_name.clone();
                    update_animator(app, entity_id, animator_index, move |animator| {
                        animator.clips.entry(clip.clone()).or_insert_with(AnimationClip::default);
                        animator.states.entry(target.clone()).or_default().clip = clip.clone();
                        animator.current = clip.clone();
                    });
                    app.status_msg = format!("✅ Clip '{}' salvo no estado '{}'.", clip_name, selected_name);
                }
            });

            ui.add_space(8.0);
            draw_frame_drop_zone(app, ui, entity_id, animator_index, &clip_name);
            ui.add_space(8.0);
            draw_frames_list(app, ui, entity_id, animator_index, &clip_name, selected_clip);
        });
}

fn draw_preview_and_settings(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    entity_id: &str,
    animator_index: usize,
    animator: &Animator,
    state_names: &[String],
    selected_name: &str,
    selected_state: &AnimationState,
    selected_clip_name: &str,
    selected_clip: &AnimationClip,
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Preview e ajustes").strong());
            ui.add_space(8.0);
            draw_clip_preview(app, ui, selected_clip_name, selected_clip);
            ui.add_space(10.0);
            draw_simple_settings(app, ui, entity_id, animator_index, animator, state_names, selected_name, selected_state, selected_clip_name, selected_clip);
        });
}

fn draw_frame_drop_zone(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    entity_id: &str,
    animator_index: usize,
    clip_name: &str,
) {
    let desired_h = 96.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(220.0), desired_h),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);

    let hovering_internal = app
        .dragging_asset_path
        .as_ref()
        .map(|p| is_image_asset_path(p) && response.hovered())
        .unwrap_or(false);
    let dropped_files = ui.ctx().input(|i| i.raw.dropped_files.clone());
    let hovering_external = !dropped_files.is_empty() && response.hovered();
    let active = hovering_internal || hovering_external;
    let stroke = if active {
        egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 200, 255))
    } else {
        egui::Stroke::new(1.0, egui::Color32::from_gray(75))
    };
    painter.rect_filled(rect, 8.0, egui::Color32::from_rgb(21, 28, 38));
    painter.rect_stroke(rect, 8.0, stroke);
    painter.text(
        rect.center_top() + egui::vec2(0.0, 22.0),
        egui::Align2::CENTER_CENTER,
        "Solte frames aqui",
        egui::FontId::proportional(16.0),
        egui::Color32::WHITE,
    );
    painter.text(
        rect.center_bottom() - egui::vec2(0.0, 22.0),
        egui::Align2::CENTER_CENTER,
        "Arraste imagens do Assets ou do sistema",
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(170, 185, 205),
    );

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button("+ asset selecionado").clicked() {
            match app.selected_asset.clone() {
                Some(path) if is_image_asset_path(&path) => {
                    let rel = relative_to_project_or_full(&app.project_root, &path);
                    append_frames_to_clip(app, entity_id, animator_index, clip_name, vec![rel]);
                }
                Some(_) => {
                    app.status_msg = "❌ O asset selecionado não é uma imagem compatível.".to_string();
                }
                None => {
                    app.status_msg = "❌ Nenhum asset selecionado no painel Assets.".to_string();
                }
            }
        }
        if ui.button("Limpar frames").clicked() {
            let had_frames = app
                .scene
                .find_entity(entity_id)
                .and_then(|entity| entity.components.get(animator_index))
                .and_then(|component| match component {
                    Component::Animator(animator) => animator.clips.get(clip_name),
                    _ => None,
                })
                .map(|clip| !clip.frames.is_empty())
                .unwrap_or(false);
            if had_frames {
                let clip = clip_name.to_string();
                update_animator(app, entity_id, animator_index, move |animator| {
                    animator.clips.entry(clip.clone()).or_default().frames.clear();
                });
                app.status_msg = format!("✅ Frames do clip '{}' foram limpos.", clip_name);
            } else {
                app.status_msg = format!("ℹ O clip '{}' já está sem frames.", clip_name);
            }
        }
    });

    if hovering_internal && ui.ctx().input(|i| i.pointer.any_released()) {
        if let Some(path) = app.dragging_asset_path.clone() {
            let rel = relative_to_project_or_full(&app.project_root, &path);
            append_frames_to_clip(app, entity_id, animator_index, clip_name, vec![rel]);
        }
    }

    if response.hovered() && !dropped_files.is_empty() {
        let mut to_add = Vec::new();
        for file in dropped_files {
            if let Some(path) = file.path {
                if is_image_asset_path(&path) {
                    to_add.push(relative_to_project_or_full(&app.project_root, &path));
                }
            }
        }
        if !to_add.is_empty() {
            append_frames_to_clip(app, entity_id, animator_index, clip_name, to_add);
        }
    }
}

fn draw_frames_list(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    entity_id: &str,
    animator_index: usize,
    clip_name: &str,
    clip: &AnimationClip,
) {
    ui.label(egui::RichText::new("Lista de frames").strong());
    ui.add_space(4.0);
    if clip.frames.is_empty() {
        ui.label(egui::RichText::new("Nenhum frame ainda. Arraste sprites para a caixa acima.").small().weak());
        return;
    }

    egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
        for (index, frame) in clip.frames.iter().enumerate() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if let Some(texture) = app.load_texture_from_relative_path(ui.ctx(), frame) {
                        ui.image((texture.id(), egui::vec2(40.0, 40.0)));
                    }
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(format!("Frame {}", index + 1)).strong().small());
                        ui.label(egui::RichText::new(frame).small().weak());
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("🗑").clicked() {
                            remove_frame(app, entity_id, animator_index, clip_name, index);
                        }
                        if index + 1 < clip.frames.len() && ui.small_button("↓").clicked() {
                            move_frame(app, entity_id, animator_index, clip_name, index, index + 1);
                        }
                        if index > 0 && ui.small_button("↑").clicked() {
                            move_frame(app, entity_id, animator_index, clip_name, index, index - 1);
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }
    });
}

fn draw_clip_preview(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    clip_name: &str,
    clip: &AnimationClip,
) {
    ui.label(egui::RichText::new("Preview").strong());
    ui.add_space(6.0);
    let preview_h = if ui.available_width() < 340.0 { 180.0 } else { 240.0 };
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(220.0), preview_h),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 8.0, egui::Color32::from_rgb(12, 17, 25));

    if clip.frames.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Adicione frames para ver o preview",
            egui::FontId::proportional(15.0),
            egui::Color32::from_rgb(170, 185, 205),
        );
        return;
    }

    let fps = clip.fps.max(1.0);
    let time = ui.input(|i| i.time) as f32;
    let frame_index = ((time * fps) as usize) % clip.frames.len();
    let frame_path = &clip.frames[frame_index];
    if let Some(texture) = app.load_texture_from_relative_path(ui.ctx(), frame_path) {
        let size = texture.size_vec2();
        let scale = (rect.width() / size.x).min(rect.height() / size.y).min(1.0);
        let draw_size = size * scale * 0.9;
        let image_rect = egui::Rect::from_center_size(rect.center(), draw_size);
        painter.image(
            texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        painter.text(
            rect.left_top() + egui::vec2(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!("{}  •  frame {}/{}", clip_name, frame_index + 1, clip.frames.len()),
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(215, 225, 240),
        );
        ui.ctx().request_repaint();
    } else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("Não foi possível abrir
{}", frame_path),
            egui::FontId::proportional(14.0),
            egui::Color32::from_rgb(214, 102, 102),
        );
    }
}

fn draw_simple_settings(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    entity_id: &str,
    animator_index: usize,
    animator: &Animator,
    state_names: &[String],
    selected_name: &str,
    selected_state: &AnimationState,
    selected_clip_name: &str,
    clip: &AnimationClip,
) {
    let mut fps = clip.fps;
    let mut looped = selected_state.looped;
    let mut interruptible = selected_state.interruptible;
    let mut next_state = selected_state.next_state.clone();
    let mut default_state = animator.default_state.clone();
    let mut threshold = animator.locomotion_threshold;
    let mut changed = false;

    egui::Grid::new(format!("simple_settings_grid_{}_{}", entity_id, selected_name))
        .num_columns(2)
        .spacing([10.0, 8.0])
        .show(ui, |ui| {
            ui.label("FPS:");
            changed |= ui.add(egui::DragValue::new(&mut fps).speed(0.25).range(1.0..=60.0)).changed();
            ui.end_row();

            ui.label("Loop:");
            changed |= ui.checkbox(&mut looped, "Repetir animação").changed();
            ui.end_row();

            ui.label("Interruptível:");
            changed |= ui.checkbox(&mut interruptible, "Pode trocar no meio").changed();
            ui.end_row();

            ui.label("Próximo estado:");
            let next_before = next_state.clone();
            egui::ComboBox::from_id_source(format!("simple_next_state_{}_{}", entity_id, selected_name))
                .selected_text(if next_state.is_empty() { "<nenhum>" } else { next_state.as_str() })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut next_state, String::new(), "<nenhum>");
                    for name in state_names {
                        if name != selected_name {
                            ui.selectable_value(&mut next_state, name.clone(), name);
                        }
                    }
                });
            changed |= next_state != next_before;
            ui.end_row();

            ui.label("Estado padrão:");
            let default_before = default_state.clone();
            egui::ComboBox::from_id_source(format!("simple_default_state_{}_{}", entity_id, selected_name))
                .selected_text(if default_state.is_empty() { "<nenhum>" } else { default_state.as_str() })
                .show_ui(ui, |ui| {
                    for name in state_names {
                        ui.selectable_value(&mut default_state, name.clone(), name);
                    }
                });
            changed |= default_state != default_before;
            ui.end_row();

            ui.label("Threshold movimento:");
            changed |= ui.add(egui::DragValue::new(&mut threshold).speed(0.25).range(0.0..=999.0)).changed();
            ui.end_row();
        });

    if changed {
        let state_name = selected_name.to_string();
        let clip_name = selected_clip_name.to_string();
        let next_state_for_save = next_state.clone();
        let default_state_for_save = default_state.clone();
        update_animator(app, entity_id, animator_index, move |animator| {
            animator.state_mode = true;
            animator.default_state = default_state_for_save.clone();
            animator.locomotion_threshold = threshold;
            animator.clips.entry(clip_name.clone()).or_default().fps = fps;
            if let Some(state) = animator.states.get_mut(&state_name) {
                state.clip = clip_name.clone();
                state.looped = looped;
                state.interruptible = interruptible;
                state.next_state = next_state_for_save.clone();
            }
            if animator.current_state == state_name {
                animator.looped = looped;
            }
        });
    }

    ui.add_space(8.0);
    if ui.button("Salvar ajustes").clicked() {
        let state_name = selected_name.to_string();
        let clip_name = selected_clip_name.to_string();
        let next_state_for_save = next_state.clone();
        let default_state_for_save = default_state.clone();
        update_animator(app, entity_id, animator_index, move |animator| {
            animator.state_mode = true;
            animator.default_state = default_state_for_save.clone();
            animator.locomotion_threshold = threshold;
            animator.clips.entry(clip_name.clone()).or_default().fps = fps;
            if let Some(state) = animator.states.get_mut(&state_name) {
                state.clip = clip_name.clone();
                state.looped = looped;
                state.interruptible = interruptible;
                state.next_state = next_state_for_save.clone();
            }
            if animator.current_state == state_name {
                animator.looped = looped;
            }
        });
        app.status_msg = format!("✅ Ajustes do estado '{}' salvos com sucesso.", selected_name);
    }
}

fn draw_state_rename_row(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    entity_id: &str,
    animator_index: usize,
    selected_name: &str,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Nome do estado:");
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.animator_state_rename_buffer)
                .desired_width(220.0)
                .hint_text("idle, run, attack..."),
        );
        let confirm = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let clicked = ui.button("Salvar nome").clicked();
        if confirm || clicked {
            rename_selected_state(app, entity_id, animator_index, selected_name);
        }
        if ui
            .add(egui::Button::new(egui::RichText::new("🗑 Deletar estado").size(12.0)))
            .on_hover_text("Remove o estado atual com segurança")
            .clicked()
        {
            delete_state(app, entity_id, animator_index, selected_name);
        }
    });
    ui.label(egui::RichText::new("Exemplo: idle, run, attack, jump, slide.").small().weak());
}

fn delete_state(
    app: &mut EditorApp,
    entity_id: &str,
    animator_index: usize,
    state_name: &str,
) {
    let state_to_delete = state_name.trim().to_string();
    if state_to_delete.is_empty() {
        app.status_msg = "❌ Nenhum estado selecionado para deletar.".to_string();
        return;
    }

    let mut delete_result: Result<String, String> = Err("Estado inválido.".to_string());
    update_animator(app, entity_id, animator_index, |animator| {
        if !animator.states.contains_key(&state_to_delete) {
            delete_result = Err(format!("Estado '{}' não encontrado.", state_to_delete));
            return;
        }

        let removed_state = animator.states.remove(&state_to_delete).unwrap_or_default();
        let removed_clip_name = removed_state.clip.clone();

        for state in animator.states.values_mut() {
            if state.next_state == state_to_delete {
                state.next_state.clear();
            }
        }

        if animator.current_state == state_to_delete {
            animator.current_state = if !animator.default_state.is_empty()
                && animator.states.contains_key(&animator.default_state)
            {
                animator.default_state.clone()
            } else {
                animator.states.keys().next().cloned().unwrap_or_default()
            };
        }
        if animator.default_state == state_to_delete {
            animator.default_state = animator.states.keys().next().cloned().unwrap_or_default();
        }
        if animator.queued_state == state_to_delete {
            animator.queued_state.clear();
        }

        let clip_still_used = animator
            .states
            .values()
            .any(|state| !removed_clip_name.is_empty() && state.clip == removed_clip_name);

        if !removed_clip_name.is_empty() && !clip_still_used {
            animator.clips.remove(&removed_clip_name);
            if animator.current == removed_clip_name {
                animator.current = if animator.current_state.is_empty() {
                    String::new()
                } else {
                    animator
                        .states
                        .get(&animator.current_state)
                        .map(|state| state.clip.clone())
                        .unwrap_or_default()
                };
            }
            if animator.prev_clip == removed_clip_name {
                animator.prev_clip.clear();
            }
        }

        if animator.states.is_empty() {
            animator.current_state.clear();
            animator.default_state.clear();
            animator.current.clear();
        } else if animator.current.is_empty() {
            animator.current = if animator.current_state.is_empty() {
                String::new()
            } else {
                animator
                    .states
                    .get(&animator.current_state)
                    .map(|state| state.clip.clone())
                    .unwrap_or_default()
            };
        }

        delete_result = Ok(removed_clip_name);
    });

    match delete_result {
        Ok(_) => {
            app.animator_node_positions.remove(&state_to_delete);
            let remaining_states: Vec<String> = app
                .scene
                .find_entity(entity_id)
                .and_then(|entity| entity.components.get(animator_index))
                .and_then(|component| match component {
                    Component::Animator(animator) => Some(sorted_state_names(animator)),
                    _ => None,
                })
                .unwrap_or_default();

            app.animator_selected_state = remaining_states.first().cloned().unwrap_or_default();
            app.animator_state_rename_buffer = app.animator_selected_state.clone();
            if remaining_states.is_empty() {
                app.status_msg = format!("✅ Estado '{}' removido. O Animator ficou sem estados.", state_to_delete);
            } else {
                app.status_msg = format!("✅ Estado '{}' removido com sucesso.", state_to_delete);
            }
        }
        Err(error) => {
            app.status_msg = format!("❌ {}", error);
        }
    }
}

fn rename_selected_state(
    app: &mut EditorApp,
    entity_id: &str,
    animator_index: usize,
    old_name: &str,
) {
    let new_name = app.animator_state_rename_buffer.trim().to_string();
    if new_name.is_empty() {
        app.status_msg = "❌ Digite um nome válido para o estado.".to_string();
        return;
    }
    if new_name == old_name {
        app.status_msg = "ℹ O estado já está com esse nome.".to_string();
        return;
    }

    let mut rename_result: Result<(), String> = Ok(());
    update_animator(app, entity_id, animator_index, |animator| {
        if animator.states.contains_key(&new_name) {
            rename_result = Err(format!("Já existe um estado chamado '{}'.", new_name));
            return;
        }
        let Some(mut state) = animator.states.remove(old_name) else {
            rename_result = Err(format!("Estado '{}' não encontrado.", old_name));
            return;
        };

        let should_rename_clip = state.clip.trim().is_empty() || state.clip == old_name;
        if should_rename_clip {
            state.clip = new_name.clone();
        }
        animator.states.insert(new_name.clone(), state);

        for state in animator.states.values_mut() {
            if state.next_state == old_name {
                state.next_state = new_name.clone();
            }
            if should_rename_clip && state.clip == old_name {
                state.clip = new_name.clone();
            }
        }

        if should_rename_clip {
            if let Some(clip) = animator.clips.remove(old_name) {
                animator.clips.insert(new_name.clone(), clip);
            } else {
                animator.clips.entry(new_name.clone()).or_insert_with(AnimationClip::default);
            }
            if animator.current == old_name {
                animator.current = new_name.clone();
            }
            if animator.prev_clip == old_name {
                animator.prev_clip = new_name.clone();
            }
        }

        if animator.current_state == old_name {
            animator.current_state = new_name.clone();
        }
        if animator.default_state == old_name {
            animator.default_state = new_name.clone();
        }
        if animator.queued_state == old_name {
            animator.queued_state = new_name.clone();
        }
    });

    match rename_result {
        Ok(()) => {
            if let Some(position) = app.animator_node_positions.remove(old_name) {
                app.animator_node_positions.insert(new_name.clone(), position);
            }
            app.animator_selected_state = new_name.clone();
            app.animator_state_rename_buffer = new_name.clone();
            app.status_msg = format!("✅ Estado '{}' renomeado para '{}'.", old_name, new_name);
        }
        Err(error) => {
            app.status_msg = format!("❌ {}", error);
        }
    }
}

fn append_frames_to_clip(
    app: &mut EditorApp,
    entity_id: &str,
    animator_index: usize,
    clip_name: &str,
    frames: Vec<String>,
) {
    if frames.is_empty() {
        app.status_msg = "❌ Nenhum asset de imagem válido foi encontrado para adicionar.".to_string();
        return;
    }
    let clip = clip_name.to_string();
    let added_count = frames.len();
    update_animator(app, entity_id, animator_index, move |animator| {
        let entry = animator.clips.entry(clip.clone()).or_default();
        for frame in &frames {
            if !entry.frames.iter().any(|existing| existing == frame) {
                entry.frames.push(frame.clone());
            }
        }
    });
    app.status_msg = if added_count == 1 {
        format!("✅ 1 frame adicionado ao clip '{}'.", clip_name)
    } else {
        format!("✅ {} frames adicionados ao clip '{}'.", added_count, clip_name)
    };
}

fn move_frame(
    app: &mut EditorApp,
    entity_id: &str,
    animator_index: usize,
    clip_name: &str,
    from: usize,
    to: usize,
) {
    let clip = clip_name.to_string();
    update_animator(app, entity_id, animator_index, move |animator| {
        let frames = &mut animator.clips.entry(clip.clone()).or_default().frames;
        if from < frames.len() && to < frames.len() {
            let frame = frames.remove(from);
            frames.insert(to, frame);
        }
    });
}

fn remove_frame(
    app: &mut EditorApp,
    entity_id: &str,
    animator_index: usize,
    clip_name: &str,
    index: usize,
) {
    let clip = clip_name.to_string();
    update_animator(app, entity_id, animator_index, move |animator| {
        let frames = &mut animator.clips.entry(clip.clone()).or_default().frames;
        if index < frames.len() {
            frames.remove(index);
        }
    });
}

fn is_image_asset_path(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()),
        Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp")
    )
}

fn relative_to_project_or_full(project_root: &std::path::Path, path: &std::path::Path) -> String {
    if let Ok(relative) = path.strip_prefix(project_root) {
        return relative.to_string_lossy().replace('\\', "/");
    }
    path.to_string_lossy().replace('\\', "/")
}


fn show_empty_state(ui: &mut egui::Ui, text: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.label(egui::RichText::new(text).strong());
    });
}

fn sorted_state_names(animator: &Animator) -> Vec<String> {
    let mut names: Vec<String> = animator.states.keys().cloned().collect();
    names.sort();
    names
}

fn sorted_clip_names(animator: &Animator) -> Vec<String> {
    let mut names: Vec<String> = animator.clips.keys().cloned().collect();
    names.sort();
    names
}

fn unique_state_name(animator: &Animator, base: &str) -> String {
    if !animator.states.contains_key(base) {
        return base.to_string();
    }
    let mut index = 1usize;
    loop {
        let candidate = format!("{}_{}", base, index);
        if !animator.states.contains_key(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn ensure_node_layout(app: &mut EditorApp, state_names: &[String]) {
    for (index, state_name) in state_names.iter().enumerate() {
        app.animator_node_positions.entry(state_name.clone()).or_insert_with(|| {
            let row = index as f32;
            [0.25 + ((index % 2) as f32) * 0.28, (0.18 + row * 0.12).min(0.82)]
        });
    }
    app.animator_node_positions.retain(|name, _| state_names.iter().any(|state| state == name));
}

fn update_animator<F>(app: &mut EditorApp, entity_id: &str, animator_index: usize, mut update: F)
where
    F: FnMut(&mut Animator),
{
    if let Some(entity) = app.scene.find_entity_mut(entity_id) {
        if let Some(Component::Animator(animator)) = entity.components.get_mut(animator_index) {
            update(animator);
        }
    }
}
