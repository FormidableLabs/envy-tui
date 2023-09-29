use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use futures_channel::mpsc::UnboundedSender;
use tokio::time::sleep;

use crate::app::{ActiveBlock, App, Request, RequestDetailsPane, ResponseDetailsPane};
use crate::parser::generate_curl_command;
use crate::utils::parse_query_params;
use crate::UIDispatchEvent;

pub fn handle_up(app: &mut App, key: KeyEvent) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::ResponseDetails => app.active_block = ActiveBlock::RequestDetails,
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
                let next_index = if app.selected_request_header_index == 0 {
                    0
                } else {
                    app.selected_request_header_index - 1
                };

                app.selected_request_header_index = next_index
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

// NOTE: Find something like urlSearchParams for JS.
pub fn handle_down(app: &mut App, key: KeyEvent) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::RequestDetails => app.active_block = ActiveBlock::ResponseDetails,
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
                let index = &app.items.iter().collect::<Vec<&Request>>()[app.selection_index];

                let params = parse_query_params(index.uri.clone());

                let next_index = if app.selected_params_index + 1 >= params.len() {
                    params.len() - 1
                } else {
                    app.selected_params_index + 1
                };

                app.selected_params_index = next_index
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Headers, _) => {
                let item = &app.items.iter().collect::<Vec<&Request>>()[app.selection_index];

                let item_length = item.request_headers.len();

                let next_index = if app.selected_request_header_index + 1 >= item_length {
                    item_length - 1
                } else {
                    app.selected_request_header_index + 1
                };

                app.selected_request_header_index = next_index
            }
            (ActiveBlock::ResponseDetails, _, ResponseDetailsPane::Headers) => {
                let item = &app.items.iter().collect::<Vec<&Request>>()[app.selection_index];

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
        _ => {}
    }
}

pub fn handle_enter(app: &mut App, _key: KeyEvent) {
    if app.active_block == ActiveBlock::NetworkRequests {
        app.active_block = ActiveBlock::RequestDetails
    }
}

pub fn handle_esc(app: &mut App, _key: KeyEvent) {
    app.active_block = ActiveBlock::NetworkRequests
}

pub fn handle_search(app: &mut App, key: KeyEvent) {
    match app.active_block {
        ActiveBlock::SearchQuery => {
            match key.code {
              KeyCode::Backspace => {
                  app.search_query.pop();
              },
              KeyCode::Enter => {
                app.active_block = ActiveBlock::NetworkRequests;
              },
              KeyCode::Char(c) => app.search_query.push(c),
              _ => app.active_block = ActiveBlock::NetworkRequests,
            }
        }
        _ => app.active_block = ActiveBlock::SearchQuery,
    }
}

pub fn handle_tab(app: &mut App, _key: KeyEvent) {
    match app.active_block {
        ActiveBlock::NetworkRequests => app.active_block = ActiveBlock::RequestSummary,
        ActiveBlock::RequestSummary => app.active_block = ActiveBlock::RequestDetails,
        ActiveBlock::RequestDetails => app.active_block = ActiveBlock::ResponseDetails,
        ActiveBlock::ResponseDetails => app.active_block = ActiveBlock::NetworkRequests,
        _ => {}
    }
}

pub fn handle_back_tab(app: &mut App, _key: KeyEvent) {
    match app.active_block {
        ActiveBlock::NetworkRequests => app.active_block = ActiveBlock::ResponseDetails,
        ActiveBlock::RequestSummary => app.active_block = ActiveBlock::NetworkRequests,
        ActiveBlock::RequestDetails => app.active_block = ActiveBlock::RequestSummary,
        ActiveBlock::ResponseDetails => app.active_block = ActiveBlock::RequestDetails,
        _ => {}
    }
}

pub fn handle_pane_next(app: &mut App, _key: KeyEvent) {
    match (app.active_block, app.request_details_block) {
        (ActiveBlock::RequestDetails, RequestDetailsPane::Headers) => {
            app.request_details_block = RequestDetailsPane::Query
        }
        (ActiveBlock::RequestDetails, RequestDetailsPane::Query) => {
            app.request_details_block = RequestDetailsPane::Headers
        }
        (_, _) => {}
    }
}

pub fn handle_pane_prev(app: &mut App, _key: KeyEvent) {
    match (app.active_block, app.request_details_block) {
        (ActiveBlock::RequestDetails, RequestDetailsPane::Headers) => {
            app.request_details_block = RequestDetailsPane::Query
        }
        (ActiveBlock::RequestDetails, RequestDetailsPane::Query) => {
            app.request_details_block = RequestDetailsPane::Headers
        }
        (_, _) => {}
    }
}

pub fn handle_yank(app: &mut App, _key: KeyEvent, loop_sender: UnboundedSender<UIDispatchEvent>) {
    let items_as_vector = app.items.iter().collect::<Vec<&Request>>();

    let selected_item = items_as_vector.get(app.selection_index);

    match selected_item {
        Some(request) => {
            let cmd = generate_curl_command(request);

            match clippers::Clipboard::get().write_text(cmd) {
                Ok(_) => {
                    app.status_message = Some(String::from("Request copied as cURL command!"));
                }
                Err(_) => {
                    app.status_message = Some(String::from(
                        "Something went wrong while copying to the clipboard!",
                    ));
                }
            }

            app.abort_handlers.iter().for_each(|handler| {
                handler.abort();
            });

            app.abort_handlers.clear();

            let thread_handler = tokio::spawn(async move {
                sleep(Duration::from_millis(5000)).await;

                loop_sender.unbounded_send(UIDispatchEvent::ClearStatusMessage)
            });

            app.abort_handlers.push(thread_handler.abort_handle());
        }
        None => {}
    }
}
