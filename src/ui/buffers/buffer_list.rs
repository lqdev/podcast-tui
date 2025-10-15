// Buffer list display - Emacs-style buffer list
//
// This buffer shows all open buffers in a list format, allowing
// users to navigate and manage buffers like in Emacs.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::ui::{
    buffers::{Buffer, BufferId},
    themes::Theme,
    UIAction, UIComponent,
};

/// Buffer that displays a list of all open buffers
pub struct BufferListBuffer {
    /// Buffer items with (id, name, current) tuples
    buffer_items: Vec<(BufferId, String, bool)>,
    /// Current selection
    list_state: ListState,
    /// Theme for styling
    theme: Theme,
    /// Whether this buffer has focus
    focused: bool,
}

impl BufferListBuffer {
    /// Create a new buffer list buffer
    pub fn new() -> Self {
        let mut buffer = Self {
            buffer_items: Vec::new(),
            list_state: ListState::default(),
            theme: Theme::default(),
            focused: false,
        };
        buffer.list_state.select(Some(0));
        buffer
    }

    /// Update the buffer list with current buffers
    pub fn update_buffer_list(
        &mut self,
        buffers: Vec<(BufferId, String)>,
        current_buffer: Option<&BufferId>,
    ) {
        self.buffer_items = buffers
            .into_iter()
            .map(|(id, name)| {
                let is_current = current_buffer.map_or(false, |current| current == &id);
                (id, name, is_current)
            })
            .collect();

        // Ensure selection is valid
        if self.buffer_items.is_empty() {
            self.list_state.select(None);
        } else if self.list_state.selected().unwrap_or(0) >= self.buffer_items.len() {
            self.list_state.select(Some(0));
        }
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.buffer_items.is_empty() {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let new_index = if selected == 0 {
            self.buffer_items.len() - 1
        } else {
            selected - 1
        };
        self.list_state.select(Some(new_index));
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.buffer_items.is_empty() {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let new_index = (selected + 1) % self.buffer_items.len();
        self.list_state.select(Some(new_index));
    }

    /// Get the currently selected buffer ID
    pub fn selected_buffer_id(&self) -> Option<&BufferId> {
        self.list_state
            .selected()
            .and_then(|index| self.buffer_items.get(index))
            .map(|(id, _, _)| id)
    }

    /// Set the theme
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
}

impl Buffer for BufferListBuffer {
    fn id(&self) -> BufferId {
        "*Buffer List*".to_string()
    }

    fn name(&self) -> String {
        "*Buffer List*".to_string()
    }

    fn can_close(&self) -> bool {
        true
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Buffer List Commands:".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑/k         - Move up".to_string(),
            "  ↓/j         - Move down".to_string(),
            "  Enter/Space - Switch to selected buffer".to_string(),
            "".to_string(),
            "Actions:".to_string(),
            "  d           - Mark buffer for deletion".to_string(),
            "  x           - Execute deletions".to_string(),
            "  r/F5        - Refresh buffer list".to_string(),
            "  q           - Close this buffer".to_string(),
            "".to_string(),
            "Buffer symbols:".to_string(),
            "  *           - Current buffer".to_string(),
        ]
    }
}

impl UIComponent for BufferListBuffer {
    fn handle_action(&mut self, action: UIAction) -> UIAction {
        match action {
            UIAction::MoveUp => {
                self.select_previous();
                UIAction::Render
            }
            UIAction::MoveDown => {
                self.select_next();
                UIAction::Render
            }
            UIAction::SelectItem => {
                if let Some(buffer_id) = self.selected_buffer_id() {
                    UIAction::SwitchBuffer(buffer_id.clone())
                } else {
                    UIAction::ShowMessage("No buffer selected".to_string())
                }
            }
            UIAction::Refresh => UIAction::ShowMessage("Buffer list refreshed".to_string()),
            UIAction::Quit => UIAction::CloseBuffer(self.id()),
            _ => UIAction::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .buffer_items
            .iter()
            .map(|(id, name, is_current)| {
                let marker = if *is_current { "*" } else { " " };
                let display_text = if id != name {
                    format!("{} {} ({})", marker, name, id)
                } else {
                    format!("{} {}", marker, name)
                };

                let style = if *is_current {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };

                ListItem::new(display_text).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Buffer List")
                    .title_style(self.theme.title_style())
                    .border_style(if self.focused {
                        self.theme.border_focused_style()
                    } else {
                        self.theme.border_style()
                    }),
            )
            .style(self.theme.text_style())
            .highlight_style(self.theme.selected_style());

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn title(&self) -> String {
        "*Buffer List*".to_string()
    }

    fn has_focus(&self) -> bool {
        self.focused
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}

impl Default for BufferListBuffer {
    fn default() -> Self {
        Self::new()
    }
}
