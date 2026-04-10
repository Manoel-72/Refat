use eframe::egui;

use crate::{
    component::{AnimationClip, AnimationState, Animator, Component},
    editor::EditorApp,
};

const ENTRY_NODE_W: f32 = 110.0;
const ENTRY_NODE_H: f32 = 32.0;
const EXIT_NODE_W: f32 = 110.0;
const EXIT_NODE_H: f32 = 32.0;
const STATE_NODE_W: f32 = 138.0;
const STATE_NODE_H: f32 = 42.0;
const GRAPH_MIN_W: f32 = 320.0;
const GRAPH_MIN_H: f32 = 220.0;

pub fn show(app: &mut EditorApp, ui: &mut egui::Ui) {
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

    let mut state_names = sorted_state_names(&animator_snapshot);
    if state_names.is_empty() {
        state_names.push("idle".to_string());
    }

    ensure_node_layout(app, &state_names);

    if app.animator_selected_state.trim().is_empty()
        || !state_names.iter().any(|name| name == &app.animator_selected_state)
    {
        let fallback_state = if !animator_snapshot.current_state.trim().is_empty() {
            animator_snapshot.current_state.clone()
        } else if !animator_snapshot.default_state.trim().is_empty() {
            animator_snapshot.default_state.clone()
        } else {
            state_names[0].clone()
        };
        app.animator_selected_state = fallback_state;
    }

    let selected_state_name = app.animator_selected_state.clone();
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

    draw_header(ui, &entity.name, &animator_snapshot, &selected_state_name, &selected_clip_name);
    ui.add_space(6.0);
    draw_help_bar(ui);
    ui.add_space(8.0);

    let compact_layout = ui.available_width() < 920.0;
    if compact_layout {
        draw_state_sidebar(app, ui, &entity_id, animator_index, &animator_snapshot, &state_names);
        ui.add_space(8.0);
        draw_graph_workspace(app, ui, &entity_id, animator_index, &animator_snapshot, &state_names);
    } else {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(240.0, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| draw_state_sidebar(app, ui, &entity_id, animator_index, &animator_snapshot, &state_names),
            );
            ui.add_space(8.0);
            ui.vertical(|ui| {
                draw_graph_workspace(app, ui, &entity_id, animator_index, &animator_snapshot, &state_names);
            });
        });
    }

    ui.add_space(8.0);
    draw_timeline(ui, &selected_state_name, &selected_state, &selected_clip);
}

fn draw_header(
    ui: &mut egui::Ui,
    entity_name: &str,
    animator: &Animator,
    selected_state_name: &str,
    selected_clip_name: &str,
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
                let mode = if animator.state_mode { "State Machine" } else { "Clip direto" };
                ui.label(format!("Modo: {}", mode));
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
                ui.label(egui::RichText::new("Como usar").strong());
                ui.separator();
                ui.label("1. Selecione um estado na lista.");
                ui.separator();
                ui.label("2. Arraste o nó no graph.");
                ui.separator();
                ui.label("3. Ajuste clip, loop e próximo estado.");
                ui.separator();
                ui.label("4. Clique em Salvar estado.");
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
            ui.set_min_width(200.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Máquinas de Estado").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("+ Estado").clicked() {
                        let mut new_name = unique_state_name(animator, "novo_estado");
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
                            if !animator.clips.contains_key(&new_name) {
                                animator.clips.insert(new_name.clone(), AnimationClip::default());
                            }
                            animator.current_state = new_name.clone();
                            animator.state_mode = true;
                        });
                        let count = state_names.len().max(1) as f32;
                        let y = (0.18 + (count * 0.12)).min(0.82);
                        app.animator_node_positions.insert(new_name.clone(), [0.40, y]);
                        app.animator_selected_state = std::mem::take(&mut new_name);
                    }
                });
            });
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .id_source("animator_state_sidebar")
                .max_height(240.0)
                .show(ui, |ui| {
                    for state_name in state_names {
                        let selected = app.animator_selected_state == *state_name;
                        let resp = ui.selectable_label(selected, state_name);
                        if resp.clicked() {
                            app.animator_selected_state = state_name.clone();
                        }
                        if resp.double_clicked() {
                            let target = state_name.clone();
                            update_animator(app, entity_id, animator_index, |animator| {
                                animator.current_state = target.clone();
                                if let Some(state) = animator.states.get(&target) {
                                    animator.current = state.clip.clone();
                                    animator.looped = state.looped;
                                }
                            });
                        }
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
                ui.add(egui::Slider::new(&mut app.animator_graph_zoom, 0.75..=1.5).clamp_to_range(true));
            });

            if ui.small_button("Reorganizar nós").clicked() {
                app.animator_node_positions.clear();
                ensure_node_layout(app, state_names);
            }

            if ui.small_button("Abrir state mode").clicked() {
                update_animator(app, entity_id, animator_index, |animator| {
                    animator.state_mode = true;
                });
            }

            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Clique para selecionar. Duplo clique ativa. Arraste no graph para organizar.")
                    .small()
                    .weak(),
            );
        });
}

fn draw_graph_workspace(
    app: &mut EditorApp,
    ui: &mut egui::Ui,
    entity_id: &str,
    animator_index: usize,
    animator: &Animator,
    state_names: &[String],
) {
    let selected_name = app.animator_selected_state.clone();
    let selected_state = animator
        .states
        .get(&selected_name)
        .cloned()
        .unwrap_or_else(|| AnimationState {
            clip: selected_name.clone(),
            ..AnimationState::default()
        });

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(16, 21, 30))
        .rounding(6.0)
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Graph View").strong());
                ui.separator();
                ui.label("Entrada → estados → saída");
                ui.separator();
                ui.label("Arraste nós com o mouse.");
            });
            ui.add_space(6.0);

            let graph_h: f32 = if ui.available_width() < 620.0 { 260.0 } else { 320.0 };
            let desired_size = egui::vec2(ui.available_width().max(GRAPH_MIN_W), graph_h.max(GRAPH_MIN_H));
            let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(11, 16, 24));
            paint_animator_grid(&painter, rect, app.animator_graph_zoom);

            let content_rect = rect.shrink2(egui::vec2(18.0, 18.0));
            let entry_pos = egui::pos2(content_rect.left() + 18.0, content_rect.top() + 10.0);
            let exit_pos = egui::pos2(content_rect.left() + 18.0, content_rect.bottom() - EXIT_NODE_H - 10.0);
            let state_positions = state_node_positions(app, content_rect, state_names);

            handle_graph_interaction(app, &response, content_rect, state_names);

            draw_node(&painter, entry_pos, egui::vec2(ENTRY_NODE_W, ENTRY_NODE_H), "Entrada", egui::Color32::from_rgb(92, 171, 83), false);
            draw_node(&painter, exit_pos, egui::vec2(EXIT_NODE_W, EXIT_NODE_H), "Saída", egui::Color32::from_rgb(170, 65, 65), false);

            if let Some(default_pos) = state_positions.get(&animator.default_state).copied().or_else(|| state_positions.values().next().copied()) {
                draw_link(&painter, entry_pos + egui::vec2(ENTRY_NODE_W * 0.5, ENTRY_NODE_H), default_pos + egui::vec2(STATE_NODE_W * 0.5, 0.0), egui::Color32::from_rgb(92, 171, 83));
            }

            for state_name in state_names {
                let pos = state_positions.get(state_name).copied().unwrap_or(content_rect.left_top());
                let is_selected = state_name == &selected_name;
                let is_current = animator.current_state == *state_name;
                let color = if is_selected {
                    egui::Color32::from_rgb(88, 166, 255)
                } else if is_current {
                    egui::Color32::from_rgb(220, 142, 59)
                } else {
                    egui::Color32::from_rgb(62, 78, 108)
                };
                draw_node(&painter, pos, egui::vec2(STATE_NODE_W, STATE_NODE_H), state_name, color, is_current);

                let state = animator.states.get(state_name).cloned().unwrap_or_default();
                let target_name = state.next_state.trim();
                if !target_name.is_empty() {
                    if let Some(target_pos) = state_positions.get(target_name).copied() {
                        draw_link(
                            &painter,
                            pos + egui::vec2(STATE_NODE_W, STATE_NODE_H * 0.5),
                            target_pos + egui::vec2(0.0, STATE_NODE_H * 0.5),
                            egui::Color32::from_rgb(155, 170, 190),
                        );
                    }
                } else {
                    draw_link(
                        &painter,
                        pos + egui::vec2(STATE_NODE_W * 0.5, STATE_NODE_H),
                        exit_pos + egui::vec2(EXIT_NODE_W * 0.5, 0.0),
                        egui::Color32::from_rgb(96, 110, 132),
                    );
                }
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Estado selecionado").strong());

            let mut clip_name = selected_state.clip.clone();
            let mut looped = selected_state.looped;
            let mut interruptible = selected_state.interruptible;
            let mut next_state = selected_state.next_state.clone();
            let mut default_state = animator.default_state.clone();
            let mut locomotion_threshold = animator.locomotion_threshold;
            let clip_names = sorted_clip_names(animator);

            egui::Grid::new("animator_workspace_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Nome do estado:");
                    ui.monospace(&selected_name);
                    ui.end_row();

                    ui.label("Clip:");
                    egui::ComboBox::from_id_source("animator_workspace_clip")
                        .selected_text(if clip_name.is_empty() { "<vazio>" } else { clip_name.as_str() })
                        .show_ui(ui, |ui| {
                            for clip in &clip_names {
                                ui.selectable_value(&mut clip_name, clip.clone(), clip);
                            }
                        });
                    ui.end_row();

                    ui.label("Loop:");
                    ui.checkbox(&mut looped, "Executar em ciclo");
                    ui.end_row();

                    ui.label("Interruptível:");
                    ui.checkbox(&mut interruptible, "Pode trocar no meio");
                    ui.end_row();

                    ui.label("Próximo estado:");
                    egui::ComboBox::from_id_source("animator_workspace_next")
                        .selected_text(if next_state.trim().is_empty() { "<saída>" } else { next_state.as_str() })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut next_state, String::new(), "<saída>");
                            for name in state_names {
                                if name != &selected_name {
                                    ui.selectable_value(&mut next_state, name.clone(), name);
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Estado padrão:");
                    egui::ComboBox::from_id_source("animator_workspace_default")
                        .selected_text(if default_state.trim().is_empty() { "<vazio>" } else { default_state.as_str() })
                        .show_ui(ui, |ui| {
                            for name in state_names {
                                ui.selectable_value(&mut default_state, name.clone(), name);
                            }
                        });
                    ui.end_row();

                    ui.label("Threshold run:");
                    ui.add(egui::DragValue::new(&mut locomotion_threshold).speed(0.25).range(0.0..=999.0));
                    ui.end_row();
                });

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button("Salvar estado").clicked() {
                    let target = selected_name.clone();
                    update_animator(app, entity_id, animator_index, move |animator| {
                        animator.default_state = default_state.clone();
                        animator.locomotion_threshold = locomotion_threshold;
                        animator.state_mode = true;
                        animator.states.insert(
                            target.clone(),
                            AnimationState {
                                clip: clip_name.clone(),
                                looped,
                                interruptible,
                                next_state: next_state.clone(),
                            },
                        );
                    });
                }

                if ui.button("Definir como atual").clicked() {
                    let target = selected_name.clone();
                    update_animator(app, entity_id, animator_index, move |animator| {
                        animator.current_state = target.clone();
                    });
                }

                if selected_name != "idle" && ui.button("Excluir estado").clicked() {
                    let target = selected_name.clone();
                    update_animator(app, entity_id, animator_index, move |animator| {
                        animator.states.remove(&target);
                        if animator.current_state == target {
                            animator.current_state = animator.default_state.clone();
                        }
                        if animator.default_state == target {
                            animator.default_state = "idle".to_string();
                        }
                    });
                    app.animator_node_positions.remove(&selected_name);
                    app.animator_selected_state = "idle".to_string();
                }
            });
        });
}

fn draw_timeline(ui: &mut egui::Ui, state_name: &str, state: &AnimationState, clip: &AnimationClip) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(18, 24, 32))
        .rounding(6.0)
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(format!("Animação: {}", state_name)).strong());
                ui.separator();
                ui.label(format!("Clip: {}", state.clip));
                ui.separator();
                ui.label(format!("Frames: {}", clip.frames.len()));
                ui.separator();
                ui.label(format!("FPS: {:.1}", clip.fps));
                ui.separator();
                ui.label(if state.looped { "Loop" } else { "One-shot" });
            });
            ui.add_space(8.0);

            let tracks = ["Root Transform", "Sprite", "Collider"];
            let timeline_height = 34.0 * tracks.len() as f32 + 34.0;
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), timeline_height), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(11, 16, 24));

            let header_h = 26.0;
            let sidebar_w = if rect.width() < 540.0 { 96.0 } else { 124.0 };
            let row_h = 32.0;
            let frame_count = clip.frames.len().max(1);
            let track_width = (rect.width() - sidebar_w - 12.0).max(120.0);
            let step = track_width / frame_count as f32;

            for i in 0..=frame_count {
                let x = rect.left() + sidebar_w + (i as f32 * step).min(track_width);
                painter.line_segment(
                    [egui::pos2(x, rect.top() + header_h), egui::pos2(x, rect.bottom() - 8.0)],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 40, 56)),
                );
                if i < frame_count {
                    let time = if clip.fps > 0.0 { i as f32 / clip.fps } else { 0.0 };
                    painter.text(
                        egui::pos2(x + 6.0, rect.top() + 5.0),
                        egui::Align2::LEFT_TOP,
                        format!("{:.2}", time),
                        egui::FontId::proportional(11.0),
                        egui::Color32::from_rgb(130, 145, 170),
                    );
                }
            }

            for (row_index, track_name) in tracks.iter().enumerate() {
                let row_top = rect.top() + header_h + row_index as f32 * row_h;
                let row_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 6.0, row_top),
                    egui::vec2(rect.width() - 12.0, row_h),
                );
                painter.rect_filled(
                    row_rect,
                    4.0,
                    if row_index % 2 == 0 {
                        egui::Color32::from_rgb(16, 22, 31)
                    } else {
                        egui::Color32::from_rgb(19, 26, 36)
                    },
                );
                painter.text(
                    egui::pos2(row_rect.left() + 8.0, row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    *track_name,
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_rgb(215, 225, 240),
                );

                for frame_index in 0..frame_count {
                    let center = egui::pos2(
                        rect.left() + sidebar_w + frame_index as f32 * step + step * 0.5,
                        row_rect.center().y,
                    );
                    let color = match row_index {
                        0 => egui::Color32::from_rgb(122, 162, 247),
                        1 => egui::Color32::from_rgb(160, 205, 120),
                        _ => egui::Color32::from_rgb(188, 156, 255),
                    };
                    painter.circle_filled(center, 3.5, color);
                }
            }

            ui.add_space(8.0);
            if let Some(frame) = clip.frames.first() {
                ui.label(egui::RichText::new(format!("Primeiro frame: {}", frame)).small().weak());
            } else {
                ui.label(egui::RichText::new("Nenhum frame no clip atual.").small().weak());
            }
        });
}

fn handle_graph_interaction(
    app: &mut EditorApp,
    response: &egui::Response,
    content_rect: egui::Rect,
    state_names: &[String],
) {
    let pointer_pos = response.interact_pointer_pos();

    if response.drag_started() {
        if let Some(pointer) = pointer_pos {
            for state_name in state_names.iter().rev() {
                let rect = state_rect_for_name(app, content_rect, state_name);
                if rect.contains(pointer) {
                    app.animator_selected_state = state_name.clone();
                    app.animator_dragging_state = Some(state_name.clone());
                    app.animator_drag_offset = [pointer.x - rect.min.x, pointer.y - rect.min.y];
                    break;
                }
            }
        }
    }

    if response.dragged() {
        if let (Some(state_name), Some(pointer)) = (app.animator_dragging_state.clone(), pointer_pos) {
            let mut x = pointer.x - app.animator_drag_offset[0];
            let mut y = pointer.y - app.animator_drag_offset[1];
            x = x.clamp(content_rect.left(), content_rect.right() - STATE_NODE_W);
            y = y.clamp(content_rect.top(), content_rect.bottom() - STATE_NODE_H);
            let nx = ((x - content_rect.left()) / (content_rect.width() - STATE_NODE_W).max(1.0)).clamp(0.0, 1.0);
            let ny = ((y - content_rect.top()) / (content_rect.height() - STATE_NODE_H).max(1.0)).clamp(0.0, 1.0);
            app.animator_node_positions.insert(state_name, [nx, ny]);
        }
    }

    if !response.ctx.input(|i| i.pointer.primary_down()) {
        app.animator_dragging_state = None;
    }

    if response.clicked() {
        if let Some(pointer) = pointer_pos {
            for state_name in state_names.iter().rev() {
                let rect = state_rect_for_name(app, content_rect, state_name);
                if rect.contains(pointer) {
                    app.animator_selected_state = state_name.clone();
                    break;
                }
            }
        }
    }
}

fn ensure_node_layout(app: &mut EditorApp, state_names: &[String]) {
    for (index, state_name) in state_names.iter().enumerate() {
        app.animator_node_positions.entry(state_name.clone()).or_insert_with(|| {
            let column = index % 3;
            let row = index / 3;
            let x = 0.18 + column as f32 * 0.28;
            let y = 0.12 + row as f32 * 0.20;
            [x.min(0.82), y.min(0.82)]
        });
    }
}

fn state_node_positions(
    app: &EditorApp,
    rect: egui::Rect,
    state_names: &[String],
) -> std::collections::HashMap<String, egui::Pos2> {
    let mut out = std::collections::HashMap::new();
    let usable_w = (rect.width() - STATE_NODE_W).max(1.0);
    let usable_h = (rect.height() - STATE_NODE_H).max(1.0);
    for state_name in state_names {
        let stored = app.animator_node_positions.get(state_name).copied().unwrap_or([0.2, 0.2]);
        let pos = egui::pos2(
            rect.left() + stored[0].clamp(0.0, 1.0) * usable_w,
            rect.top() + stored[1].clamp(0.0, 1.0) * usable_h,
        );
        out.insert(state_name.clone(), pos);
    }
    out
}

fn state_rect_for_name(app: &EditorApp, rect: egui::Rect, state_name: &str) -> egui::Rect {
    let usable_w = (rect.width() - STATE_NODE_W).max(1.0);
    let usable_h = (rect.height() - STATE_NODE_H).max(1.0);
    let stored = app.animator_node_positions.get(state_name).copied().unwrap_or([0.2, 0.2]);
    let pos = egui::pos2(
        rect.left() + stored[0].clamp(0.0, 1.0) * usable_w,
        rect.top() + stored[1].clamp(0.0, 1.0) * usable_h,
    );
    egui::Rect::from_min_size(pos, egui::vec2(STATE_NODE_W, STATE_NODE_H))
}

fn update_animator<F>(app: &mut EditorApp, entity_id: &str, animator_index: usize, mut f: F)
where
    F: FnMut(&mut Animator),
{
    if let Some(entity) = app.find_entity_mut(entity_id) {
        if let Some(Component::Animator(animator)) = entity.components.get_mut(animator_index) {
            f(animator);
        }
    }
}

fn sorted_state_names(animator: &Animator) -> Vec<String> {
    let mut state_names: Vec<String> = animator.states.keys().cloned().collect();
    state_names.sort();
    state_names
}

fn sorted_clip_names(animator: &Animator) -> Vec<String> {
    let mut clip_names: Vec<String> = animator.clips.keys().cloned().collect();
    clip_names.sort();
    if clip_names.is_empty() {
        clip_names.push("idle".to_string());
    }
    clip_names
}

fn unique_state_name(animator: &Animator, base: &str) -> String {
    if !animator.states.contains_key(base) {
        return base.to_string();
    }
    for index in 2..200 {
        let candidate = format!("{}_{}", base, index);
        if !animator.states.contains_key(&candidate) {
            return candidate;
        }
    }
    format!("{}_novo", base)
}

fn show_empty_state(ui: &mut egui::Ui, message: &str) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(18, 24, 32))
        .rounding(6.0)
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new("🎞 Animator").strong().size(15.0));
            ui.add_space(6.0);
            ui.label(message);
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Selecione uma entidade e adicione o componente Animator no Inspector.").small().weak());
        });
}

fn paint_animator_grid(painter: &egui::Painter, rect: egui::Rect, zoom: f32) {
    let grid_color = egui::Color32::from_rgb(22, 32, 46);
    let step = (24.0 * zoom.clamp(0.75, 1.5)).max(16.0);
    let mut x = rect.left();
    while x < rect.right() {
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, grid_color),
        );
        x += step;
    }
    let mut y = rect.top();
    while y < rect.bottom() {
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, grid_color),
        );
        y += step;
    }
}

fn draw_node(
    painter: &egui::Painter,
    pos: egui::Pos2,
    size: egui::Vec2,
    label: &str,
    color: egui::Color32,
    current: bool,
) {
    let rect = egui::Rect::from_min_size(pos, size);
    painter.rect_filled(rect, 6.0, color);
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(
            if current { 2.0 } else { 1.0 },
            egui::Color32::from_rgb(210, 225, 255),
        ),
    );
    painter.text(
        rect.center_top() + egui::vec2(0.0, 10.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        egui::Color32::WHITE,
    );
    if current {
        painter.text(
            rect.center_bottom() - egui::vec2(0.0, 10.0),
            egui::Align2::CENTER_CENTER,
            "Atual",
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(248, 244, 214),
        );
    }
}

fn draw_link(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, color: egui::Color32) {
    let mid_x = (from.x + to.x) * 0.5;
    let points = [
        from,
        egui::pos2(mid_x, from.y),
        egui::pos2(mid_x, to.y),
        to,
    ];
    painter.add(egui::Shape::line(
        points.to_vec(),
        egui::Stroke::new(1.7, color),
    ));
    let dir = (to - egui::pos2(mid_x, to.y)).normalized();
    let arrow_tip = to;
    let left = arrow_tip - dir * 10.0 + egui::vec2(-dir.y, dir.x) * 4.0;
    let right = arrow_tip - dir * 10.0 + egui::vec2(dir.y, -dir.x) * 4.0;
    painter.add(egui::Shape::convex_polygon(vec![arrow_tip, left, right], color, egui::Stroke::NONE));
}
