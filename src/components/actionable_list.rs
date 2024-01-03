use crate::app::Action;
use ratatui::widgets::ListState;

pub type ActionableListItem = ((String, String), Option<Action>);

#[derive(Default)]
pub struct ActionableList {
    pub state: ListState,
    pub items: Vec<ActionableListItem>,
}

impl ActionableList {
    pub fn new(items: Vec<ActionableListItem>, state: ListState) -> ActionableList {
        ActionableList { state, items }
    }

    pub fn reset(&mut self) {
        self.state.select(Some(0));
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    i
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn select(&mut self) -> Option<Action> {
        match self.state.selected() {
            Some(i) => {
                if let Some((_labels, action)) = self.items.get(i) {
                    action.clone()
                } else {
                    None
                }
            }
            None => None,
        }
    }
}
