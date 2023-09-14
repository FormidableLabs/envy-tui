use crossterm::event::{KeyEvent, KeyModifiers};

use crate::app::{ActiveBlock, App};
use crate::utils::parse_query_params;

pub fn handle_up(app: &mut App, key: KeyEvent) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::RequestHeaders => app.active_block = ActiveBlock::RequestDetails,
            ActiveBlock::ResponseHeaders => app.active_block = ActiveBlock::RequestHeaders,
            ActiveBlock::RequestDetails => app.active_block = ActiveBlock::Summary,
            _ => {}
        },
        _ => match app.active_block {
            ActiveBlock::NetworkRequests => {
                if app.selection_index > 0 {
                    app.selection_index = app.selection_index - 1;
                }

                app.selected_params_index = 0
            }
            ActiveBlock::RequestDetails => {
                let next_index = if app.selected_params_index == 0 {
                    0
                } else {
                    app.selected_params_index - 1
                };

                app.selected_params_index = next_index
            }
            _ => {}
        },
    }
}

// NOTE: Find stg like urlsearchparams for JS.
pub fn handle_down(app: &mut App, key: KeyEvent) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::Summary => app.active_block = ActiveBlock::RequestDetails,
            ActiveBlock::RequestDetails => app.active_block = ActiveBlock::RequestHeaders,
            ActiveBlock::RequestHeaders => app.active_block = ActiveBlock::ResponseHeaders,
            _ => {}
        },
        _ => match app.active_block {
            ActiveBlock::NetworkRequests => {
                let length = app.items.len();

                if app.selection_index + 1 < length {
                    app.selection_index = app.selection_index + 1;
                }

                app.selected_params_index = 0
            }
            ActiveBlock::RequestDetails => {
                let index = &app.items[app.selection_index];

                let params = parse_query_params(index.uri.clone());

                let next_index = if app.selected_params_index + 1 >= params.len() {
                    params.len() - 1
                } else {
                    app.selected_params_index + 1
                };

                app.selected_params_index = next_index
            }
            _ => {}
        },
    }
}

pub fn handle_left(app: &mut App, _key: KeyEvent) {
    match app.active_block {
        ActiveBlock::NetworkRequests => {}
        _ => app.active_block = ActiveBlock::NetworkRequests,
    }
}

pub fn handle_right(app: &mut App, _key: KeyEvent) {
    match app.active_block {
        ActiveBlock::NetworkRequests => app.active_block = ActiveBlock::RequestDetails,
        _ => {}
    }
}

pub fn handle_enter(app: &mut App, _key: KeyEvent) {
    if app.active_block == ActiveBlock::NetworkRequests {
        app.active_block = ActiveBlock::Summary
    }
}

pub fn handle_esc(app: &mut App, _key: KeyEvent) {
    app.active_block = ActiveBlock::NetworkRequests
}
