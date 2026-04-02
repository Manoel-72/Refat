use std::{
    fmt,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptAction {
    ChangeScene(String),
    ReloadScene,
}

#[derive(Debug, Default, Clone)]
pub struct ScriptBehavior {
    pub move_x: f32,
    pub move_y: f32,
    pub rotate_speed: f32,
    pub player_controller_speed: f32,
    pub camera_follow: bool,
    pub start_message: Option<String>,
    pub on_update: Vec<String>,
    pub on_collision: Vec<ScriptAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    pub line: usize,
    pub message: String,
}

impl ScriptError {
    pub fn display(&self) -> String {
        format!("Linha {}: {}", self.line, self.message)
    }
}

#[derive(Debug, Clone)]
pub enum ScriptLoadError {
    EmptyPath,
    InvalidExtension(String),
    FileNotFound(String),
    ReadFailed(String),
    ValidationFailed(Vec<ScriptError>),
}

impl fmt::Display for ScriptLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptLoadError::EmptyPath => write!(f, "Caminho do script está vazio."),
            ScriptLoadError::InvalidExtension(path) => {
                write!(f, "'{path}' não é um arquivo .rs2 válido.")
            }
            ScriptLoadError::FileNotFound(path) => {
                write!(f, "Arquivo de script não encontrado: {path}")
            }
            ScriptLoadError::ReadFailed(path) => write!(f, "Falha ao ler o script: {path}"),
            ScriptLoadError::ValidationFailed(errors) => {
                let formatted = format_script_errors(errors);
                write!(f, "{}", formatted.join(" | "))
            }
        }
    }
}

pub fn is_valid_rs2_script(path: &str) -> bool {
    path.trim().to_ascii_lowercase().ends_with(".rs2")
}

pub fn load_script_behavior(project_root: &Path, raw_path: &str) -> Option<ScriptBehavior> {
    match load_script_behavior_checked(project_root, raw_path) {
        Ok(behavior) => Some(behavior),
        Err(error) => {
            eprintln!("RS2BR Script: {}", error);
            None
        }
    }
}

pub fn load_script_behavior_checked(
    project_root: &Path,
    raw_path: &str,
) -> Result<ScriptBehavior, ScriptLoadError> {
    let full_path = resolve_script_path(project_root, raw_path)
        .ok_or_else(|| resolve_script_error(project_root, raw_path))?;

    if !full_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("rs2"))
        .unwrap_or(false)
    {
        return Err(ScriptLoadError::InvalidExtension(raw_path.trim().to_string()));
    }

    let source = fs::read_to_string(&full_path)
        .map_err(|_| ScriptLoadError::ReadFailed(full_path.display().to_string()))?;

    let validation_errors = validate_script(&source);
    if !validation_errors.is_empty() {
        return Err(ScriptLoadError::ValidationFailed(validation_errors));
    }

    Ok(parse_rs2_script(&source))
}

pub fn validate_script_reference(project_root: &Path, raw_path: &str) -> Result<(), String> {
    load_script_behavior_checked(project_root, raw_path)
        .map(|_| ())
        .map_err(|e| e.to_string())
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

pub fn validate_rs2_source(source: &str) -> Vec<String> {
    format_script_errors(&validate_script(source))
}

pub fn format_script_errors(errors: &[ScriptError]) -> Vec<String> {
    errors.iter().map(ScriptError::display).collect()
}

pub fn validate_script(code: &str) -> Vec<ScriptError> {
    let mut errors = Vec::new();

    for (index, raw_line) in code.lines().enumerate() {
        let line_no = index + 1;
        let clean = normalize_script_line(raw_line);
        if clean.is_empty() {
            continue;
        }

        if clean.starts_with('@') {
            validate_directive_line(clean, line_no, &mut errors);
        } else if clean.contains('=') {
            validate_assignment_line(clean, line_no, &mut errors);
        }
    }

    errors
}

pub fn parse_rs2_script(source: &str) -> ScriptBehavior {
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
            &[
                "@player_controller",
                "player_controller",
                "player_controller_speed",
            ],
        ) {
            behavior.player_controller_speed = value.abs();
        }
        if let Some(value) = parse_script_text(line, &["@start_message", "start_message"]) {
            behavior.start_message = Some(value);
        }

        let clean = normalize_script_line(line);
        if clean == "@camera_follow" || clean == "camera_follow = true" {
            behavior.camera_follow = true;
        }
        if let Some(rest) = clean.strip_prefix("@on_update") {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                behavior.on_update.push(trimmed.to_string());
            }
        }
        if let Some(rest) = clean.strip_prefix("@on_collision") {
            if let Some(action) = parse_script_action(rest.trim()) {
                behavior.on_collision.push(action);
            }
        }
    }

    behavior
}

fn resolve_script_error(project_root: &Path, raw_path: &str) -> ScriptLoadError {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return ScriptLoadError::EmptyPath;
    }
    if !is_valid_rs2_script(trimmed) {
        return ScriptLoadError::InvalidExtension(trimmed.to_string());
    }
    let normalized = trimmed.replace('\\', "/");
    let first_candidate = project_root.join(normalized);
    ScriptLoadError::FileNotFound(first_candidate.display().to_string())
}

fn normalize_script_line(line: &str) -> &str {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        ""
    } else {
        trimmed
    }
}

fn validate_directive_line(clean: &str, line_no: usize, errors: &mut Vec<ScriptError>) {
    let token = clean.split_whitespace().next().unwrap_or(clean);
    match token {
        "@move_x" | "@move_y" | "@rotate_speed" | "@player_controller" => {
            let rest = clean.trim_start_matches(token).trim();
            if rest.is_empty() {
                errors.push(script_error(
                    line_no,
                    format!("{} precisa de um número.", token),
                ));
            } else if !is_numeric_token(rest) {
                errors.push(script_error(
                    line_no,
                    format!("{} esperava um número, mas recebeu '{}'.", token, rest),
                ));
            }
        }
        "@camera_follow" => {
            let rest = clean.trim_start_matches(token).trim();
            if !rest.is_empty() {
                errors.push(script_error(
                    line_no,
                    "@camera_follow não aceita argumentos.".to_string(),
                ));
            }
        }
        "@start_message" => {
            let rest = clean.trim_start_matches(token).trim();
            if parse_text_value(rest).is_none() {
                errors.push(script_error(
                    line_no,
                    "@start_message precisa de um texto não vazio.".to_string(),
                ));
            }
        }
        "@on_update" => {
            let rest = clean.trim_start_matches(token).trim();
            if rest.is_empty() {
                errors.push(script_error(
                    line_no,
                    "@on_update precisa de uma instrução ou descrição após a diretiva.".to_string(),
                ));
            }
        }
        "@on_collision" => {
            let rest = clean.trim_start_matches(token).trim();
            if parse_script_action(rest).is_none() {
                errors.push(script_error(
                    line_no,
                    "@on_collision precisa de uma ação válida: change_scene \"caminho\" ou reload_scene.".to_string(),
                ));
            }
        }
        _ => errors.push(script_error(
            line_no,
            format!("Diretiva desconhecida '{}'.", token),
        )),
    }
}

fn validate_assignment_line(clean: &str, line_no: usize, errors: &mut Vec<ScriptError>) {
    let mut parts = clean.splitn(2, '=');
    let key = parts.next().unwrap_or("").trim();
    let value = parts.next().unwrap_or("").trim().trim_end_matches(';').trim();

    match key {
        "move_x" | "move_y" | "rotate_speed" | "player_controller" | "player_controller_speed" => {
            if value.is_empty() {
                errors.push(script_error(
                    line_no,
                    format!("{} precisa de um número.", key),
                ));
            } else if value.parse::<f32>().is_err() {
                errors.push(script_error(
                    line_no,
                    format!("{} esperava um número, mas recebeu '{}'.", key, value),
                ));
            }
        }
        "start_message" => {
            if parse_text_value(value).is_none() {
                errors.push(script_error(
                    line_no,
                    "start_message precisa de um texto não vazio.".to_string(),
                ));
            }
        }
        "camera_follow" => {
            let lower = value.to_ascii_lowercase();
            if lower != "true" && lower != "false" {
                errors.push(script_error(
                    line_no,
                    "camera_follow só aceita true ou false.".to_string(),
                ));
            }
        }
        _ => errors.push(script_error(
            line_no,
            format!("Atribuição desconhecida '{}'.", key),
        )),
    }
}

fn script_error(line: usize, message: String) -> ScriptError {
    ScriptError { line, message }
}

fn is_numeric_token(value: &str) -> bool {
    value.trim().trim_end_matches(';').trim().parse::<f32>().is_ok()
}

fn parse_script_number(line: &str, keys: &[&str]) -> Option<f32> {
    let clean = normalize_script_line(line);
    for key in keys {
        if let Some(rest) = clean.strip_prefix(key) {
            let value = rest
                .trim()
                .trim_start_matches('=')
                .trim()
                .trim_end_matches(';')
                .trim();
            if let Ok(parsed) = value.parse::<f32>() {
                return Some(parsed);
            }
        }
    }
    None
}

fn parse_script_text(line: &str, keys: &[&str]) -> Option<String> {
    let clean = normalize_script_line(line);
    for key in keys {
        if let Some(rest) = clean.strip_prefix(key) {
            return parse_text_value(rest);
        }
    }
    None
}

fn parse_text_value(raw: &str) -> Option<String> {
    let value = raw
        .trim()
        .trim_start_matches('=')
        .trim()
        .trim_end_matches(';')
        .trim();
    let unquoted = value.trim_matches('"').trim_matches('\'');
    if unquoted.is_empty() {
        None
    } else {
        Some(unquoted.to_string())
    }
}

fn parse_script_action(raw: &str) -> Option<ScriptAction> {
    let cleaned = raw.trim().trim_end_matches(';').trim();
    if let Some(rest) = cleaned.strip_prefix("change_scene") {
        let path = rest.trim().trim_matches('"').trim_matches('\'');
        if !path.is_empty() {
            return Some(ScriptAction::ChangeScene(path.to_string()));
        }
    }
    if cleaned == "reload_scene" {
        return Some(ScriptAction::ReloadScene);
    }
    None
}
