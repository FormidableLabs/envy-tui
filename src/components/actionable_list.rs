use derive_new::new;
use ratatui::widgets::ListState;

use crate::app::Action;

pub struct ActionableListItem {
    pub label: String,
    pub value: Option<String>,
    pub action: Option<Action>,
}

impl ActionableListItem {
    pub fn with_label(label: &str) -> Self {
        Self {
            label: label.to_string(),
            value: None,
            action: None,
        }
    }
    pub fn with_labelled_value(label: &str, value: &str) -> Self {
        Self {
            label: label.to_string(),
            value: Some(value.to_string()),
            action: None,
        }
    }
    pub fn with_action(self, action: Action) -> Self {
        Self {
            action: Some(action),
            ..self
        }
    }
}

#[derive(Default, new)]
pub struct ActionableList {
    pub items: Vec<ActionableListItem>,
    pub state: ListState,
}

impl ActionableList {
    pub fn reset(&mut self) {
        self.state.select(None);
    }

    pub fn select(&mut self, index: usize) {
        self.state.select(Some(index));
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len().saturating_sub(1) {
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

    pub fn action(&mut self) -> Option<Action> {
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
