---
name: add-new-buffer
description: Add a new UI buffer to the podcast-tui application. Covers creating the file, implementing the Buffer trait, registering with BufferManager, adding keybindings, and updating docs.
---

# Skill: Add a New UI Buffer

## When to use
When you need a new "view" in the TUI (e.g., a statistics buffer, a search results buffer, a settings buffer).

## Checklist

### 1. Create the buffer file
Create `src/ui/buffers/<name>.rs`. Implement the `Buffer` trait:

```rust
use std::any::Any;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use crate::ui::{UIAction, buffers::Buffer};

pub struct MyNewBuffer {
    title: String,
    // your fields
}

impl MyNewBuffer {
    pub fn new() -> Self {
        Self { title: "My Buffer".to_string() }
    }
}

impl Buffer for MyNewBuffer {
    fn render(&self, frame: &mut Frame, area: Rect, _is_focused: bool) {
        // render logic here
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<UIAction> {
        match key.code {
            crossterm::event::KeyCode::Char('q') => Some(UIAction::CloseBuffer),
            _ => None,
        }
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
```

### 2. Register in mod.rs
In `src/ui/buffers/mod.rs`:
- Add `pub mod <name>;` 
- Add a typed getter to `BufferManager`:

```rust
pub fn get_my_new_buffer_mut_by_id(&mut self, id: &str) -> Option<&mut MyNewBuffer> {
    self.get_buffer_mut(id)?.as_any_mut().downcast_mut::<MyNewBuffer>()
}
```

### 3. Add keybinding (optional)
In `src/ui/keybindings.rs`, bind an F-key or command:

```rust
self.bind_key(KeyChord::function(8), UIAction::SwitchBuffer("my-buffer".to_string()));
```

### 4. Update help text
In `src/ui/buffers/help.rs`, add the new buffer to the help content.

### 5. Update documentation
- Add to buffer table in `AGENTS.md` Feature Map
- Add to `.github/instructions/ui-buffers.instructions.md` buffer table
- Add keybinding to `docs/KEYBINDINGS.md` if applicable

### 6. Write tests
Unit test the `handle_key` method for key actions, and any data loading logic.

## Notes
- Buffer IDs are lowercase-kebab-case strings, typically `<name>-{uuid}`
- Background data loading uses `tokio::spawn` + `AppEvent::BufferDataRefreshed`
- See `src/ui/buffers/playlist_list.rs` as a recent, complete example
