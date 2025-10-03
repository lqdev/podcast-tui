#[cfg(test)]
mod test_keys {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use crate::ui::keybindings::KeyHandler;
    use crate::ui::UIAction;

    #[test]
    fn test_key_bindings() {
        let mut handler = KeyHandler::new();
        
        // Test the 'a' key
        let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let action = handler.handle_key(key_event);
        println!("'a' key produces action: {:?}", action);
        assert_eq!(action, UIAction::AddPodcast);
        
        // Test the 'q' key
        let key_event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let action = handler.handle_key(key_event);
        println!("'q' key produces action: {:?}", action);
        assert_eq!(action, UIAction::Quit);
    }
}