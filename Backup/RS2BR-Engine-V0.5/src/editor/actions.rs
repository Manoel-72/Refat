// Métodos de lógica do editor (sem UI)
// Centraliza ações de seleção, histórico e pedidos de diálogo.

use crate::editor::{DeleteTarget, EditorApp};

impl EditorApp {
    pub fn find_entity_mut(&mut self, id: &str) -> Option<&mut crate::core::entity::Entity> {
        self.scene.find_entity_mut(id)
    }

    pub fn is_entity_selected(&self, id: &str) -> bool {
        self.selected_entity_ids.iter().any(|selected| selected == id)
    }

    pub fn select_single_entity(&mut self, id: Option<String>) {
        self.selected_entity_id = id.clone();
        self.selected_entity_ids = id.into_iter().collect();
    }

    pub fn clear_entity_selection(&mut self) {
        self.selected_entity_id = None;
        self.selected_entity_ids.clear();
    }

    pub fn toggle_entity_selection(&mut self, id: String) {
        if let Some(index) = self.selected_entity_ids.iter().position(|selected| selected == &id) {
            self.selected_entity_ids.remove(index);
            if self.selected_entity_id.as_deref() == Some(&id) {
                self.selected_entity_id = self.selected_entity_ids.last().cloned();
            }
        } else {
            self.selected_entity_ids.push(id.clone());
            self.selected_entity_id = Some(id);
        }
    }

    pub fn push_undo_state(&mut self) {
        self.undo_stack.push(self.scene.clone());
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo_scene(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(self.scene.clone());
            self.scene = previous;
            self.clear_entity_selection();
            self.status_msg = "↶ Desfazer aplicado.".to_string();
        } else {
            self.status_msg = "Nada para desfazer.".to_string();
        }
    }

    pub fn redo_scene(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.scene.clone());
            self.scene = next;
            self.clear_entity_selection();
            self.status_msg = "↷ Refazer aplicado.".to_string();
        } else {
            self.status_msg = "Nada para refazer.".to_string();
        }
    }

    pub fn request_delete_selected(&mut self) {
        if !self.selected_entity_ids.is_empty() {
            let ids = self.selected_entity_ids.clone();
            let label = if ids.len() == 1 {
                let name = self.scene
                    .find_entity(&ids[0])
                    .map(|entity| entity.name.clone())
                    .unwrap_or_else(|| "Entidade".to_string());
                format!("a entidade '{}'", name)
            } else {
                format!("{} entidades selecionadas", ids.len())
            };
            self.delete_confirmation = Some(DeleteTarget::Entities { ids, label });
            return;
        }

        if let Some(id) = self.selected_entity_id.clone() {
            let name = self.scene
                .find_entity(&id)
                .map(|entity| entity.name.clone())
                .unwrap_or_else(|| "Entidade".to_string());
            self.request_delete_entity(id, name);
            return;
        }

        if let Some(path) = self.selected_asset.clone() {
            self.request_delete_asset(path);
            return;
        }

        self.status_msg = "Nada selecionado para deletar.".to_string();
    }

    pub fn request_delete_entity(&mut self, id: String, name: String) {
        self.delete_confirmation = Some(DeleteTarget::Entities {
            ids: vec![id],
            label: format!("a entidade '{}'", name),
        });
    }

    pub fn request_delete_asset(&mut self, path: std::path::PathBuf) {
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("asset")
            .to_string();
        self.delete_confirmation = Some(DeleteTarget::Asset { path, label });
    }

    pub fn request_rename_asset(&mut self, path: std::path::PathBuf) {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("asset")
            .to_string();
        self.rename_asset_dialog = Some((path, name));
    }

    pub fn request_rename_selected_entity(&mut self) {
        if let Some(id) = self.selected_entity_id.clone() {
            let name = self.scene
                .find_entity(&id)
                .map(|entity| entity.name.clone())
                .unwrap_or_else(|| "Entidade".to_string());
            self.rename_entity_dialog = Some((id, name));
        }
    }
}
