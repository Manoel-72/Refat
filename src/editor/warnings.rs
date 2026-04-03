use std::{fs, path::Path};

use crate::{
    assets::types::{validate_asset_path, validate_reference_path, AssetType},
    core::{component::Component, entity::Entity, scene::Scene},
    runtime::script,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorWarningKind {
    Scene,
    Sprite,
    Script,
    Prefab,
}

#[derive(Debug, Clone)]
pub struct EditorWarning {
    pub kind: EditorWarningKind,
    pub label: String,
    pub details: String,
}

pub fn collect_scene_warnings(project_root: &Path, scene: &Scene) -> Vec<EditorWarning> {
    let mut warnings = Vec::new();

    if !scene_has_main_camera(scene) {
        warnings.push(EditorWarning {
            kind: EditorWarningKind::Scene,
            label: "Cena sem câmera principal".to_string(),
            details: "Adicione uma Camera2D marcada como principal para o runtime enquadrar a cena corretamente.".to_string(),
        });
    }

    for entity in &scene.entities {
        collect_entity_warnings(project_root, entity, &mut warnings);
    }

    collect_prefab_asset_warnings(project_root, &mut warnings);
    dedupe_warnings(&mut warnings);
    warnings
}

fn scene_has_main_camera(scene: &Scene) -> bool {
    let mut found = false;
    scene.visit_entities(|entity| {
        if entity.components.iter().any(|component| matches!(component, Component::Camera2D(camera) if camera.is_main)) {
            found = true;
        }
    });
    found
}

fn collect_entity_warnings(project_root: &Path, entity: &Entity, warnings: &mut Vec<EditorWarning>) {
    if let Some(prefab_reference) = entity.matr_source.as_deref() {
        let prefab_validation = validate_reference_path(project_root, prefab_reference, AssetType::Prefab);
        if !prefab_validation.is_valid {
            warnings.push(EditorWarning {
                kind: EditorWarningKind::Prefab,
                label: format!("Prefab referenciado inválido em '{}'", entity.name),
                details: prefab_validation.message,
            });
        }
    }

    for component in &entity.components {
        match component {
            Component::Sprite(sprite) => collect_sprite_warning(project_root, entity, sprite.texture_path.trim(), warnings),
            Component::Script(script_component) => collect_script_warning(project_root, entity, script_component.file_path.trim(), warnings),
            _ => {}
        }
    }

    for child in &entity.children {
        collect_entity_warnings(project_root, child, warnings);
    }
}

fn collect_sprite_warning(project_root: &Path, entity: &Entity, texture_path: &str, warnings: &mut Vec<EditorWarning>) {
    if texture_path.is_empty() {
        warnings.push(EditorWarning {
            kind: EditorWarningKind::Sprite,
            label: format!("Sprite sem textura em '{}'", entity.name),
            details: "Defina uma textura válida no componente Sprite para evitar placeholder no runtime.".to_string(),
        });
        return;
    }

    let validation = validate_reference_path(project_root, texture_path, AssetType::Texture);
    if !validation.is_valid {
        warnings.push(EditorWarning {
            kind: EditorWarningKind::Sprite,
            label: format!("Textura inválida em '{}'", entity.name),
            details: validation.message,
        });
    }
}

fn collect_script_warning(project_root: &Path, entity: &Entity, script_path: &str, warnings: &mut Vec<EditorWarning>) {
    if script_path.is_empty() {
        warnings.push(EditorWarning {
            kind: EditorWarningKind::Script,
            label: format!("Script vazio em '{}'", entity.name),
            details: "Associe um arquivo .rs2 ou remova o componente Script RS2.".to_string(),
        });
        return;
    }

    if let Some(script_full_path) = script::resolve_script_path(project_root, script_path) {
        match fs::read_to_string(&script_full_path) {
            Ok(source) => {
                let script_errors = script::validate_script(&source);
                if !script_errors.is_empty() {
                    let details = script::format_script_errors(&script_errors)
                        .into_iter()
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(" | ");
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Script,
                        label: format!("Script inválido em '{}'", entity.name),
                        details,
                    });
                }
            }
            Err(_) => warnings.push(EditorWarning {
                kind: EditorWarningKind::Script,
                label: format!("Script inválido em '{}'", entity.name),
                details: format!("Falha ao ler o script: {}", script_full_path.display()),
            }),
        }
    } else if let Err(error) = script::validate_script_reference(project_root, script_path) {
        warnings.push(EditorWarning {
            kind: EditorWarningKind::Script,
            label: format!("Script inválido em '{}'", entity.name),
            details: error,
        });
    }
}

fn collect_prefab_asset_warnings(project_root: &Path, warnings: &mut Vec<EditorWarning>) {
    let prefabs_dir = project_root.join("assets/prefabs");
    let Ok(entries) = fs::read_dir(prefabs_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let is_prefab = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".prefab.json") || name.ends_with(".matr.json"))
            .unwrap_or(false);

        if !is_prefab || !path.is_file() {
            continue;
        }

        let validation = validate_asset_path(&path, AssetType::Prefab);
        if !validation.is_valid {
            warnings.push(EditorWarning {
                kind: EditorWarningKind::Prefab,
                label: format!("Prefab quebrado: {}", path.file_name().and_then(|n| n.to_str()).unwrap_or("arquivo")),
                details: validation.message,
            });
        }
    }
}

fn dedupe_warnings(warnings: &mut Vec<EditorWarning>) {
    let mut unique = Vec::new();
    for warning in warnings.drain(..) {
        let exists = unique.iter().any(|item: &EditorWarning| {
            item.kind == warning.kind && item.label == warning.label && item.details == warning.details
        });
        if !exists {
            unique.push(warning);
        }
    }
    *warnings = unique;
}
