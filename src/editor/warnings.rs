use std::{collections::HashSet, fs, path::Path};

use crate::{
    core::{component::Component, entity::Entity, scene::Scene},
    runtime::script,
};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorWarningKind {
    Scene,
    Sprite,
    Script,
    Prefab,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorWarningSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct EditorWarning {
    pub kind: EditorWarningKind,
    pub severity: EditorWarningSeverity,
    pub label: String,
    pub details: String,
}

pub fn collect_scene_warnings(project_root: &Path, scene: &Scene) -> Vec<EditorWarning> {
    let mut warnings = Vec::new();

    if !scene_has_main_camera(scene) {
        warnings.push(EditorWarning {
            kind: EditorWarningKind::Scene,
            severity: EditorWarningSeverity::Warning,
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

fn dedupe_warnings(warnings: &mut Vec<EditorWarning>) {
    let mut seen = HashSet::new();
    warnings.retain(|warning| {
        seen.insert((
            warning.kind,
            warning.severity,
            warning.label.clone(),
            warning.details.clone(),
        ))
    });
}

fn scene_has_main_camera(scene: &Scene) -> bool {
    let mut found = false;
    scene.visit_entities(|entity| {
        if entity
            .components
            .iter()
            .any(|component| matches!(component, Component::Camera2D(camera) if camera.is_main))
        {
            found = true;
        }
    });
    found
}

fn collect_entity_warnings(project_root: &Path, entity: &Entity, warnings: &mut Vec<EditorWarning>) {
    for component in &entity.components {
        match component {
            Component::Sprite(sprite) => {
                let texture_path = sprite.texture_path.trim();
                if texture_path.is_empty() {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Sprite,
                        severity: EditorWarningSeverity::Warning,
                        label: format!("Sprite sem textura em '{}'", entity.name),
                        details: "Defina uma textura válida no componente Sprite para evitar placeholder no runtime.".to_string(),
                    });
                } else if resolve_project_path(project_root, texture_path).is_none() {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Sprite,
                        severity: EditorWarningSeverity::Error,
                        label: format!("Textura ausente em '{}'", entity.name),
                        details: format!("O arquivo '{}' não foi encontrado no projeto.", texture_path),
                    });
                }
            }
            Component::Script(script_component) => {
                let script_path = script_component.file_path.trim();
                if script_path.is_empty() {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Script,
                        severity: EditorWarningSeverity::Warning,
                        label: format!("Script vazio em '{}'", entity.name),
                        details: "Associe um arquivo .rs2 ou remova o componente Script RS2.".to_string(),
                    });
                } else if let Some(script_full_path) = script::resolve_script_path(project_root, script_path) {
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
                                    severity: EditorWarningSeverity::Error,
                                    label: format!("Script inválido em '{}'", entity.name),
                                    details,
                                });
                            }
                        }
                        Err(_) => warnings.push(EditorWarning {
                            kind: EditorWarningKind::Script,
                            severity: EditorWarningSeverity::Error,
                            label: format!("Script inválido em '{}'", entity.name),
                            details: format!("Falha ao ler o script: {}", script_full_path.display()),
                        }),
                    }
                } else if let Err(error) = script::validate_script_reference(project_root, script_path) {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Script,
                        severity: EditorWarningSeverity::Error,
                        label: format!("Script inválido em '{}'", entity.name),
                        details: error,
                    });
                }
            }
            _ => {}
        }
    }

    for child in &entity.children {
        collect_entity_warnings(project_root, child, warnings);
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

        let Some(content) = fs::read_to_string(&path).ok() else {
            warnings.push(EditorWarning {
                kind: EditorWarningKind::Prefab,
                severity: EditorWarningSeverity::Error,
                label: format!(
                    "Prefab inacessível: {}",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("arquivo")
                ),
                details: "Não foi possível ler o arquivo de prefab no disco.".to_string(),
            });
            continue;
        };

        let valid = crate::serialization::prefab_serializer::prefab_from_json(&content).is_some()
            || crate::core::entity::Entity::from_json(&content).is_some();
        if !valid {
            warnings.push(EditorWarning {
                kind: EditorWarningKind::Prefab,
                severity: EditorWarningSeverity::Error,
                label: format!(
                    "Prefab quebrado: {}",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("arquivo")
                ),
                details: "O JSON do prefab não pôde ser interpretado como Prefab nem como Entity legacy.".to_string(),
            });
        }
    }
}

fn resolve_project_path(project_root: &Path, relative_or_full: &str) -> Option<std::path::PathBuf> {
    let normalized = relative_or_full.trim().replace('\\', "/");
    let candidates = [
        project_root.join(&normalized),
        project_root.join("assets").join(&normalized),
    ];
    candidates.into_iter().find(|candidate| candidate.exists())
}
