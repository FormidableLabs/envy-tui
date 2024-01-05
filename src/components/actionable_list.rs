use derive_new::new;
use ratatui::widgets::ListState;

use crate::app::Action;

pub struct ActionableListItem {
    pub label: String,
    pub value: String,
    pub action: Option<Action>,
}

#[derive(Default, new)]
pub struct ActionableList {
    pub items: Vec<ActionableListItem>,
    pub state: ListState,
}

impl ActionableList {
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
                if let Some(item) = self.items.get(i) {
                    item.action.clone()
                } else {
                    None
                }
            }
            None => None,
        }
    }
}
