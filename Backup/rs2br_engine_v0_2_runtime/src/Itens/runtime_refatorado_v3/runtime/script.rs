use std::{fs, path::{Path, PathBuf}};

#[derive(Debug, Default, Clone)]
pub struct ScriptBehavior {
    pub move_x: f32,
    pub move_y: f32,
    pub rotate_speed: f32,
    pub player_controller_speed: f32,
    pub camera_follow: bool,
    pub start_message: Option<String>,
    pub on_update: Vec<String>,
}

pub fn load_script_behavior(project_root: &Path, raw_path: &str) -> Option<ScriptBehavior> {
    let full_path = resolve_script_path(project_root, raw_path)?;
    let source = fs::read_to_string(full_path).ok()?;
    Some(parse_script_behavior(&source))
}

pub fn resolve_script_path(project_root: &Path, raw_path: &str) -> Option<PathBuf> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.replace('\\', "/");
    let candidates = [
        project_root.join(&normalized),
        project_root.join("assets").join(&normalized),
        project_root.join("assets/scripts").join(&normalized),
    ];

    candidates.into_iter().find(|path| path.exists())
}

pub fn parse_script_behavior(source: &str) -> ScriptBehavior {
    let mut behavior = ScriptBehavior::default();

    for line in source.lines() {
        if let Some(value) = parse_script_number(line, &["@move_x", "move_x"]) {
            behavior.move_x = value;
        }
        if let Some(value) = parse_script_number(line, &["@move_y", "move_y"]) {
            behavior.move_y = value;
        }
        if let Some(value) = parse_script_number(line, &["@rotate_speed", "rotate_speed"]) {
            behavior.rotate_speed = value;
        }
        if let Some(value) = parse_script_number(
            line,
            &["@player_controller", "player_controller", "player_controller_speed"],
        ) {
            behavior.player_controller_speed = value.abs();
        }
        if let Some(value) = parse_script_text(line, &["@start_message", "start_message"]) {
            behavior.start_message = Some(value);
        }

        let clean = line.trim().trim_start_matches('/').trim();
        if clean.contains("@camera_follow") || clean.contains("camera_follow = true") {
            behavior.camera_follow = true;
        }
        if let Some(rest) = clean.strip_prefix("@on_update") {
            behavior.on_update.push(rest.trim().to_string());
        }
    }

    behavior
}

fn parse_script_number(line: &str, names: &[&str]) -> Option<f32> {
    let clean = line.trim().trim_start_matches('/').trim().trim_end_matches(';');
    for name in names {
        if let Some(rest) = clean.strip_prefix(name) {
            let value = rest.trim_start_matches(':').trim_start_matches('=').trim();
            if let Ok(parsed) = value.parse::<f32>() {
                return Some(parsed);
            }
        }
    }
    None
}

fn parse_script_text(line: &str, names: &[&str]) -> Option<String> {
    let clean = line.trim().trim_start_matches('/').trim().trim_end_matches(';');
    for name in names {
        if let Some(rest) = clean.strip_prefix(name) {
            let value = rest
                .trim_start_matches(':')
                .trim_start_matches('=')
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}
