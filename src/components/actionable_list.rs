use crate::app::Action;
use ratatui::widgets::ListState;

pub type ActionableListItem = ((String, String), Option<Action>);

#[derive(Default)]
pub struct ActionableList {
    pub state: ListState,
    pub items: Vec<ActionableListItem>,
    pub actions: Vec<ActionableListItem>,
}

impl ActionableList {
    pub fn with_items(
        items: Vec<ActionableListItem>,
        actions: Vec<ActionableListItem>,
    ) -> ActionableList {
        ActionableList {
            state: ListState::default(),
            items: [items, actions.clone()].concat(),
            actions,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
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
                    self.items.len() - 1
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