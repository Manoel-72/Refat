use std::{collections::HashSet, fs, path::Path};

use crate::{
    core::{component::Component, entity::Entity, scene::Scene},
    runtime::{script, systems::audio_system},
};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorWarningKind {
    Scene,
    Sprite,
    Script,
    Audio,
    Prefab,
    Animation,
    Ui,
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
            Component::Velocity(_) => {}
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
            Component::LuaScript(script_component) => {
                let script_path = script_component.file_path.trim();
                if script_path.is_empty() {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Script,
                        severity: EditorWarningSeverity::Info,
                        label: format!("LuaScript vazio em '{}'", entity.name),
                        details: "Associe um arquivo .lua para preparar a integração da V0.9.".to_string(),
                    });
                } else if let Some(script_full_path) = script::resolve_script_path(project_root, script_path) {
                    match fs::read_to_string(&script_full_path) {
                        Ok(source) => {
                            let lua_warnings = script::validate_lua_source(&source);
                            if !lua_warnings.is_empty() {
                                warnings.push(EditorWarning {
                                    kind: EditorWarningKind::Script,
                                    severity: EditorWarningSeverity::Info,
                                    label: format!("LuaScript em preparação em '{}'", entity.name),
                                    details: lua_warnings.join(" | "),
                                });
                            }
                        }
                        Err(_) => warnings.push(EditorWarning {
                            kind: EditorWarningKind::Script,
                            severity: EditorWarningSeverity::Error,
                            label: format!("LuaScript inválido em '{}'", entity.name),
                            details: format!("Falha ao ler o script Lua: {}", script_full_path.display()),
                        }),
                    }
                } else if let Err(error) = script::validate_lua_script_reference(project_root, script_path) {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Script,
                        severity: EditorWarningSeverity::Error,
                        label: format!("LuaScript inválido em '{}'", entity.name),
                        details: error,
                    });
                }
            }
            Component::Animator(animator) => {
                let resolved_clip = if animator.state_mode {
                    let state_name = if animator.current_state.trim().is_empty() {
                        animator.default_state.trim()
                    } else {
                        animator.current_state.trim()
                    };
                    animator.states.get(state_name).map(|state| state.clip.as_str()).unwrap_or(animator.current.trim())
                } else {
                    animator.current.trim()
                };

                if resolved_clip.is_empty() {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Animation,
                        severity: EditorWarningSeverity::Warning,
                        label: format!("Animator sem clip atual em '{}'", entity.name),
                        details: "Defina o campo current ou um estado válido com clip associado no Animator.".to_string(),
                    });
                } else if animator.clips.get(resolved_clip).map(|clip| clip.frames.is_empty()).unwrap_or(true) {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Animation,
                        severity: EditorWarningSeverity::Warning,
                        label: format!("Animator vazio em '{}'", entity.name),
                        details: format!("O clip '{}' não possui frames configurados.", resolved_clip),
                    });
                }
            }
            Component::TextLabel(label) => {
                if label.text.trim().is_empty() {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Ui,
                        severity: EditorWarningSeverity::Info,
                        label: format!("TextLabel vazio em '{}'", entity.name),
                        details: "Defina um texto para o HUD ou diálogo aparecer no runtime.".to_string(),
                    });
                }
            }
            Component::UIButton(button) => {
                if button.text.trim().is_empty() {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Ui,
                        severity: EditorWarningSeverity::Warning,
                        label: format!("UIButton sem texto em '{}'", entity.name),
                        details: "Defina um label visível para o botão.".to_string(),
                    });
                }
                if button.width < 72.0 || button.height < 28.0 {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Ui,
                        severity: EditorWarningSeverity::Info,
                        label: format!("UIButton muito pequeno em '{}'", entity.name),
                        details: "Para clique confortável, use pelo menos 72x28 no botão.".to_string(),
                    });
                }
                if button.target_scene.trim().is_empty() && !button.close_runtime {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Ui,
                        severity: EditorWarningSeverity::Info,
                        label: format!("UIButton sem ação em '{}'", entity.name),
                        details: "O botão renderiza no runtime, mas ainda não troca cena nem fecha o runtime.".to_string(),
                    });
                } else if !button.target_scene.trim().is_empty() {
                    let raw = button.target_scene.trim().replace('\\', "/");
                    let direct = project_root.join(&raw);
                    let scenes_dir = project_root.join("assets/scenes").join(&raw);
                    let alt_scene = if raw.ends_with(".scene.json") {
                        project_root.join(&raw)
                    } else {
                        project_root.join("assets/scenes").join(format!("{}.scene.json", raw))
                    };

                    if !direct.exists() && !scenes_dir.exists() && !alt_scene.exists() {
                        warnings.push(EditorWarning {
                            kind: EditorWarningKind::Ui,
                            severity: EditorWarningSeverity::Warning,
                            label: format!("UIButton com target_scene inválido em '{}'", entity.name),
                            details: format!("A cena '{}' não foi encontrada em caminhos comuns do projeto.", raw),
                        });
                    }
                }
            }
            Component::Audio(audio_component) => {
                let audio_path = audio_component.file_path.trim();
                if audio_path.is_empty() {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Audio,
                        severity: EditorWarningSeverity::Warning,
                        label: format!("Áudio sem arquivo em '{}'", entity.name),
                        details: "Associe um .wav, .ogg ou .mp3 ao componente Audio.".to_string(),
                    });
                } else if !audio_system::validate_audio_path(project_root, audio_path) {
                    warnings.push(EditorWarning {
                        kind: EditorWarningKind::Audio,
                        severity: EditorWarningSeverity::Error,
                        label: format!("Áudio ausente em '{}'", entity.name),
                        details: format!("O arquivo '{}' não foi encontrado no projeto.", audio_path),
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
