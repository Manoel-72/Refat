use std::{
    fmt,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

pub struct ActiveAudio {
    pub key: String,
    pub sink: Sink,
    pub looped: bool,
}

impl fmt::Debug for ActiveAudio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActiveAudio")
            .field("key", &self.key)
            .field("looped", &self.looped)
            .finish()
    }
}

pub struct AudioRuntime {
    stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    active: Vec<ActiveAudio>,
}

impl AudioRuntime {
    pub fn new() -> Self {
        Self {
            stream: None,
            handle: None,
            active: Vec::new(),
        }
    }

    pub fn play_once(
        &mut self,
        key: &str,
        file_path: &Path,
        looped: bool,
        volume: f32,
    ) -> Result<(), String> {
        self.ensure_output()?;

        if self.active.iter().any(|audio| audio.key == key) {
            return Ok(());
        }

        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| "Dispositivo de áudio indisponível".to_string())?;

        let file = File::open(file_path).map_err(|error| {
            format!("Falha ao abrir áudio '{}': {}", file_path.display(), error)
        })?;
        let source = Decoder::new(BufReader::new(file)).map_err(|error| {
            format!(
                "Falha ao decodificar áudio '{}': {}",
                file_path.display(),
                error
            )
        })?;

        let sink = Sink::try_new(handle)
            .map_err(|error| format!("Falha ao criar sink de áudio: {}", error))?;

        if looped {
            sink.append(source.repeat_infinite());
        } else {
            sink.append(source);
        }
        sink.set_volume(volume.clamp(0.0, 1.5));
        sink.play();

        self.active.push(ActiveAudio {
            key: key.to_string(),
            sink,
            looped,
        });

        Ok(())
    }

    pub fn maintain(&mut self) {
        self.active
            .retain(|audio| audio.looped || !audio.sink.empty());
    }

    pub fn stop_all(&mut self) {
        for audio in &self.active {
            audio.sink.stop();
        }
        self.active.clear();
    }

    pub fn set_volume_by_name(&mut self, name: &str, volume: f32) -> usize {
        let query = name.trim().to_ascii_lowercase();
        if query.is_empty() {
            return 0;
        }

        let mut changed = 0usize;
        let volume = volume.clamp(0.0, 1.5);
        for audio in &self.active {
            if audio.key.to_ascii_lowercase().contains(&query) {
                audio.sink.set_volume(volume);
                changed += 1;
            }
        }
        changed
    }

    pub fn stop_by_name(&mut self, name: &str) -> usize {
        let query = name.trim().to_ascii_lowercase();
        if query.is_empty() {
            return 0;
        }

        let mut stopped = 0usize;
        self.active.retain(|audio| {
            let matches = audio.key.to_ascii_lowercase().contains(&query);
            if matches {
                audio.sink.stop();
                stopped += 1;
                false
            } else {
                true
            }
        });
        stopped
    }

    fn ensure_output(&mut self) -> Result<(), String> {
        if self.stream.is_some() && self.handle.is_some() {
            return Ok(());
        }

        let (stream, handle) = OutputStream::try_default()
            .map_err(|error| format!("Nenhum dispositivo de áudio disponível: {}", error))?;
        self.stream = Some(stream);
        self.handle = Some(handle);
        Ok(())
    }
}

pub fn validate_audio_path(project_root: &Path, relative_or_full: &str) -> bool {
    resolve_audio_path(project_root, relative_or_full).is_some()
}

pub fn resolve_audio_path(project_root: &Path, relative_or_full: &str) -> Option<PathBuf> {
    if relative_or_full.trim().is_empty() {
        return None;
    }

    let normalized = relative_or_full.trim().replace('\\', "/");
    let candidates = [
        project_root.join(&normalized),
        project_root.join("assets").join(&normalized),
        project_root.join("assets/sounds").join(&normalized),
        project_root.join("sounds").join(&normalized),
    ];

    candidates.into_iter().find(|candidate| candidate.exists())
}
