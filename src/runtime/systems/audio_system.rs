use std::path::Path;

pub fn validate_audio_path(project_root: &Path, relative_or_full: &str) -> bool {
    if relative_or_full.trim().is_empty() {
        return false;
    }

    let normalized = relative_or_full.trim().replace('\\', "/");
    let candidates = [
        project_root.join(&normalized),
        project_root.join("assets").join(&normalized),
        project_root.join("assets/sounds").join(&normalized),
    ];

    candidates.into_iter().any(|candidate| candidate.exists())
}
