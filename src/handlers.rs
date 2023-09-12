use crossterm::event::{KeyEvent, KeyModifiers};

use crate::app::{ActiveBlock, App};

pub fn handle_up(app: &mut App, key: KeyEvent) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::NetworkRequests => app.active_block = ActiveBlock::RequestDetails,
            _ => app.active_block = ActiveBlock::NetworkRequests,
        },
        _ => match app.active_block {
            ActiveBlock::NetworkRequests => {
                if app.selection_index > 0 {
                    app.selection_index = app.selection_index - 1;
                }
            }
            _ => {}
        },
    }
}

pub fn handle_down(app: &mut App, key: KeyEvent) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::NetworkRequests => app.active_block = ActiveBlock::RequestDetails,
            _ => app.active_block = ActiveBlock::NetworkRequests,
        },
        _ => match app.active_block {
            ActiveBlock::NetworkRequests => {
                let length = app.requests.len();

                if app.selection_index + 1 < length {
                    app.selection_index = app.selection_index + 1;
                }
            }
            _ => {}
        },
    }
}

pub fn handle_left(app: &mut App, key: KeyEvent) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::NetworkRequests => app.active_block = ActiveBlock::RequestDetails,
            _ => app.active_block = ActiveBlock::NetworkRequests,
        },
        _ => match app.active_block {
            _ => {}
        },
    }
}

pub fn handle_right(app: &mut App, key: KeyEvent) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::NetworkRequests => app.active_block = ActiveBlock::RequestDetails,
            _ => app.active_block = ActiveBlock::NetworkRequests,
        },
        _ => match app.active_block {
            _ => {}
        },
    }
}

pub fn handle_enter(app: &mut App, _key: KeyEvent) {
    app.active_block = ActiveBlock::RequestDetails
}

pub fn handle_esc(app: &mut App, _key: KeyEvent) {
    app.active_block = ActiveBlock::NetworkRequests
}
