use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use podcast_tui::ui::keybindings::KeyHandler;

fn main() {
    let mut handler = KeyHandler::new();
    
    // Test the 'a' key
    let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    let action = handler.handle_key(key_event);
    println!("'a' key produces action: {:?}", action);
    
    // Test the 'q' key
    let key_event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    let action = handler.handle_key(key_event);
    println!("'q' key produces action: {:?}", action);
    
    // Test Ctrl+C
    let key_event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let action = handler.handle_key(key_event);
    println!("Ctrl+C produces action: {:?}", action);
    
    // Test Ctrl+X then Ctrl+C
    let key1 = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
    let action1 = handler.handle_key(key1);
    println!("Ctrl+X produces action: {:?}", action1);
    
    let key2 = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let action2 = handler.handle_key(key2);
    println!("Then Ctrl+C produces action: {:?}", action2);
}