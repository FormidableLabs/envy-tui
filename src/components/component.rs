use std::error::Error;

use crossterm::event::KeyEvent;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::Action,
    tui::{Event, Frame, Rect},
};

pub trait Component {
    #[allow(unused_variables)]
    fn register_action_handler(
        &mut self,
        tx: UnboundedSender<Action>,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>, Box<dyn Error>> {
        let r = match event {
            Some(Event::Key(key_event)) => self.handle_key_events(key_event)?,
            _ => None,
        };
        Ok(r)
    }
    #[allow(unused_variables)]
    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>, Box<dyn Error>> {
        Ok(None)
    }
    #[allow(unused_variables)]
    fn update(&mut self, action: Action) -> Result<Option<Action>, Box<dyn Error>> {
        Ok(None)
    }
    fn render(&mut self, f: &mut Frame, r: Rect) -> Result<(), Box<dyn Error>>;

    fn on_mount(&mut self) -> Result<Option<Action>, Box<dyn Error>>;
}
