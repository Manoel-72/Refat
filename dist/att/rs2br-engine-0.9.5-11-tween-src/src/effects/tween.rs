use std::collections::HashMap;

use mlua::Function;

use crate::core::{component::Component, entity::Entity};

#[derive(Clone)]
pub struct TweenEntry {
    pub entity_id: String,
    pub props: HashMap<String, f32>,
    pub start: HashMap<String, f32>,
    pub duration: f32,
    pub elapsed: f32,
    pub easing: String,
    pub on_complete: Option<Function>,
    pub loop_tw: bool,
    pub yoyo: bool,
    pub reversed: bool,
}

impl TweenEntry {
    pub fn new(
        entity_id: impl Into<String>,
        props: HashMap<String, f32>,
        duration: f32,
        easing: impl Into<String>,
        on_complete: Option<Function>,
    ) -> Self {
        Self {
            entity_id: entity_id.into(),
            props,
            start: HashMap::new(),
            duration: duration.max(0.0001),
            elapsed: 0.0,
            easing: easing.into(),
            on_complete,
            loop_tw: false,
            yoyo: false,
            reversed: false,
        }
    }

    pub fn looping(mut self, yoyo: bool) -> Self {
        self.loop_tw = true;
        self.yoyo = yoyo;
        self
    }
}

#[derive(Clone)]
pub struct SequenceHandle {
    pub steps: Vec<SequenceStep>,
    pub index: usize,
    pub playing: bool,
}

#[derive(Clone)]
pub enum SequenceStep {
    Tween(TweenEntry),
    Wait { remaining: f32 },
}

#[derive(Clone, Default)]
pub struct TweenManager {
    pub active: Vec<TweenEntry>,
    pub sequences: HashMap<u32, SequenceHandle>,
    next_sequence_id: u32,
}

impl TweenManager {
    pub fn new() -> Self {
        Self {
            active: Vec::new(),
            sequences: HashMap::new(),
            next_sequence_id: 1,
        }
    }

    pub fn update(&mut self, entities: &mut [Entity], dt: f32) {
        let dt = dt.max(0.0);
        let mut still_active = Vec::with_capacity(self.active.len());
        for mut tween in self.active.drain(..) {
            if !Self::update_tween_entry(&mut tween, entities, dt) {
                still_active.push(tween);
            }
        }
        self.active = still_active;

        let handles: Vec<u32> = self.sequences.keys().copied().collect();
        for handle in handles {
            let Some(sequence) = self.sequences.get_mut(&handle) else {
                continue;
            };
            if !sequence.playing || sequence.index >= sequence.steps.len() {
                continue;
            }

            let mut advance = false;
            match &mut sequence.steps[sequence.index] {
                SequenceStep::Wait { remaining } => {
                    *remaining -= dt;
                    advance = *remaining <= 0.0;
                }
                SequenceStep::Tween(tween) => {
                    advance = Self::update_tween_entry(tween, entities, dt);
                }
            }

            if advance {
                sequence.index += 1;
                if sequence.index >= sequence.steps.len() {
                    sequence.playing = false;
                }
            }
        }
    }

    pub fn add_tween(&mut self, tween: TweenEntry) {
        self.active.push(tween);
    }

    pub fn new_sequence(&mut self) -> u32 {
        let handle = self.next_sequence_id.max(1);
        self.next_sequence_id = self.next_sequence_id.saturating_add(1).max(1);
        self.sequences.insert(
            handle,
            SequenceHandle {
                steps: Vec::new(),
                index: 0,
                playing: false,
            },
        );
        handle
    }

    pub fn ensure_sequence(&mut self, handle: u32) {
        self.sequences.entry(handle).or_insert_with(|| SequenceHandle {
            steps: Vec::new(),
            index: 0,
            playing: false,
        });
        if handle >= self.next_sequence_id {
            self.next_sequence_id = handle.saturating_add(1).max(1);
        }
    }

    pub fn seq_add(&mut self, handle: u32, tween: TweenEntry) {
        self.ensure_sequence(handle);
        if let Some(sequence) = self.sequences.get_mut(&handle) {
            sequence.steps.push(SequenceStep::Tween(tween));
        }
    }

    pub fn seq_wait(&mut self, handle: u32, seconds: f32) {
        self.ensure_sequence(handle);
        if let Some(sequence) = self.sequences.get_mut(&handle) {
            sequence.steps.push(SequenceStep::Wait {
                remaining: seconds.max(0.0),
            });
        }
    }

    pub fn seq_play(&mut self, handle: u32) {
        if let Some(sequence) = self.sequences.get_mut(&handle) {
            sequence.index = 0;
            sequence.playing = !sequence.steps.is_empty();
            for step in &mut sequence.steps {
                match step {
                    SequenceStep::Tween(tween) => {
                        tween.elapsed = 0.0;
                        tween.start.clear();
                        tween.reversed = false;
                    }
                    SequenceStep::Wait { remaining } => {
                        *remaining = remaining.max(0.0);
                    }
                }
            }
        }
    }

    pub fn seq_stop(&mut self, handle: u32) {
        if let Some(sequence) = self.sequences.get_mut(&handle) {
            sequence.playing = false;
        }
    }

    fn update_tween_entry(tween: &mut TweenEntry, entities: &mut [Entity], dt: f32) -> bool {
        let Some(entity) = find_entity_mut(entities, &tween.entity_id) else {
            return true;
        };

        if tween.start.is_empty() {
            for prop in tween.props.keys() {
                if let Some(value) = read_property(entity, prop) {
                    tween.start.insert(prop.clone(), value);
                }
            }
        }

        tween.elapsed = (tween.elapsed + dt).min(tween.duration);
        let raw_t = (tween.elapsed / tween.duration).clamp(0.0, 1.0);
        let eased = ease(raw_t, &tween.easing);
        let factor = if tween.reversed { 1.0 - eased } else { eased };

        for (prop, end_value) in &tween.props {
            let start_value = tween.start.get(prop).copied().unwrap_or_else(|| {
                read_property(entity, prop).unwrap_or(*end_value)
            });
            let value = start_value + (*end_value - start_value) * factor;
            write_property(entity, prop, value);
        }

        if tween.elapsed < tween.duration {
            return false;
        }

        if tween.loop_tw {
            if tween.yoyo {
                tween.reversed = !tween.reversed;
            }
            tween.elapsed = 0.0;
            return false;
        }

        if let Some(callback) = tween.on_complete.take() {
            if let Err(err) = callback.call::<()>(()) {
                eprintln!("[Tween][callback] {}", err);
            }
        }
        true
    }
}

fn ease(t: f32, easing: &str) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match easing.trim().to_ascii_lowercase().as_str() {
        "in" | "ease_in" | "ease-in" => t * t,
        "out" | "ease_out" | "ease-out" => 1.0 - (1.0 - t) * (1.0 - t),
        "inout" | "in_out" | "ease_in_out" | "ease-in-out" => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - ((-2.0 * t + 2.0).powi(2) / 2.0)
            }
        }
        _ => t,
    }
}

fn find_entity_mut<'a>(entities: &'a mut [Entity], id: &str) -> Option<&'a mut Entity> {
    for entity in entities {
        if entity.id == id {
            return Some(entity);
        }
        if let Some(found) = entity.find_mut(id) {
            return Some(found);
        }
    }
    None
}

fn read_property(entity: &Entity, prop: &str) -> Option<f32> {
    let key = normalize_prop(prop);
    match key.as_str() {
        "x" => entity.transform().map(|t| t.x),
        "y" => entity.transform().map(|t| t.y),
        "scale_x" => entity.transform().map(|t| t.scale_x),
        "scale_y" => entity.transform().map(|t| t.scale_y),
        "rotation" => entity.transform().map(|t| t.rotation),
        "alpha" => entity.components.iter().find_map(|component| match component {
            Component::Sprite(sprite) => Some(sprite.color_a),
            Component::TextLabel(label) => Some(label.color_a),
            Component::UIButton(button) => Some(button.color_a),
            _ => None,
        }),
        _ => None,
    }
}

fn write_property(entity: &mut Entity, prop: &str, value: f32) -> bool {
    let key = normalize_prop(prop);
    match key.as_str() {
        "x" => entity.transform_mut().map(|t| t.x = value).is_some(),
        "y" => entity.transform_mut().map(|t| t.y = value).is_some(),
        "scale_x" => entity.transform_mut().map(|t| t.scale_x = value).is_some(),
        "scale_y" => entity.transform_mut().map(|t| t.scale_y = value).is_some(),
        "rotation" => entity.transform_mut().map(|t| t.rotation = value).is_some(),
        "alpha" => {
            let alpha = value.clamp(0.0, 1.0);
            let mut changed = false;
            for component in &mut entity.components {
                match component {
                    Component::Sprite(sprite) => {
                        sprite.color_a = alpha;
                        changed = true;
                    }
                    Component::TextLabel(label) => {
                        label.color_a = alpha;
                        changed = true;
                    }
                    Component::UIButton(button) => {
                        button.color_a = alpha;
                        changed = true;
                    }
                    _ => {}
                }
            }
            changed
        }
        _ => false,
    }
}

fn normalize_prop(prop: &str) -> String {
    match prop.trim().to_ascii_lowercase().as_str() {
        "scale" => "scale_x".to_string(),
        "sx" => "scale_x".to_string(),
        "sy" => "scale_y".to_string(),
        "rot" => "rotation".to_string(),
        "a" => "alpha".to_string(),
        other => other.to_string(),
    }
}
