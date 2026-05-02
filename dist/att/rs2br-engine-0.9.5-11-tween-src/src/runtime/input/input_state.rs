use std::collections::HashSet;

use super::key_code::KeyCode;

#[derive(Debug, Clone, Default)]
pub struct InputState {
    current: HashSet<KeyCode>,
    previous: HashSet<KeyCode>,
}

impl InputState {
    pub fn begin_frame(&mut self) {
        self.previous = self.current.clone();
        self.current.clear();
    }

    pub fn set_key_down(&mut self, key: KeyCode, down: bool) {
        if down {
            self.current.insert(key);
        } else {
            self.current.remove(&key);
        }
    }

    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.current.contains(&key) && !self.previous.contains(&key)
    }

    pub fn is_held(&self, key: KeyCode) -> bool {
        self.current.contains(&key)
    }

    pub fn is_released(&self, key: KeyCode) -> bool {
        !self.current.contains(&key) && self.previous.contains(&key)
    }
}
