use std::{
    fmt,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use rodio::{
    source::{ChannelVolume, Source},
    Decoder, OutputStream, OutputStreamHandle, Sink,
};

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

#[derive(Clone)]
struct Crossfade {
    old_key: String,
    new_key: String,
    old_start_volume: f32,
    elapsed: f32,
    duration: f32,
}

pub struct AudioRuntime {
    stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    active: Vec<ActiveAudio>,
    sfx_volume: f32,
    music_volume: f32,
    anon_seq: u64,
    /// Chave do `ActiveAudio` que é a música principal (após crossfade ou `play` looped).
    current_music_key: Option<String>,
    crossfade: Option<Crossfade>,
}

impl AudioRuntime {
    pub fn new() -> Self {
        Self {
            stream: None,
            handle: None,
            active: Vec::new(),
            sfx_volume: 1.0,
            music_volume: 1.0,
            anon_seq: 0,
            current_music_key: None,
            crossfade: None,
        }
    }

    pub fn alloc_play_key(&mut self) -> String {
        self.anon_seq = self.anon_seq.wrapping_add(1);
        format!("__rs2_audio_{}", self.anon_seq)
    }

    fn remove_active_by_key(&mut self, key: &str) {
        self.active.retain(|a| {
            if a.key == key {
                a.sink.stop();
                false
            } else {
                true
            }
        });
    }

    fn stop_current_music_track(&mut self) {
        if let Some(k) = self.current_music_key.take() {
            self.remove_active_by_key(&k);
        }
    }

    fn abort_crossfade(&mut self) {
        if let Some(cf) = self.crossfade.take() {
            self.remove_active_by_key(&cf.old_key);
            self.remove_active_by_key(&cf.new_key);
        }
    }

    fn sink_by_key(&self, key: &str) -> Option<&Sink> {
        self.active.iter().find(|a| a.key == key).map(|a| &a.sink)
    }

    fn sink_by_key_mut(&mut self, key: &str) -> Option<&mut Sink> {
        self.active.iter_mut().find(|a| a.key == key).map(|a| &mut a.sink)
    }

    /// Primeiro sink = `key_first`, segundo = `key_second` (ordem independente do índice em `active`).
    fn two_ordered_sinks_mut(
        &mut self,
        key_first: &str,
        key_second: &str,
    ) -> Option<(&mut Sink, &mut Sink)> {
        let i_first = self.active.iter().position(|a| a.key == key_first)?;
        let i_second = self.active.iter().position(|a| a.key == key_second)?;
        if i_first == i_second {
            return None;
        }
        let (lo, hi) = if i_first < i_second {
            (i_first, i_second)
        } else {
            (i_second, i_first)
        };
        let (left, right) = self.active.split_at_mut(hi);
        if i_first < i_second {
            Some((&mut left[lo].sink, &mut right[0].sink))
        } else {
            Some((&mut right[0].sink, &mut left[lo].sink))
        }
    }

    pub fn sfx_volume(&self) -> f32 {
        self.sfx_volume
    }

    pub fn music_volume(&self) -> f32 {
        self.music_volume
    }

    pub fn set_sfx_volume(&mut self, v: f32) {
        self.sfx_volume = v.clamp(0.0, 2.0);
    }

    pub fn set_music_volume(&mut self, v: f32) {
        let prev_mv = self.music_volume;
        self.music_volume = v.clamp(0.0, 2.0);
        let ratio = self.music_volume / prev_mv.max(1e-6);
        let cur_key = self.current_music_key.clone();
        if let Some(ref k) = cur_key {
            if let Some(s) = self.sink_by_key_mut(k) {
                s.set_volume((s.volume() * ratio).clamp(0.0, 1.5));
            }
        }
        let mv = self.music_volume;
        let cf_snap = self.crossfade.clone();
        if let Some(cf) = cf_snap {
            let t = (cf.elapsed / cf.duration.max(1e-6)).clamp(0.0, 1.0);
            if let Some((s_old, s_new)) = self.two_ordered_sinks_mut(&cf.old_key, &cf.new_key) {
                s_old.set_volume((cf.old_start_volume * (1.0 - t)).clamp(0.0, 1.5));
                s_new.set_volume((mv * t).clamp(0.0, 1.5));
            }
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

        let group = if looped {
            self.music_volume
        } else {
            self.sfx_volume
        };
        let eff = (volume * group).clamp(0.0, 1.5);

        if looped {
            self.abort_crossfade();
            self.stop_current_music_track();

            sink.append(source.repeat_infinite());
            sink.set_volume(eff);
            sink.play();

            let key_owned = key.to_string();
            self.current_music_key = Some(key_owned.clone());
            self.active.push(ActiveAudio {
                key: key_owned,
                sink,
                looped: true,
            });
            return Ok(());
        }

        sink.append(source);
        sink.set_volume(eff);
        sink.play();

        self.active.push(ActiveAudio {
            key: key.to_string(),
            sink,
            looped,
        });

        Ok(())
    }

    pub fn play_at(
        &mut self,
        key: &str,
        file_path: &Path,
        world_x: f32,
        world_y: f32,
        cam_x: f32,
        cam_y: f32,
        max_dist: f32,
        base_vol: f32,
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
        let decoder = Decoder::new(BufReader::new(file)).map_err(|error| {
            format!(
                "Falha ao decodificar áudio '{}': {}",
                file_path.display(),
                error
            )
        })?;

        let dx = world_x - cam_x;
        let dy = world_y - cam_y;
        let dist = (dx * dx + dy * dy).sqrt();
        let max_d = max_dist.max(1e-6);
        let dist_vol = (1.0 - dist / max_d).max(0.0);
        let pan = (dx / max_d).clamp(-1.0, 1.0);
        let linear = (dist_vol * base_vol * self.sfx_volume).clamp(0.0, 1.5);
        let left = ((1.0 - pan) * 0.5 * linear).max(0.0);
        let right = ((1.0 + pan) * 0.5 * linear).max(0.0);

        let sink = Sink::try_new(handle)
            .map_err(|error| format!("Falha ao criar sink de áudio: {}", error))?;

        let mono = decoder.convert_samples::<f32>();
        let panned = ChannelVolume::new(mono, vec![left, right]);
        sink.append(panned);
        sink.set_volume(1.0);
        sink.play();

        self.active.push(ActiveAudio {
            key: key.to_string(),
            sink,
            looped: false,
        });

        Ok(())
    }

    pub fn crossfade_to(
        &mut self,
        key: &str,
        file_path: &Path,
        duration_secs: f32,
    ) -> Result<(), String> {
        self.ensure_output()?;
        self.abort_crossfade();

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

        let new_sink = Sink::try_new(handle)
            .map_err(|error| format!("Falha ao criar sink de áudio: {}", error))?;
        new_sink.append(source.repeat_infinite());
        new_sink.set_volume(0.0);
        new_sink.play();

        let key_owned = key.to_string();
        let duration = duration_secs.max(0.0);

        if duration <= 1e-6 {
            self.stop_current_music_track();
            new_sink.set_volume(self.music_volume.clamp(0.0, 1.5));
            self.current_music_key = Some(key_owned.clone());
            self.active.push(ActiveAudio {
                key: key_owned,
                sink: new_sink,
                looped: true,
            });
            return Ok(());
        }

        if let Some(old_key) = self.current_music_key.take() {
            let old_start_volume = self
                .sink_by_key(&old_key)
                .map(|s| s.volume())
                .unwrap_or(self.music_volume);
            self.crossfade = Some(Crossfade {
                old_key: old_key.clone(),
                new_key: key_owned.clone(),
                old_start_volume,
                elapsed: 0.0,
                duration,
            });
            self.active.push(ActiveAudio {
                key: key_owned.clone(),
                sink: new_sink,
                looped: true,
            });
        } else {
            new_sink.set_volume(self.music_volume.clamp(0.0, 1.5));
            self.current_music_key = Some(key_owned.clone());
            self.active.push(ActiveAudio {
                key: key_owned,
                sink: new_sink,
                looped: true,
            });
        }

        Ok(())
    }

    pub fn play_music_looped(
        &mut self,
        intro_key: &str,
        intro_path: &Path,
        loop_key: &str,
        loop_path: &Path,
    ) -> Result<(), String> {
        self.ensure_output()?;
        self.abort_crossfade();
        self.stop_current_music_track();

        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| "Dispositivo de áudio indisponível".to_string())?;

        let intro_file = File::open(intro_path).map_err(|error| {
            format!(
                "Falha ao abrir áudio intro '{}': {}",
                intro_path.display(),
                error
            )
        })?;
        let intro_dec = Decoder::new(BufReader::new(intro_file)).map_err(|error| {
            format!(
                "Falha ao decodificar intro '{}': {}",
                intro_path.display(),
                error
            )
        })?;

        let loop_file = File::open(loop_path).map_err(|error| {
            format!(
                "Falha ao abrir áudio loop '{}': {}",
                loop_path.display(),
                error
            )
        })?;
        let loop_dec = Decoder::new(BufReader::new(loop_file)).map_err(|error| {
            format!(
                "Falha ao decodificar loop '{}': {}",
                loop_path.display(),
                error
            )
        })?;

        let sink = Sink::try_new(handle)
            .map_err(|error| format!("Falha ao criar sink de áudio: {}", error))?;
        sink.append(intro_dec);
        sink.append(loop_dec.repeat_infinite());
        sink.set_volume(self.music_volume.clamp(0.0, 1.5));
        sink.play();

        let composite_key = format!("{}|{}", intro_key, loop_key);
        self.current_music_key = Some(composite_key.clone());
        self.active.push(ActiveAudio {
            key: composite_key,
            sink,
            looped: true,
        });

        Ok(())
    }

    pub fn pause_music(&mut self) {
        let cur = self.current_music_key.clone();
        if let Some(ref k) = cur {
            if let Some(s) = self.sink_by_key_mut(k) {
                s.pause();
            }
        }
        let cf = self.crossfade.clone();
        if let Some(ref c) = cf {
            if let Some(s) = self.sink_by_key_mut(&c.old_key) {
                s.pause();
            }
            if let Some(s) = self.sink_by_key_mut(&c.new_key) {
                s.pause();
            }
        }
    }

    pub fn resume_music(&mut self) {
        let cur = self.current_music_key.clone();
        if let Some(ref k) = cur {
            if let Some(s) = self.sink_by_key_mut(k) {
                s.play();
            }
        }
        let cf = self.crossfade.clone();
        if let Some(ref c) = cf {
            if let Some(s) = self.sink_by_key_mut(&c.old_key) {
                s.play();
            }
            if let Some(s) = self.sink_by_key_mut(&c.new_key) {
                s.play();
            }
        }
    }

    pub fn music_playing(&self) -> bool {
        let sink_ok = |s: &Sink| !s.empty() && !s.is_paused();
        if let Some(ref k) = self.current_music_key {
            if let Some(s) = self.sink_by_key(k) {
                if sink_ok(s) {
                    return true;
                }
            }
        }
        if let Some(ref cf) = self.crossfade {
            if let Some(s) = self.sink_by_key(&cf.old_key) {
                if sink_ok(s) {
                    return true;
                }
            }
            if let Some(s) = self.sink_by_key(&cf.new_key) {
                if sink_ok(s) {
                    return true;
                }
            }
        }
        false
    }

    pub fn maintain(&mut self, dt: f32) {
        if let Some(mut cf) = self.crossfade.take() {
            cf.elapsed += dt;
            let t = (cf.elapsed / cf.duration.max(1e-6)).min(1.0);
            let mv = self.music_volume;
            if let Some((s_old, s_new)) = self.two_ordered_sinks_mut(&cf.old_key, &cf.new_key) {
                s_old.set_volume((cf.old_start_volume * (1.0 - t)).clamp(0.0, 1.5));
                s_new.set_volume((mv * t).clamp(0.0, 1.5));
            }
            if t >= 1.0 - 1e-5 {
                self.remove_active_by_key(&cf.old_key);
                let mv_end = self.music_volume;
                if let Some(s) = self.sink_by_key_mut(&cf.new_key) {
                    s.set_volume(mv_end.clamp(0.0, 1.5));
                }
                self.current_music_key = Some(cf.new_key.clone());
                self.crossfade = None;
            } else {
                self.crossfade = Some(cf);
            }
        }

        self.active.retain(|audio| audio.looped || !audio.sink.empty());

        if let Some(ref k) = self.current_music_key {
            if self.sink_by_key(k).map(|s| s.empty()).unwrap_or(true) {
                self.current_music_key = None;
            }
        }
    }

    pub fn stop_all(&mut self) {
        self.abort_crossfade();
        for audio in &self.active {
            audio.sink.stop();
        }
        self.active.clear();
        self.current_music_key = None;
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
                let group = if audio.looped {
                    self.music_volume
                } else {
                    self.sfx_volume
                };
                audio.sink.set_volume((volume * group).clamp(0.0, 1.5));
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

        if let Some(ref k) = self.current_music_key {
            if !self.active.iter().any(|a| &a.key == k) {
                self.current_music_key = None;
            }
        }

        if let Some(cf) = self.crossfade.clone() {
            let old_exists = self.active.iter().any(|a| a.key == cf.old_key);
            let new_exists = self.active.iter().any(|a| a.key == cf.new_key);
            if !old_exists || !new_exists {
                self.crossfade = None;
                if new_exists {
                    self.current_music_key = Some(cf.new_key);
                } else if old_exists {
                    self.current_music_key = Some(cf.old_key);
                } else {
                    self.current_music_key = None;
                }
            }
        }

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
