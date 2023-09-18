use crossterm::event::{KeyEvent, KeyModifiers};

use crate::app::{ActiveBlock, App, RequestDetailsPane, ResponseDetailsPane};
use crate::utils::parse_query_params;

fn clear_aux_indexes(app: &mut App) {
    app.selected_params_index = 0;
    app.selected_params_index = 0;
}

pub fn handle_up(app: &mut App, key: KeyEvent) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::ResponseDetails => app.active_block = ActiveBlock::RequestDetails,
            ActiveBlock::RequestDetails => app.active_block = ActiveBlock::Summary,
            _ => {}
        },
        _ => match (
            app.active_block,
            app.request_details_block,
            app.response_details_block,
        ) {
            (ActiveBlock::NetworkRequests, _, _) => {
                if app.selection_index > 0 {
                    app.selection_index = app.selection_index - 1;
                }

                app.selected_params_index = 0
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Query, _) => {
                let next_index = if app.selected_params_index == 0 {
                    0
                } else {
                    app.selected_params_index - 1
                };

                app.selected_params_index = next_index
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Headers, _) => {
                let next_index = if app.selected_header_index == 0 {
                    0
                } else {
                    app.selected_header_index - 1
                };

                app.selected_header_index = next_index
            }
            (ActiveBlock::ResponseDetails, _, ResponseDetailsPane::Headers) => {
                let next_index = if app.selected_response_header_index == 0 {
                    0
                } else {
                    app.selected_response_header_index - 1
                };

                app.selected_response_header_index = next_index
            }
            (ActiveBlock::ResponseDetails, _, ResponseDetailsPane::Body) => {}
            _ => {}
        },
    }
}

// NOTE: Find stg like urlsearchparams for JS.
pub fn handle_down(app: &mut App, key: KeyEvent) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::Summary => app.active_block = ActiveBlock::RequestDetails,
            ActiveBlock::RequestDetails => app.active_block = ActiveBlock::ResponseDetails,
            // ActiveBlock::RequestHeaders => app.active_block = ActiveBlock::ResponseHeaders,
            _ => {}
        },
        _ => match (
            app.active_block,
            app.request_details_block,
            app.response_details_block,
        ) {
            (ActiveBlock::NetworkRequests, _, _) => {
                let length = app.items.len();

                if app.selection_index + 1 < length {
                    app.selection_index = app.selection_index + 1;
                }

                app.selected_params_index = 0
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Query, _) => {
                let index = &app.items[app.selection_index];

                let params = parse_query_params(index.uri.clone());

                let next_index = if app.selected_params_index + 1 >= params.len() {
                    params.len() - 1
                } else {
                    app.selected_params_index + 1
                };

                app.selected_params_index = next_index
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Headers, _) => {
                let item = &app.items[app.selection_index];

                let item_length = item.request_headers.len();

                let next_index = if app.selected_header_index + 1 >= item_length {
                    item_length - 1
                } else {
                    app.selected_header_index + 1
                };

                app.selected_header_index = next_index
            }
            (ActiveBlock::ResponseDetails, _, ResponseDetailsPane::Headers) => {
                let item = &app.items[app.selection_index];

                let item_length = item.response_headers.len();

                let next_index = if app.selected_response_header_index + 1 >= item_length {
                    item_length - 1
                } else {
                    app.selected_response_header_index + 1
                };

                app.selected_response_header_index = next_index
            }
            (ActiveBlock::ResponseDetails, _, ResponseDetailsPane::Body) => {}
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
        ActiveBlock::NetworkRequests => app.active_block = ActiveBlock::Summary,
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

pub fn handle_tab(app: &mut App, _key: KeyEvent) {
    match app.active_block {
        ActiveBlock::NetworkRequests => app.active_block = ActiveBlock::Summary,
        ActiveBlock::Summary => app.active_block = ActiveBlock::RequestDetails,
        ActiveBlock::RequestDetails => app.active_block = ActiveBlock::ResponseDetails,
        ActiveBlock::ResponseDetails => {}
    }
}

pub fn handle_back_tab(app: &mut App, _key: KeyEvent) {
    match app.active_block {
        ActiveBlock::NetworkRequests => clear_aux_indexes(app),
        ActiveBlock::Summary => app.active_block = ActiveBlock::NetworkRequests,
        ActiveBlock::RequestDetails => app.active_block = ActiveBlock::Summary,
        ActiveBlock::ResponseDetails => app.active_block = ActiveBlock::RequestDetails,
    }
}

pub fn handle_pane_next(app: &mut App, _key: KeyEvent) {
    match (
        app.active_block,
        app.request_details_block,
        app.response_details_block,
    ) {
        (ActiveBlock::RequestDetails, RequestDetailsPane::Body, _) => {
            app.request_details_block = RequestDetailsPane::Query
        }
        (ActiveBlock::RequestDetails, RequestDetailsPane::Headers, _) => {
            app.request_details_block = RequestDetailsPane::Body
        }
        (ActiveBlock::RequestDetails, RequestDetailsPane::Query, _) => {
            app.request_details_block = RequestDetailsPane::Headers
        }
        (ActiveBlock::ResponseDetails, _, ResponseDetailsPane::Body) => {
            app.response_details_block = ResponseDetailsPane::Headers
        }
        (ActiveBlock::ResponseDetails, _, ResponseDetailsPane::Headers) => {
            app.response_details_block = ResponseDetailsPane::Body
        }
        (_, _, _) => {}
    }
}

pub fn handle_pane_prev(app: &mut App, _key: KeyEvent) {
    match (
        app.active_block,
        app.request_details_block,
        app.response_details_block,
    ) {
        (ActiveBlock::RequestDetails, RequestDetailsPane::Body, _) => {
            app.request_details_block = RequestDetailsPane::Query
        }
        (ActiveBlock::RequestDetails, RequestDetailsPane::Headers, _) => {
            app.request_details_block = RequestDetailsPane::Body
        }
        (ActiveBlock::RequestDetails, RequestDetailsPane::Query, _) => {
            app.request_details_block = RequestDetailsPane::Headers
        }
        (ActiveBlock::ResponseDetails, _, ResponseDetailsPane::Headers) => {
            app.response_details_block = ResponseDetailsPane::Body
        }
        (ActiveBlock::ResponseDetails, _, ResponseDetailsPane::Body) => {
            app.response_details_block = ResponseDetailsPane::Headers
        }
        (_, _, _) => {}
    }
}
