// Event handling system for the UI
//
// This module provides the core event system for handling keyboard input,
// converting them to UI actions, and managing the event loop.

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use tokio::sync::mpsc;
use std::time::{Duration, Instant};

use crate::ui::{UIAction, UIError, UIResult};

/// UI event handler for processing terminal events
#[derive(Clone)]
pub struct UIEventHandler {
    tick_rate: Duration,
}

impl UIEventHandler {
    /// Create a new event handler with the specified tick rate
    pub fn new(tick_rate: Duration) -> Self {        
        Self {
            tick_rate,
        }
    }
    
    /// Run the event loop, sending events to the provided channel
    pub async fn run(&self, event_tx: mpsc::UnboundedSender<UIEvent>) {
        let mut last_tick = Instant::now();
        
        loop {
            let timeout = self.tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or(Duration::ZERO);
            
            if event::poll(timeout).unwrap_or(false) {
                match event::read() {
                    Ok(crossterm_event) => {
                        let ui_event = Self::convert_event(crossterm_event);
                        if event_tx.send(ui_event).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            
            if last_tick.elapsed() >= self.tick_rate {
                if event_tx.send(UIEvent::Tick).is_err() {
                    break;
                }
                last_tick = Instant::now();
            }
        }
    }
    
    /// Convert crossterm events to UI events
    fn convert_event(event: Event) -> UIEvent {
        match event {
            Event::Key(key) => UIEvent::Key(key),
            Event::Mouse(mouse) => UIEvent::Mouse(mouse),
            Event::Resize(w, h) => UIEvent::Resize(w, h),
            _ => UIEvent::Tick,
        }
    }
}

/// UI events that can occur
#[derive(Debug, Clone, PartialEq)]
pub enum UIEvent {
    /// Keyboard input
    Key(crossterm::event::KeyEvent),
    
    /// Mouse input
    Mouse(crossterm::event::MouseEvent),
    
    /// Terminal resize
    Resize(u16, u16),
    
    /// Periodic tick
    Tick,
    
    /// Application should quit
    Quit,
}