// Buffer management system - Emacs-style buffers for different content types
//
// This module implements the core buffer system that mimics Emacs buffers,
// where each buffer represents different content (podcast list, episode list, etc.)

pub mod episode_list;
pub mod help;
pub mod podcast_list;

use ratatui::layout::Rect;
use std::collections::HashMap;

use crate::ui::{UIAction, UIComponent, UIError, UIResult};
use crate::{podcast::subscription::SubscriptionManager, storage::JsonStorage};
use std::sync::Arc;

/// Unique identifier for buffers
pub type BufferId = String;

/// Trait that all buffer types must implement
pub trait Buffer: UIComponent {
    /// Get the unique ID of this buffer
    fn id(&self) -> BufferId;

    /// Get the buffer name for display in buffer lists
    fn name(&self) -> String;

    /// Check if this buffer can be closed
    fn can_close(&self) -> bool {
        true
    }

    /// Called when the buffer is activated (receives focus)
    fn on_activate(&mut self) {}

    /// Called when the buffer is deactivated (loses focus)  
    fn on_deactivate(&mut self) {}

    /// Get help text for this buffer's keybindings
    fn help_text(&self) -> Vec<String> {
        vec![
            "Buffer-specific help not available".to_string(),
            "Press C-h for general help".to_string(),
        ]
    }
}

/// Buffer manager that handles multiple buffers and switching between them
pub struct BufferManager {
    buffers: HashMap<BufferId, Box<dyn Buffer>>,
    active_buffer: Option<BufferId>,
    buffer_order: Vec<BufferId>,
}

impl BufferManager {
    /// Create a new buffer manager
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            active_buffer: None,
            buffer_order: Vec::new(),
        }
    }

    /// Add a buffer to the manager
    pub fn add_buffer(&mut self, buffer: Box<dyn Buffer>) -> UIResult<()> {
        let id = buffer.id();
        let name = buffer.name();

        if self.buffers.contains_key(&id) {
            return Err(UIError::InvalidOperation(format!(
                "Buffer '{}' already exists",
                name
            )));
        }

        self.buffer_order.push(id.clone());
        self.buffers.insert(id.clone(), buffer);

        // If this is the first buffer, make it active
        if self.active_buffer.is_none() {
            self.active_buffer = Some(id);
        }

        Ok(())
    }

    /// Remove a buffer from the manager
    pub fn remove_buffer(&mut self, buffer_id: &BufferId) -> UIResult<()> {
        let buffer = self
            .buffers
            .get(buffer_id)
            .ok_or_else(|| UIError::BufferNotFound(buffer_id.clone()))?;

        if !buffer.can_close() {
            return Err(UIError::InvalidOperation(format!(
                "Buffer '{}' cannot be closed",
                buffer.name()
            )));
        }

        self.buffers.remove(buffer_id);
        self.buffer_order.retain(|id| id != buffer_id);

        // If we removed the active buffer, switch to another one
        if self.active_buffer.as_ref() == Some(buffer_id) {
            self.active_buffer = self.buffer_order.first().cloned();
        }

        Ok(())
    }

    /// Switch to a buffer by ID
    pub fn switch_to_buffer(&mut self, buffer_id: &BufferId) -> UIResult<()> {
        if !self.buffers.contains_key(buffer_id) {
            return Err(UIError::BufferNotFound(buffer_id.clone()));
        }

        // Deactivate current buffer
        if let Some(current_id) = &self.active_buffer {
            if let Some(current_buffer) = self.buffers.get_mut(current_id) {
                current_buffer.on_deactivate();
                current_buffer.set_focus(false);
            }
        }

        // Activate new buffer
        self.active_buffer = Some(buffer_id.clone());
        if let Some(new_buffer) = self.buffers.get_mut(buffer_id) {
            new_buffer.on_activate();
            new_buffer.set_focus(true);
        }

        Ok(())
    }

    /// Switch to the next buffer in the order
    pub fn next_buffer(&mut self) -> UIResult<()> {
        if self.buffer_order.is_empty() {
            return Err(UIError::InvalidOperation(
                "No buffers available".to_string(),
            ));
        }

        let current_index = self
            .active_buffer
            .as_ref()
            .and_then(|id| self.buffer_order.iter().position(|bid| bid == id))
            .unwrap_or(0);

        let next_index = (current_index + 1) % self.buffer_order.len();
        let next_id = self.buffer_order[next_index].clone();

        self.switch_to_buffer(&next_id)
    }

    /// Switch to the previous buffer in the order
    pub fn previous_buffer(&mut self) -> UIResult<()> {
        if self.buffer_order.is_empty() {
            return Err(UIError::InvalidOperation(
                "No buffers available".to_string(),
            ));
        }

        let current_index = self
            .active_buffer
            .as_ref()
            .and_then(|id| self.buffer_order.iter().position(|bid| bid == id))
            .unwrap_or(0);

        let prev_index = if current_index == 0 {
            self.buffer_order.len() - 1
        } else {
            current_index - 1
        };
        let prev_id = self.buffer_order[prev_index].clone();

        self.switch_to_buffer(&prev_id)
    }

    /// Get the currently active buffer
    pub fn active_buffer(&mut self) -> Option<&mut Box<dyn Buffer>> {
        self.active_buffer
            .as_ref()
            .and_then(|id| self.buffers.get_mut(id))
    }

    /// Get the active buffer ID
    pub fn active_buffer_id(&self) -> Option<&BufferId> {
        self.active_buffer.as_ref()
    }

    /// Get a buffer by ID
    pub fn get_buffer(&mut self, buffer_id: &BufferId) -> Option<&mut Box<dyn Buffer>> {
        self.buffers.get_mut(buffer_id)
    }

    /// Get all buffer names for switching UI
    pub fn buffer_names(&self) -> Vec<(BufferId, String)> {
        self.buffer_order
            .iter()
            .filter_map(|id| {
                self.buffers
                    .get(id)
                    .map(|buffer| (id.clone(), buffer.name()))
            })
            .collect()
    }

    /// Get all buffer IDs
    pub fn get_buffer_ids(&self) -> Vec<BufferId> {
        self.buffer_order.clone()
    }

    /// Get current buffer name (alias for active buffer name)
    pub fn current_buffer_name(&self) -> Option<String> {
        self.active_buffer
            .as_ref()
            .and_then(|id| self.buffers.get(id).map(|buffer| buffer.name()))
    }

    /// Get current buffer (alias for active buffer)
    pub fn current_buffer_mut(&mut self) -> Option<&mut Box<dyn Buffer>> {
        self.active_buffer()
    }

    /// Switch to buffer by name (convenience method)
    pub fn switch_to_buffer_by_name(&mut self, buffer_name: String) {
        let _ = self.switch_to_buffer(&buffer_name);
    }

    /// Create help buffer
    pub fn create_help_buffer(&mut self) {
        let help_buffer = Box::new(crate::ui::buffers::help::HelpBuffer::new());
        let _ = self.add_buffer(help_buffer);
    }

    /// Create podcast list buffer
    pub fn create_podcast_list_buffer(
        &mut self,
        subscription_manager: Arc<SubscriptionManager<JsonStorage>>,
    ) {
        let mut podcast_buffer = crate::ui::buffers::podcast_list::PodcastListBuffer::new();
        podcast_buffer.set_subscription_manager(subscription_manager);
        let _ = self.add_buffer(Box::new(podcast_buffer));
    }

    /// Get mutable reference to podcast list buffer
    pub fn get_podcast_list_buffer_mut(
        &mut self,
    ) -> Option<&mut crate::ui::buffers::podcast_list::PodcastListBuffer> {
        let podcast_id = "podcast-list".to_string();
        self.get_buffer(&podcast_id).and_then(|buffer| {
            // This is safe because we know podcast-list buffer is always PodcastListBuffer
            let raw_ptr = buffer.as_mut() as *mut dyn Buffer;
            unsafe {
                (raw_ptr as *mut crate::ui::buffers::podcast_list::PodcastListBuffer).as_mut()
            }
        })
    }

    /// Handle a UI action, dispatching to the active buffer if appropriate
    pub fn handle_action(&mut self, action: UIAction) -> UIAction {
        match action {
            UIAction::NextBuffer => match self.next_buffer() {
                Ok(_) => UIAction::Render,
                Err(_) => UIAction::None,
            },
            UIAction::PreviousBuffer => match self.previous_buffer() {
                Ok(_) => UIAction::Render,
                Err(_) => UIAction::None,
            },
            UIAction::SwitchBuffer(buffer_name) => {
                // Find buffer by name
                let buffer_id = self
                    .buffer_names()
                    .into_iter()
                    .find(|(_, name)| name == &buffer_name)
                    .map(|(id, _)| id);

                if let Some(id) = buffer_id {
                    match self.switch_to_buffer(&id) {
                        Ok(_) => UIAction::Render,
                        Err(_) => UIAction::None,
                    }
                } else {
                    UIAction::None
                }
            }
            UIAction::CloseBuffer(buffer_name) => {
                // Find buffer by name
                let buffer_id = self
                    .buffer_names()
                    .into_iter()
                    .find(|(_, name)| name == &buffer_name)
                    .map(|(id, _)| id);

                if let Some(id) = buffer_id {
                    match self.remove_buffer(&id) {
                        Ok(_) => UIAction::Render,
                        Err(_) => UIAction::None,
                    }
                } else {
                    UIAction::None
                }
            }
            _ => {
                // Pass other actions to the active buffer
                if let Some(buffer) = self.active_buffer() {
                    buffer.handle_action(action)
                } else {
                    UIAction::None
                }
            }
        }
    }

    /// Render the active buffer
    pub fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        if let Some(buffer) = self.active_buffer() {
            buffer.render(frame, area);
        }
    }

    /// Get the title of the active buffer
    pub fn active_title(&self) -> String {
        self.active_buffer
            .as_ref()
            .and_then(|id| self.buffers.get(id))
            .map(|buffer| buffer.title())
            .unwrap_or_else(|| "No Buffer".to_string())
    }

    /// Check if there are any buffers
    pub fn has_buffers(&self) -> bool {
        !self.buffers.is_empty()
    }

    /// Get help text from the active buffer
    pub fn active_help_text(&self) -> Vec<String> {
        self.active_buffer
            .as_ref()
            .and_then(|id| self.buffers.get(id))
            .map(|buffer| buffer.help_text())
            .unwrap_or_else(|| vec!["No active buffer".to_string()])
    }
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::buffers::help::HelpBuffer;

    #[test]
    fn test_buffer_manager_creation() {
        let manager = BufferManager::new();
        assert!(!manager.has_buffers());
        assert!(manager.active_buffer_id().is_none());
    }

    #[test]
    fn test_add_and_switch_buffers() {
        let mut manager = BufferManager::new();

        // Add help buffer
        let help_buffer = HelpBuffer::new();
        let help_id = help_buffer.id();
        manager.add_buffer(Box::new(help_buffer)).unwrap();

        assert!(manager.has_buffers());
        assert_eq!(manager.active_buffer_id(), Some(&help_id));

        // Test buffer switching
        let names = manager.buffer_names();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].0, help_id);
    }

    #[test]
    fn test_next_previous_buffer() {
        let mut manager = BufferManager::new();

        // Add multiple buffers
        let help1 = HelpBuffer::new();
        let help2 = HelpBuffer::with_content("test".to_string(), vec!["Test".to_string()]);

        manager.add_buffer(Box::new(help1)).unwrap();
        manager.add_buffer(Box::new(help2)).unwrap();

        let initial_id = manager.active_buffer_id().cloned();

        // Test next buffer
        manager.next_buffer().unwrap();
        let next_id = manager.active_buffer_id();
        assert_ne!(initial_id.as_ref(), next_id);

        // Test previous buffer (should go back)
        manager.previous_buffer().unwrap();
        assert_eq!(manager.active_buffer_id(), initial_id.as_ref());
    }
}
