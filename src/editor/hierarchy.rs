// ============================================================
//  editor/hierarchy.rs
//  Painel de Hierarquia — lista de entidades da cena em árvore
//  com ícones por tipo, seleção múltipla e visibilidade.
// ============================================================

use eframe::egui;

use crate::{
    component::{Animator, Audio, BoxCollider, Camera2D, Component, LuaScript, RigidBody2D, Script, Sprite, TextLabel, UIButton, Velocity},
    entity::Entity,
};
use super::{DeleteTarget, EditorApp};

pub fn show(app: &mut EditorApp, ui: &mut egui::Ui) {
    // ── Cabeçalho da Hierarquia ──
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(14, 18, 28))
        .inner_margin(egui::Margin { left: 10.0, right: 8.0, top: 7.0, bottom: 5.0 })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // ponto colorido de acento
                let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter().circle_filled(dot.center(), 4.0, egui::Color32::from_rgb(88, 166, 255));
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Hierarquia").strong().size(13.0).color(egui::Color32::WHITE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let count = app.scene.entities.len();
                    ui.label(
                        egui::RichText::new(format!("{}", count))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(88, 166, 255)),
                    );
                    ui.label(
                        egui::RichText::new("ent ·")
                            .size(10.0)
                            .color(egui::Color32::from_rgb(80, 95, 115)),
                    );
                });
            });
        });

    ui.separator();

    // ── Botão Nova Entidade + info de seleção ──
    ui.horizontal_wrapped(|ui| {
        if ui.add(
            egui::Button::new(egui::RichText::new("➕ Nova Entidade").size(12.0))
                .min_size(egui::vec2(130.0, 24.0))
        ).on_hover_text("Cria uma entidade vazia na cena (ou use clique direito)").clicked() {
            app.new_entity_dialog = Some("Entidade".to_string());
        }

        if !app.selected_entity_ids.is_empty() {
            ui.label(
                egui::RichText::new(format!(
                    "✔ {} selecionado(s)",
                    app.selected_entity_ids.len()
                ))
                .small()
                .color(egui::Color32::from_rgb(88, 166, 255)),
            );
        }
    });

    ui.add_space(2.0);
    ui.label(
        egui::RichText::new("💡 Clique direito para mais opções")
            .small()
            .color(egui::Color32::from_rgb(100, 112, 130)),
    );
    ui.separator();

    // Um contexto de clique direito disponível na área vazia da hierarquia
    // (para criar entidades na raiz sem clicar em um item existente)

    // Lista rolável de entidades
    egui::ScrollArea::vertical()
        .id_source("hierarchy_scroll")
        .show(ui, |ui| {
            // Coletar ações para não ter borrow duplo
            let mut to_delete: Option<(Vec<String>, String)> = None;

            // Renderizar a árvore
            let entities_clone = app.scene.entities.clone();
            for entity in &entities_clone {
                show_entity_node(ui, entity, app, 0, &mut to_delete);
            }

            // Clique direito no espaço restante da hierarquia
            let extra_space = ui.available_rect_before_wrap();
            if extra_space.height() > 8.0 {
                let response = ui.allocate_rect(extra_space, egui::Sense::click());
                response.context_menu(|ui| {
                    ui.label(egui::RichText::new("Criar na raiz da cena").strong());
                    ui.separator();
                    if ui.button("🔷 Entidade Vazia").clicked() {
                        app.push_undo_state();
                        let entity = Entity::new("Entidade");
                        let id = entity.id.clone();
                        app.scene.add_entity(entity);
                        app.select_single_entity(Some(id));
                        ui.close_menu();
                    }
                    if ui.button("🖼 Sprite").clicked() {
                        app.push_undo_state();
                        let mut entity = Entity::new("Sprite");
                        entity.add_component(Component::Sprite(Sprite::default()));
                        let id = entity.id.clone();
                        app.scene.add_entity(entity);
                        app.select_single_entity(Some(id));
                        ui.close_menu();
                    }
                    if ui.button("📷 Câmera").clicked() {
                        app.push_undo_state();
                        let mut entity = Entity::new("Camera");
                        entity.add_component(Component::Camera2D(Camera2D::default()));
                        let id = entity.id.clone();
                        app.scene.add_entity(entity);
                        app.select_single_entity(Some(id));
                        ui.close_menu();
                    }
                });
            }

            // Aplicar ações
            if let Some((ids, label)) = to_delete {
                app.delete_confirmation = Some(DeleteTarget::Entities { ids, label });
            }
        });
}


fn show_add_component_menu(ui: &mut egui::Ui, app: &mut EditorApp, entity_id: &str) {
    ui.label(egui::RichText::new("Adicionar componente").small().strong());
    ui.separator();

    ui.menu_button("🎮 Gameplay", |ui| {
        component_menu_button(ui, app, entity_id, "💨 Velocity", Component::Velocity(Velocity::default()));
        component_menu_button(ui, app, entity_id, "⚽ RigidBody2D", Component::RigidBody2D(RigidBody2D::default()));
        component_menu_button(ui, app, entity_id, "📐 BoxCollider", Component::BoxCollider(BoxCollider::default()));
        component_menu_button(ui, app, entity_id, "🎞 Animator", Component::Animator(Animator::default()));
    });

    ui.menu_button("🧠 Scripts", |ui| {
        component_menu_button(ui, app, entity_id, "📜 Script RS2", Component::Script(Script { file_path: String::new() }));
        component_menu_button(ui, app, entity_id, "🌙 LuaScript", Component::LuaScript(LuaScript { file_path: String::new() }));
    });

    ui.menu_button("🖼 Visual", |ui| {
        component_menu_button(ui, app, entity_id, "🖼 Sprite", Component::Sprite(Sprite::default()));
        component_menu_button(ui, app, entity_id, "🔤 TextLabel", Component::TextLabel(TextLabel::default()));
        component_menu_button(ui, app, entity_id, "🔘 UIButton", Component::UIButton(UIButton::default()));
        component_menu_button(ui, app, entity_id, "🔊 Audio", Component::Audio(Audio::default()));
    });

    ui.menu_button("📷 Cena", |ui| {
        component_menu_button(ui, app, entity_id, "📷 Camera2D", Component::Camera2D(Camera2D::default()));
    });
}

fn component_menu_button(ui: &mut egui::Ui, app: &mut EditorApp, entity_id: &str, label: &str, component: Component) {
    let already_has = app
        .find_entity_mut(entity_id)
        .map(|entity| entity.components.iter().any(|existing| std::mem::discriminant(existing) == std::mem::discriminant(&component)))
        .unwrap_or(false);

    let response = ui.add_enabled(!already_has, egui::Button::new(label));
    if response.clicked() {
        app.push_undo_state();
        if let Some(entity) = app.find_entity_mut(entity_id) {
            entity.add_component(component);
        }
        ui.close_menu();
    }

    if already_has {
        response.on_hover_text("Esta entidade já possui esse componente.");
    }
}

/// Renderiza um nó da hierarquia (recursivamente para filhos)
fn show_entity_node(
    ui: &mut egui::Ui,
    entity: &Entity,
    app: &mut EditorApp,
    depth: usize,
    to_delete: &mut Option<(Vec<String>, String)>,
) {
    let indent = depth as f32 * 14.0;
    let is_selected = app.is_entity_selected(&entity.id);
    let ctrl_pressed = ui.ctx().input(|i| i.modifiers.command || i.modifiers.ctrl);

    ui.horizontal(|ui| {
        ui.add_space(indent);

        // Ícone de visibilidade funcional
        let vis_icon = if entity.visible { "👁" } else { "🙈" };
        if ui.small_button(vis_icon).clicked() {
            app.push_undo_state();
            if let Some(current) = app.find_entity_mut(&entity.id) {
                current.visible = !current.visible;
            }
        }

        // Ícone do tipo da entidade
        let obj_icon = entity_icon(entity);
        let mut label_text = format!("{} {}", obj_icon, entity.name);
        if entity.matr_source.is_some() {
            label_text.push_str(" [MATR]");
        }

        let response = ui.add(egui::SelectableLabel::new(is_selected, label_text));

        // Clique esquerdo → seleciona ou adiciona à seleção múltipla
        if response.clicked() {
            if ctrl_pressed {
                app.toggle_entity_selection(entity.id.clone());
            } else {
                app.select_single_entity(Some(entity.id.clone()));
            }
        }

        // Clique direito → menu de contexto
        response.context_menu(|ui| {
            if ui.button(if entity.visible { "🙈 Ocultar" } else { "👁 Mostrar" }).clicked() {
                app.push_undo_state();
                if let Some(current) = app.find_entity_mut(&entity.id) {
                    current.visible = !current.visible;
                }
                ui.close_menu();
            }

            ui.separator();
            ui.label(egui::RichText::new("Criar como filho").small().strong());

            if ui.button("🔷 Entidade filha").clicked() {
                app.push_undo_state();
                if let Some(parent) = app.find_entity_mut(&entity.id) {
                    parent.add_child(Entity::new("Entidade"));
                }
                ui.close_menu();
            }

            if ui.button("🖼 Sprite filho").clicked() {
                app.push_undo_state();
                if let Some(parent) = app.find_entity_mut(&entity.id) {
                    let mut child = Entity::new("Sprite");
                    child.add_component(Component::Sprite(Sprite::default()));
                    parent.add_child(child);
                }
                ui.close_menu();
            }

            if ui.button("📷 Câmera filha").clicked() {
                app.push_undo_state();
                if let Some(parent) = app.find_entity_mut(&entity.id) {
                    let mut child = Entity::new("Camera");
                    child.add_component(Component::Camera2D(Camera2D::default()));
                    parent.add_child(child);
                }
                ui.close_menu();
            }

            ui.separator();
            show_add_component_menu(ui, app, &entity.id);

            ui.separator();

            if ui.button("🗑 Deletar").clicked() {
                if app.is_entity_selected(&entity.id) && app.selected_entity_ids.len() > 1 {
                    *to_delete = Some((
                        app.selected_entity_ids.clone(),
                        format!("{} entidades selecionadas", app.selected_entity_ids.len()),
                    ));
                } else {
                    *to_delete = Some((
                        vec![entity.id.clone()],
                        format!("a entidade '{}'", entity.name),
                    ));
                }
                ui.close_menu();
            }
        });
    });

    // Filhos recursivos
    for child in &entity.children {
        show_entity_node(ui, child, app, depth + 1, to_delete);
    }
}

fn entity_icon(entity: &Entity) -> &'static str {
    if entity.matr_source.is_some() {
        return "🧱";
    }

    for component in &entity.components {
        match component {
            Component::Camera2D(_) => return "📷",
            Component::Sprite(_) => return "🖼",
            Component::Script(_) => return "📜",
            Component::LuaScript(_) => return "🌙",
            Component::Audio(_) => return "🔊",
            Component::Animator(_) => return "🎞",
            Component::TextLabel(_) => return "🔤",
            Component::UIButton(_) => return "🔘",
            Component::RigidBody2D(_) => return "⚙",
            Component::Velocity(_) => return "💨",
            Component::BoxCollider(_) => return "📐",
            Component::Transform(_) => {}
        }
    }

    if !entity.children.is_empty() {
        "📦"
    } else {
        "🔷"
    }
}
