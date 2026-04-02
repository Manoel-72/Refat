// ============================================================
//  editor/asset_browser.rs
//  Painel de Assets — exibe arquivos/pastas do projeto com
//  busca, importação, drag-and-drop e suporte a MATRs.
// ============================================================

use eframe::egui;
use rfd::FileDialog;
use std::path::{Path, PathBuf};

use crate::engine::{
    assets::AssetNode,
    component::{Camera2D, Component},
    entity::Entity,
};
use super::EditorApp;

pub fn show(app: &mut EditorApp, ui: &mut egui::Ui) {
    let assets_root = app.project_root.join("assets");

    ui.horizontal_wrapped(|ui| {
        ui.heading("📂 Assets");
        ui.separator();
        ui.label("Buscar:");
        ui.add(
            egui::TextEdit::singleline(&mut app.asset_search)
                .hint_text("sprite, MATR, script rs2...")
                .desired_width(180.0),
        );

        if ui.button("📥 Importar Sprite").clicked() {
            import_sprite_file(app, &assets_root);
        }
        if ui.button("📜 Script RS2").clicked() {
            app.new_script_dialog = Some((assets_root.clone(), "meu_script".to_string()));
        }
        if ui.button("📁 Pasta").clicked() {
            app.new_folder_dialog = Some((assets_root.clone(), "nova_pasta".to_string()));
        }
        if ui.button("🔷 Entidade").clicked() {
            app.new_entity_dialog = Some("Entidade".to_string());
        }
        if ui.button("🔄 Recarregar").clicked() {
            app.assets.refresh();
        }
    });

    ui.label(
        egui::RichText::new(
            "Dica: clique direito para criar/deletar/renomear e arraste imagens ou MATRs direto para a cena.",
        )
        .small()
        .weak(),
    );
    ui.separator();

    ui.columns(2, |columns| {
        // ── Árvore de assets ──
        columns[0].vertical(|ui| {
            let blank_response = ui.allocate_response(
                egui::vec2(ui.available_width(), 6.0),
                egui::Sense::click(),
            );

            let mut action: Option<AssetAction> = None;
            blank_response.context_menu(|ui| {
                ui.label(egui::RichText::new("Raiz de assets").strong());
                ui.separator();
                if ui.button("📥 Importar Sprite").clicked() {
                    action = Some(AssetAction::ImportSprite);
                    ui.close_menu();
                }
                if ui.button("📜 Criar Script RS2 (.rs2)").clicked() {
                    action = Some(AssetAction::NewScript(assets_root.clone()));
                    ui.close_menu();
                }
                if ui.button("📁 Criar Pasta").clicked() {
                    action = Some(AssetAction::NewFolder(assets_root.clone()));
                    ui.close_menu();
                }
                if ui.button("🔷 Criar Entidade").clicked() {
                    action = Some(AssetAction::CreateEntity);
                    ui.close_menu();
                }
                if ui.button("📷 Criar Câmera").clicked() {
                    action = Some(AssetAction::CreateCamera);
                    ui.close_menu();
                }
            });

            egui::ScrollArea::both()
                .id_source("asset_browser_scroll")
                .show(ui, |ui: &mut egui::Ui| {
                    if let Some(tree) = app.assets.tree.clone() {
                        show_node(
                            ui,
                            &tree,
                            &app.selected_asset,
                            &app.asset_search,
                            &mut action,
                            &mut app.dragging_asset_path,
                        );
                    } else {
                        ui.label("Pasta 'assets' não encontrada.");
                    }
                });

            apply_asset_action(app, action, &assets_root);
        });

        // ── Painel de detalhes do asset selecionado ──
        columns[1].vertical(|ui| {
            show_selected_asset_panel(app, ui, &assets_root);
        });
    });
}

/// Ações disparadas pelo clique do usuário
#[derive(Debug, Clone)]
enum AssetAction {
    Select(PathBuf),
    NewScript(PathBuf),
    NewFolder(PathBuf),
    CreateEntity,
    CreateCamera,
    CreateSpriteFromAsset(PathBuf),
    InstantiateMatr(PathBuf),
    Rename(PathBuf),
    Delete(PathBuf),
    ImportSprite,
    Refresh,
}

fn apply_asset_action(app: &mut EditorApp, action: Option<AssetAction>, assets_root: &Path) {
    match action {
        Some(AssetAction::Select(path)) => {
            app.selected_asset = Some(path.clone());
            app.status_msg = format!(
                "Asset selecionado: {:?}",
                path.file_name().unwrap_or_default()
            );
        }
        Some(AssetAction::NewScript(folder)) => {
            app.new_script_dialog = Some((folder, "meu_script".to_string()));
        }
        Some(AssetAction::NewFolder(parent)) => {
            app.new_folder_dialog = Some((parent, "nova_pasta".to_string()));
        }
        Some(AssetAction::CreateEntity) => {
            app.new_entity_dialog = Some("Entidade".to_string());
        }
        Some(AssetAction::CreateCamera) => {
            app.push_undo_state();
            let mut entity = Entity::new("Camera");
            entity.add_component(Component::Camera2D(Camera2D::default()));
            let id = entity.id.clone();
            app.scene.add_entity(entity);
            app.select_single_entity(Some(id));
            app.status_msg = "📷 Câmera adicionada à cena.".to_string();
        }
        Some(AssetAction::CreateSpriteFromAsset(path)) => {
            match app.create_sprite_entity_from_asset(&path, Some((0.0, 0.0))) {
                Ok(name) => app.status_msg = format!("🖼 Sprite '{}' adicionado à cena.", name),
                Err(e) => app.status_msg = format!("❌ {}", e),
            }
        }
        Some(AssetAction::InstantiateMatr(path)) => {
            match app.instantiate_matr_from_path(&path, Some((0.0, 0.0))) {
                Ok(name) => app.status_msg = format!("🧱 MATR '{}' instanciado.", name),
                Err(e) => app.status_msg = format!("❌ {}", e),
            }
        }
        Some(AssetAction::Rename(path)) => {
            app.request_rename_asset(path);
        }
        Some(AssetAction::Delete(path)) => {
            app.request_delete_asset(path);
        }
        Some(AssetAction::ImportSprite) => {
            import_sprite_file(app, assets_root);
        }
        Some(AssetAction::Refresh) => {
            app.assets.refresh();
        }
        None => {}
    }
}

/// Renderiza um nó da árvore de assets recursivamente
fn show_node(
    ui: &mut egui::Ui,
    node: &AssetNode,
    selected: &Option<PathBuf>,
    filter: &str,
    action: &mut Option<AssetAction>,
    dragging_asset: &mut Option<PathBuf>,
) {
    if !matches_filter(node, filter) {
        return;
    }

    let is_selected = selected.as_deref() == Some(&node.path);

    if node.is_dir {
        let header = format!("{} {}", node.icon(), node.name);
        let node_path = node.path.clone();
        let node_name = node.name.clone();
        let children = node.children.clone();

        let response = egui::CollapsingHeader::new(&header)
            .id_source(node_path.to_string_lossy().to_string())
            .default_open(filter.trim().is_empty())
            .show(ui, |ui: &mut egui::Ui| {
                for child in &children {
                    show_node(ui, child, selected, filter, action, dragging_asset);
                }
            });

        if response.header_response.clicked() {
            *action = Some(AssetAction::Select(node_path.clone()));
        }

        response.header_response.context_menu(|ui: &mut egui::Ui| {
            ui.label(egui::RichText::new(&node_name).strong());
            ui.separator();
            if ui.button("📥 Importar Sprite").clicked() {
                *action = Some(AssetAction::ImportSprite);
                ui.close_menu();
            }
            if ui.button("📜 Criar Script RS2 (.rs2)").clicked() {
                *action = Some(AssetAction::NewScript(node_path.clone()));
                ui.close_menu();
            }
            if ui.button("📁 Criar Pasta").clicked() {
                *action = Some(AssetAction::NewFolder(node_path.clone()));
                ui.close_menu();
            }
            if ui.button("🔷 Criar Entidade").clicked() {
                *action = Some(AssetAction::CreateEntity);
                ui.close_menu();
            }
            if ui.button("📷 Criar Câmera").clicked() {
                *action = Some(AssetAction::CreateCamera);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("✏ Renomear Pasta").clicked() {
                *action = Some(AssetAction::Rename(node_path.clone()));
                ui.close_menu();
            }
            if ui.button("🗑 Deletar Pasta").clicked() {
                *action = Some(AssetAction::Delete(node_path.clone()));
                ui.close_menu();
            }
            if ui.button("🔄 Recarregar").clicked() {
                *action = Some(AssetAction::Refresh);
                ui.close_menu();
            }
        });
    } else {
        let node_path = node.path.clone();
        let label = format!("{} {}", node.icon(), node.name);
        let can_drag = is_image_file(&node_path) || is_matr_file(&node_path);

        ui.horizontal(|ui: &mut egui::Ui| {
            ui.add_space(8.0);

            let response = ui.add(
                egui::Button::new(&label)
                    .selected(is_selected)
                    .sense(egui::Sense::click_and_drag()),
            );

            if response.clicked() {
                *action = Some(AssetAction::Select(node_path.clone()));
            }

            if can_drag && response.drag_started() {
                *dragging_asset = Some(node_path.clone());
            }

            response.context_menu(|ui: &mut egui::Ui| {
                                if node_path.extension().and_then(|e| e.to_str()) == Some("rs") {
                                    if ui.button("📝 Abrir em IDE").clicked() {
                                        #[cfg(target_os = "windows")]
                                        {
                                            let _ = std::process::Command::new("cmd").args(["/C", "code", &node_path.to_string_lossy()]).spawn();
                                        }
                                        ui.close_menu();
                                    }
                                    ui.separator();
                                }
                if is_image_file(&node_path) {
                    if ui.button("🖼 Criar Entidade Sprite").clicked() {
                        *action = Some(AssetAction::CreateSpriteFromAsset(node_path.clone()));
                        ui.close_menu();
                    }
                    ui.separator();
                }

                if is_matr_file(&node_path) {
                    if ui.button("🧱 Instanciar MATR").clicked() {
                        *action = Some(AssetAction::InstantiateMatr(node_path.clone()));
                        ui.close_menu();
                    }
                    ui.separator();
                }

                if ui.button("✏ Renomear").clicked() {
                    *action = Some(AssetAction::Rename(node_path.clone()));
                    ui.close_menu();
                }

                if ui.button("📋 Copiar caminho").clicked() {
                    ui.output_mut(|o| {
                        o.copied_text = node_path.to_string_lossy().to_string();
                    });
                    ui.close_menu();
                }

                if ui.button("🗑 Deletar").clicked() {
                    *action = Some(AssetAction::Delete(node_path.clone()));
                    ui.close_menu();
                }
            });
        });
    }
}

fn show_selected_asset_panel(app: &mut EditorApp, ui: &mut egui::Ui, assets_root: &Path) {
    ui.group(|ui| {
        ui.heading("🧭 Detalhes do Asset");

        let selected = app.selected_asset.clone();
        let Some(path) = selected else {
            ui.label("Selecione um arquivo ou pasta para ver detalhes.");
            ui.label("• imagens podem virar entidades Sprite");
            ui.label("• MATRs podem ser arrastados para a cena");
            ui.label("• clique direito para criar scripts RS2, pastas e deletar");
            return;
        };

        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let relative = app
            .assets
            .relative_path(&path)
            .map(|p| p.to_string_lossy().replace('\\', "/"));

        ui.label(egui::RichText::new(name).strong());
        ui.label(if path.is_dir() { "Tipo: pasta" } else { "Tipo: arquivo" });

        if let Some(rel) = &relative {
            ui.label("Caminho relativo:");
            ui.monospace(rel);
        }

        ui.add_space(6.0);

        if path.is_dir() {
            ui.horizontal_wrapped(|ui| {
                if ui.button("📜 Criar Script RS2").clicked() {
                    app.new_script_dialog = Some((path.clone(), "meu_script".to_string()));
                }
                if ui.button("📁 Criar Pasta").clicked() {
                    app.new_folder_dialog = Some((path.clone(), "nova_pasta".to_string()));
                }
                if ui.button("📥 Importar Sprite").clicked() {
                    import_sprite_file(app, assets_root);
                }
                if ui.button("✏ Renomear Pasta").clicked() {
                    app.request_rename_asset(path.clone());
                }
                if ui.button("🗑 Deletar Pasta").clicked() {
                    app.request_delete_asset(path.clone());
                }
            });
            return;
        }

        if is_image_file(&path) {
            if let Some(rel) = &relative {
                if let Some(texture) = app.load_texture_from_relative_path(ui.ctx(), rel) {
                    let tex_size = texture.size_vec2();
                    let scale = (ui.available_width() / tex_size.x)
                        .min(160.0 / tex_size.y)
                        .min(1.0);
                    ui.image((texture.id(), tex_size * scale.max(0.1)));
                } else {
                    ui.label("Pré-visualização indisponível para esta imagem.");
                }
            }

            ui.horizontal_wrapped(|ui| {
                if ui.button("🖼 Instanciar Sprite na Cena").clicked() {
                    match app.create_sprite_entity_from_asset(&path, Some((0.0, 0.0))) {
                        Ok(name) => app.status_msg = format!("🖼 Sprite '{}' adicionado à cena.", name),
                        Err(e) => app.status_msg = format!("❌ {}", e),
                    }
                }

                if ui.button("📥 Importar Outro Sprite").clicked() {
                    import_sprite_file(app, assets_root);
                }
            });
        }

        if is_matr_file(&path) {
            ui.label("Este arquivo é um MATR reutilizável da cena.");
            if ui.button("🧱 Instanciar MATR na Cena").clicked() {
                match app.instantiate_matr_from_path(&path, Some((0.0, 0.0))) {
                    Ok(name) => app.status_msg = format!("🧱 MATR '{}' instanciado.", name),
                    Err(e) => app.status_msg = format!("❌ {}", e),
                }
            }
        }

        ui.horizontal_wrapped(|ui| {
            if ui.button("📋 Copiar caminho").clicked() {
                let copied = relative.unwrap_or_else(|| path.to_string_lossy().to_string());
                ui.output_mut(|o| {
                    o.copied_text = copied;
                });
            }

            if ui.button("✏ Renomear").clicked() {
                app.request_rename_asset(path.clone());
            }

            if ui.button("🗑 Deletar Asset").clicked() {
                app.request_delete_asset(path.clone());
            }

            if ui.button("🔄 Recarregar Assets").clicked() {
                app.assets.refresh();
            }
        });
    });
}

fn import_sprite_file(app: &mut EditorApp, assets_root: &Path) {
    let Some(source_path) = FileDialog::new()
        .add_filter("Imagens", &["png", "jpg", "jpeg", "webp"])
        .pick_file()
    else {
        return;
    };

    let target_dir = assets_root.join("sprites");
    match app.assets.import_file(&source_path, &target_dir) {
        Ok(imported_path) => {
            app.assets.refresh();
            app.selected_asset = Some(imported_path.clone());

            match app.create_sprite_entity_from_asset(&imported_path, Some((0.0, 0.0))) {
                Ok(name) => {
                    app.status_msg = format!(
                        "📥 Sprite importado e instanciado para teste: '{}'",
                        name
                    );
                }
                Err(e) => {
                    app.status_msg = format!("⚠ Sprite importado, mas não foi instanciado: {}", e);
                }
            }
        }
        Err(e) => {
            app.status_msg = format!("❌ Erro ao importar sprite: {}", e);
        }
    }
}

fn matches_filter(node: &AssetNode, filter: &str) -> bool {
    let query = filter.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }

    node.name.to_lowercase().contains(&query)
        || node.children.iter().any(|child| matches_filter(child, filter))
}

fn is_image_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()),
        Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp")
    )
}

fn is_matr_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.ends_with(".matr.json") || n.ends_with(".prefab.json"))
        .unwrap_or(false)
}
