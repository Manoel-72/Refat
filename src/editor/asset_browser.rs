// ============================================================
//  editor/asset_browser.rs
//  Painel de Assets — exibe arquivos/pastas do projeto com
//  busca, importação, drag-and-drop e suporte a MATRs.
// ============================================================

use eframe::egui;
use rfd::FileDialog;
use std::fs;
use std::path::{Path, PathBuf};

use super::{AssetBrowserFilter, EditorApp};
use crate::{
    assets::{AssetLoadStatus, AssetNode, AssetType},
    component::{Camera2D, Component},
    entity::Entity,
};

pub fn show(app: &mut EditorApp, ui: &mut egui::Ui) {
    // Namespace estável: evita colisão de IDs com Animator/Console no mesmo painel inferior.
    ui.push_id("asset_browser_dock", |ui| {
    let assets_root = app.project_root.join("assets");

    // ── Cabeçalho + Busca ──
    ui.horizontal_wrapped(|ui| {
        ui.heading("Project");
        ui.separator();
        ui.label("Search:");
        ui.add(
            egui::TextEdit::singleline(&mut app.asset_search)
                .hint_text("sprite, MATR, script rs2/lua...")
                .desired_width(160.0),
        );
        if ui.button("✖").on_hover_text("Limpar busca").clicked() {
            app.asset_search.clear();
            app.asset_filter = AssetBrowserFilter::All;
        }
    });

    // ── Grupo "Criar" ──
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("Create:").weak().small());
        if ui.small_button("🎬 Cena").clicked() {
            match app.assets.create_scene_file(&assets_root.join("scenes"), "nova_cena") {
                Ok(path) => {
                    app.assets.refresh();
                    app.selected_asset = Some(path.clone());
                    app.status_msg = format!("🎬 Cena criada: {}", path.file_name().and_then(|n| n.to_str()).unwrap_or("nova_cena.scene.json"));
                }
                Err(error) => app.status_msg = format!("❌ {}", error),
            }
        }
        if ui.small_button("📜 Script RS2").clicked() {
            app.new_script_dialog = Some((assets_root.join("scripts"), "meu_script".to_string()));
        }
        if ui.small_button("🌙 Script Lua").clicked() {
            app.new_lua_dialog = Some((assets_root.join("scripts"), "novo_script".to_string()));
        }
        if ui.small_button("📁 Pasta").clicked() {
            app.new_folder_dialog = Some((assets_root.clone(), "nova_pasta".to_string()));
        }
        if ui.small_button("📦 Template").clicked() {
            app.new_template_dialog = Some("MeuProjeto".to_string());
        }

        ui.separator();
        ui.label(egui::RichText::new("Import:").weak().small());
        if ui.small_button("📥 Sprite").clicked() {
            import_sprite_file(app, &assets_root);
        }
        if ui.small_button("🔄").on_hover_text("Recarregar assets").clicked() {
            app.assets.refresh();
            app.status_msg = "✅ Assets recarregados.".to_string();
        }
    });

    ui.horizontal_wrapped(|ui| {
        ui.label("Quick filters:");
        for (label, filter) in quick_filters() {
            let selected = app.asset_filter == filter;
            if ui.selectable_label(selected, label).clicked() {
                app.asset_filter = filter;
            }
        }

        let total_assets = app.assets.list_asset_records().len();
        ui.separator();
        ui.small(format!("{} registros", total_assets));
    });

    ui.label(egui::RichText::new("Asset cards view: clique para selecionar, duplo clique para abrir pasta, clique direito para ações.").small().weak());

    if let Some(selected_asset) = app.selected_asset.clone() {
        let record = app.assets.asset_record_for(&selected_asset);
        let (color, status_label) = if record.validation.is_valid {
            (egui::Color32::from_rgb(120, 220, 140), "válido")
        } else {
            (egui::Color32::from_rgb(255, 120, 120), "com problema")
        };
        ui.horizontal_wrapped(|ui| {
            ui.small("Selecionado:");
            ui.colored_label(color, format!("{} ({})", record.name, status_label));
            if !record.validation.is_valid {
                ui.label(egui::RichText::new(&record.validation.message).small().weak());
            }
        });
    }
    ui.separator();

    let mut action: Option<AssetAction> = None;
    let scope_dir = current_asset_scope_dir(app, &assets_root);
    let mut scope_entries = read_scope_entries(&scope_dir);
    scope_entries.retain(|path| matches_scope_filter(path, &app.asset_search, app.asset_filter));

    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("Location:").small().weak());
        let rel = scope_dir
            .strip_prefix(&assets_root)
            .ok()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| "assets".to_string());
        ui.monospace(rel);

        if scope_dir != assets_root {
            ui.separator();
            if ui.small_button("Up").clicked() {
                if let Some(parent) = scope_dir.parent() {
                    action = Some(AssetAction::Select(parent.to_path_buf()));
                }
            }
        }
    });
    ui.add_space(6.0);

    // Altura do grid nunca pode ser derivada de "available_height" sem teto: no primeiro
    // layout do TopBottomPanel o valor pode ser enorme e o painel "come" o CentralPanel.
    const MAX_CARD_GRID_H: f32 = 520.0;
    let grid_h = (ui.available_height() - 110.0).clamp(140.0, MAX_CARD_GRID_H);

    ui.vertical(|ui| {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_source(egui::Id::new("asset_browser_card_grid_scroll"))
                .max_height(grid_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let available_w = ui.available_width().max(280.0);
                    let estimated_cols = (available_w / 118.0).floor().max(1.0);
                    let card_width = ((available_w / estimated_cols) - 12.0).clamp(92.0, 132.0);
                    let card_height = (card_width * 0.72).clamp(68.0, 96.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
                        if scope_entries.is_empty() {
                            ui.label(egui::RichText::new("Nenhum asset encontrado nesse filtro.").small().weak());
                        }
                        for entry in scope_entries {
                            let is_dir = entry.is_dir();
                            let selected = app.selected_asset.as_deref() == Some(entry.as_path());
                            let can_drag = !is_dir
                                && (is_image_file(&entry)
                                    || is_matr_file(&entry)
                                    || is_rs2_file(&entry)
                                    || is_lua_file(&entry));
                            let name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("asset");
                            let icon = if is_dir {
                                "📁"
                            } else if is_image_file(&entry) {
                                "🖼"
                            } else if is_matr_file(&entry) {
                                "🧱"
                            } else if is_rs2_file(&entry) || is_lua_file(&entry) {
                                "📄"
                            } else {
                                "📦"
                            };
                            let card_text = egui::RichText::new(format!("{}\n{}", icon, name)).size(13.0);
                            let response = ui.add_sized(
                                egui::vec2(card_width, card_height),
                                egui::Button::new(card_text)
                                    .selected(selected)
                                    .sense(egui::Sense::click_and_drag()),
                            );

                            if response.clicked() {
                                action = Some(AssetAction::Select(entry.clone()));
                            }
                            if response.double_clicked() {
                                if is_dir {
                                    action = Some(AssetAction::Select(entry.clone()));
                                } else {
                                    let ext = entry.extension().and_then(|e| e.to_str()).unwrap_or("");
                                    if matches!(ext, "rs2" | "lua") {
                                        #[cfg(target_os = "windows")]
                                        {
                                            let _ = std::process::Command::new("cmd")
                                                .args(["/C", "code", &entry.to_string_lossy()])
                                                .spawn();
                                        }
                                    }
                                }
                            }
                            if can_drag && response.drag_started() {
                                app.dragging_asset_path = Some(entry.clone());
                            }
                            response.context_menu(|ui| {
                                if is_dir {
                                    if ui.button("📁 Open folder").clicked() {
                                        action = Some(AssetAction::Select(entry.clone()));
                                        ui.close_menu();
                                    }
                                    if ui.button("📜 New Script RS2").clicked() {
                                        action = Some(AssetAction::NewScript(entry.clone()));
                                        ui.close_menu();
                                    }
                                    if ui.button("🌙 New Lua Script").clicked() {
                                        action = Some(AssetAction::NewLuaScript(entry.clone()));
                                        ui.close_menu();
                                    }
                                } else {
                                    if is_image_file(&entry) && ui.button("🖼 Create Sprite Entity").clicked() {
                                        action = Some(AssetAction::CreateSpriteFromAsset(entry.clone()));
                                        ui.close_menu();
                                    }
                                    if is_matr_file(&entry) && ui.button("🧱 Instantiate MATR").clicked() {
                                        action = Some(AssetAction::InstantiateMatr(entry.clone()));
                                        ui.close_menu();
                                    }
                                }
                                ui.separator();
                                if ui.button("✏ Rename").clicked() {
                                    action = Some(AssetAction::Rename(entry.clone()));
                                    ui.close_menu();
                                }
                                if ui.button("📄 Duplicate").clicked() {
                                    action = Some(AssetAction::Duplicate(entry.clone()));
                                    ui.close_menu();
                                }
                                if ui.button("🗑 Delete").clicked() {
                                    action = Some(AssetAction::Delete(entry.clone()));
                                    ui.close_menu();
                                }
                            });
                        }
                    });
                });
        });

        ui.add_space(6.0);

        let details_max = ui.available_height().max(96.0).min(320.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_source(egui::Id::new("asset_browser_details_scroll"))
                .max_height(details_max)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    show_selected_asset_panel(app, ui, &assets_root);
                });
        });
    });

    apply_asset_action(app, action, &assets_root);
    });
}

fn current_asset_scope_dir(app: &EditorApp, assets_root: &Path) -> PathBuf {
    match app.selected_asset.as_ref() {
        Some(path) if path.is_dir() => path.clone(),
        Some(path) => path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| assets_root.to_path_buf()),
        None => assets_root.to_path_buf(),
    }
}

fn read_scope_entries(scope_dir: &Path) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(scope_dir) {
        for entry in read_dir.flatten() {
            entries.push(entry.path());
        }
    }
    entries.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_lowercase()
            .cmp(
                &b.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_lowercase(),
            ),
    });
    entries
}

fn matches_scope_filter(path: &Path, search: &str, asset_filter: AssetBrowserFilter) -> bool {
    let query = search.trim().to_lowercase();
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let search_ok = query.is_empty() || name.contains(&query);
    let filter_ok = path.is_dir() || asset_matches_filter(path, asset_filter);
    search_ok && filter_ok
}

/// Ações disparadas pelo clique do usuário
#[allow(dead_code)]
#[derive(Debug, Clone)]
enum AssetAction {
    Select(PathBuf),
    NewScript(PathBuf),
    NewLuaScript(PathBuf),
    NewFolder(PathBuf),
    NewScene(PathBuf),
    CreateEntity,
    CreateCamera,
    CreateSpriteFromAsset(PathBuf),
    InstantiateMatr(PathBuf),
    Rename(PathBuf),
    Duplicate(PathBuf),
    Delete(PathBuf),
    ImportSprite,
    CreateBasicTemplate,
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
        Some(AssetAction::NewLuaScript(folder)) => {
            app.new_lua_dialog = Some((folder, "novo_script".to_string()));
        }
        Some(AssetAction::NewFolder(parent)) => {
            app.new_folder_dialog = Some((parent, "nova_pasta".to_string()));
        }
        Some(AssetAction::NewScene(parent)) => {
            match app.assets.create_scene_file(&parent, "nova_cena") {
                Ok(path) => {
                    app.assets.refresh();
                    app.selected_asset = Some(path.clone());
                    app.status_msg = format!(
                        "🎬 Cena criada: {}",
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("nova_cena.scene.json")
                    );
                }
                Err(error) => app.status_msg = format!("❌ {}", error),
            }
        }
        Some(AssetAction::CreateEntity) => {
            app.new_entity_dialog = Some("Entidade".to_string());
        }
        Some(AssetAction::CreateBasicTemplate) => {
            app.new_template_dialog = Some("MeuProjeto".to_string());
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
        Some(AssetAction::Duplicate(path)) => match app.assets.duplicate_path(&path) {
            Ok(new_path) => {
                app.assets.refresh();
                app.selected_asset = Some(new_path.clone());
                app.status_msg = format!(
                    "📄 Duplicado: {}",
                    new_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("asset")
                );
            }
            Err(error) => app.status_msg = format!("❌ {}", error),
        },
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
#[allow(dead_code)]
fn show_node(
    ui: &mut egui::Ui,
    node: &AssetNode,
    selected: &Option<PathBuf>,
    search: &str,
    asset_filter: AssetBrowserFilter,
    action: &mut Option<AssetAction>,
    dragging_asset: &mut Option<PathBuf>,
) {
    if !matches_filter(node, search, asset_filter) {
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
            .default_open(
                search.trim().is_empty() && matches!(asset_filter, AssetBrowserFilter::All),
            )
            .show(ui, |ui: &mut egui::Ui| {
                for child in &children {
                    show_node(
                        ui,
                        child,
                        selected,
                        search,
                        asset_filter,
                        action,
                        dragging_asset,
                    );
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
            if ui.button("🎬 Criar Cena (.scene.json)").clicked() {
                *action = Some(AssetAction::NewScene(node_path.clone()));
                ui.close_menu();
            }
            if ui.button("📜 Criar Script RS2 (.rs2)").clicked() {
                *action = Some(AssetAction::NewScript(node_path.clone()));
                ui.close_menu();
            }
            if ui.button("🌙 Criar Script Lua (.lua)").clicked() {
                *action = Some(AssetAction::NewLuaScript(node_path.clone()));
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
        let can_drag = is_image_file(&node_path)
            || is_matr_file(&node_path)
            || is_rs2_file(&node_path)
            || is_lua_file(&node_path);

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

            // Duplo clique em script (.rs2 ou .lua) abre no VS Code
            if response.double_clicked() {
                let ext = node_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "rs2" | "lua") {
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("cmd")
                            .args(["/C", "code", &node_path.to_string_lossy()])
                            .spawn();
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        let _ = std::process::Command::new("code").arg(&node_path).spawn();
                    }
                }
            }

            if can_drag && response.drag_started() {
                *dragging_asset = Some(node_path.clone());
            }

            response.context_menu(|ui: &mut egui::Ui| {
                if node_path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if ui.button("📝 Abrir em IDE").clicked() {
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("cmd")
                                .args(["/C", "code", &node_path.to_string_lossy()])
                                .spawn();
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

                if ui.button("📄 Duplicar").clicked() {
                    *action = Some(AssetAction::Duplicate(node_path.clone()));
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
        let record = app.assets.asset_record_for(&path);
        let tipo_texto = match record.asset_type {
            AssetType::Folder => "pasta",
            AssetType::Scene => "cena",
            AssetType::Prefab => "prefab",
            AssetType::Texture => "textura",
            AssetType::Audio => "áudio",
            AssetType::Font => "fonte",
            AssetType::ScriptRs2 => "script RS2",
            AssetType::ScriptLua => "script Lua",
            AssetType::Json => "json",
            AssetType::Unknown => "desconhecido",
        };
        ui.label(format!("Tipo: {}", tipo_texto));
        let status_text = match record.load_status {
            AssetLoadStatus::Ready => "Ready",
            AssetLoadStatus::NotLoaded => "NotLoaded",
            AssetLoadStatus::Missing => "Missing",
            AssetLoadStatus::Invalid => "Invalid",
        };
        ui.label(format!("Status: {}", status_text));
        if !record.validation.is_valid {
            ui.colored_label(
                egui::Color32::YELLOW,
                format!("⚠ {}", record.validation.message),
            );
        } else {
            ui.colored_label(egui::Color32::LIGHT_GREEN, "✔ Validação OK");
        }

        if let Some(rel) = &relative {
            ui.label("Caminho relativo:");
            ui.monospace(rel);
        }

        ui.add_space(6.0);

        if path.is_dir() {
            ui.horizontal_wrapped(|ui| {
                if ui.button("🎬 Criar Cena").clicked() {
                    match app.assets.create_scene_file(&path, "nova_cena") {
                        Ok(new_path) => {
                            app.assets.refresh();
                            app.selected_asset = Some(new_path.clone());
                            app.status_msg = format!(
                                "🎬 Cena criada: {}",
                                new_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("nova_cena.scene.json")
                            );
                        }
                        Err(error) => app.status_msg = format!("❌ {}", error),
                    }
                }
                if ui.button("📜 Criar Script RS2").clicked() {
                    app.new_script_dialog = Some((path.clone(), "meu_script".to_string()));
                }
                if ui.button("🌙 Criar Script Lua").clicked() {
                    app.new_lua_dialog = Some((path.clone(), "novo_script".to_string()));
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
                        Ok(name) => {
                            app.status_msg = format!("🖼 Sprite '{}' adicionado à cena.", name)
                        }
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
            if ui
                .button("🧱 Exportar entidade selecionada como Prefab")
                .clicked()
            {
                if let Some(selected_id) = app.selected_entity_id.clone() {
                    if let Some(entity) = app.scene.find_entity(&selected_id).cloned() {
                        match app.export_entity_as_matr(&entity) {
                            Ok(prefab_path) => {
                                app.assets.refresh();
                                app.status_msg = format!(
                                    "🧱 Prefab exportado: {}",
                                    prefab_path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("prefab.prefab.json")
                                );
                            }
                            Err(error) => app.status_msg = format!("❌ {}", error),
                        }
                    } else {
                        app.status_msg = "❌ Entidade selecionada não encontrada.".to_string();
                    }
                } else {
                    app.status_msg =
                        "❌ Selecione uma entidade para exportar como prefab.".to_string();
                }
            }
            if ui.button("📋 Copiar caminho").clicked() {
                let copied = relative.unwrap_or_else(|| path.to_string_lossy().to_string());
                ui.output_mut(|o| {
                    o.copied_text = copied;
                });
            }

            if ui.button("📄 Duplicar").clicked() {
                match app.assets.duplicate_path(&path) {
                    Ok(new_path) => {
                        app.assets.refresh();
                        app.selected_asset = Some(new_path.clone());
                        app.status_msg = format!(
                            "📄 Duplicado: {}",
                            new_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("asset")
                        );
                    }
                    Err(error) => app.status_msg = format!("❌ {}", error),
                }
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

fn is_rs2_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("rs2"))
        .unwrap_or(false)
}

fn is_lua_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("lua"))
        .unwrap_or(false)
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
                    app.status_msg =
                        format!("📥 Sprite importado e instanciado para teste: '{}'", name);
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

fn quick_filters() -> [(&'static str, AssetBrowserFilter); 7] {
    [
        ("Todos", AssetBrowserFilter::All),
        ("Imagens", AssetBrowserFilter::Images),
        ("Scripts", AssetBrowserFilter::Scripts),
        ("Scenes", AssetBrowserFilter::Scenes),
        ("Prefabs", AssetBrowserFilter::Prefabs),
        ("Áudio", AssetBrowserFilter::Audio),
        ("Fontes", AssetBrowserFilter::Fonts),
    ]
}

#[allow(dead_code)]
fn matches_filter(node: &AssetNode, search: &str, asset_filter: AssetBrowserFilter) -> bool {
    let query = search.trim().to_lowercase();
    let search_matches = query.is_empty() || node.name.to_lowercase().contains(&query);
    let type_matches = node.is_dir || asset_matches_filter(&node.path, asset_filter);

    (search_matches && type_matches)
        || node
            .children
            .iter()
            .any(|child| matches_filter(child, search, asset_filter))
}

fn asset_matches_filter(path: &Path, asset_filter: AssetBrowserFilter) -> bool {
    match asset_filter {
        AssetBrowserFilter::All => true,
        AssetBrowserFilter::Images => is_image_file(path),
        AssetBrowserFilter::Scripts => is_rs2_file(path) || is_lua_file(path),
        AssetBrowserFilter::Scenes => matches!(
            path.file_name().and_then(|n| n.to_str()),
            Some(name) if name.ends_with(".scene.json")
        ),
        AssetBrowserFilter::Prefabs => is_matr_file(path),
        AssetBrowserFilter::Audio => matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .as_deref(),
            Some("wav") | Some("ogg") | Some("mp3")
        ),
        AssetBrowserFilter::Fonts => matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .as_deref(),
            Some("ttf") | Some("otf")
        ),
    }
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
