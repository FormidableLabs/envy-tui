use crate::app::{Action, ActiveBlock, RequestDetailsPane};
use crate::components::home::Home;
use crate::consts::{
    NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE, REQUEST_HEADERS_UNUSABLE_VERTICAL_SPACE,
    RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE, RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE,
};
use crate::parser::{generate_curl_command, pretty_parse_body};
use crate::services::websocket::Trace;
use crate::utils::{
    calculate_scrollbar_position, get_content_length, get_currently_selected_trace,
    parse_query_params, set_content_length,
};
use crossterm::event::{KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct HandlerMetadata {
    pub main_height: u16,
    pub response_body_rectangle_height: u16,
    pub response_body_rectangle_width: u16,
    pub request_body_rectangle_height: u16,
    pub request_body_rectangle_width: u16,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Direction {
    Up,
    Down,
    Right,
    Left,
}

fn reset_request_and_response_body_ui_state(app: &mut Home) {
    app.response_body.offset = 0;
    app.response_body.horizontal_offset = 0;

    app.request_body.offset = 0;
    app.request_body.horizontal_offset = 0;

    app.response_body.horizontal_scroll_state =
        app.response_body.horizontal_scroll_state.position(0);
    app.response_body.scroll_state = app.response_body.scroll_state.position(0);

    app.request_body.offset = 0;
    app.request_body.horizontal_offset = 0;

    app.request_body.offset = 0;
    app.request_body.horizontal_offset = 0;

    app.request_body.horizontal_scroll_state = app.request_body.horizontal_scroll_state.position(0);
    app.request_body.scroll_state = app.request_body.scroll_state.position(0);

    app.response_details.offset = 0;

    app.response_details.scroll_state = app.response_details.scroll_state.position(0);

    app.selected_response_header_index = 0;

    app.request_details.offset = 0;

    app.request_details.scroll_state = app.request_details.scroll_state.position(0);

    app.selected_request_header_index = 0;
}

pub fn handle_debug(app: &mut Home) -> Option<Action> {
    app.previous_block = Some(app.active_block);
    app.active_block = ActiveBlock::Debug;
    None
}

pub fn handle_help(app: &mut Home) -> Option<Action> {
    app.previous_block = Some(app.active_block);
    app.active_block = ActiveBlock::Help;
    None
}

pub fn handle_up(
    app: &mut Home,
    key: KeyEvent,
    additinal_metadata: HandlerMetadata,
) -> Option<Action> {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::ResponseDetails => {
                app.active_block = ActiveBlock::RequestDetails;
                None
            }
            _ => None,
        },
        _ => match (app.active_block, app.request_details_block) {
            (ActiveBlock::TracesBlock, _) => {
                if app.main.index > 0 {
                    app.main.index -= 1;

                    if app.main.index < app.main.offset {
                        app.main.offset -= 1;
                    }

                    return Some(Action::SelectTrace(
                        get_currently_selected_trace(app).cloned(),
                    ));
                }

                let number_of_lines: u16 = app.items.len().try_into().unwrap();

                let usable_height = additinal_metadata
                    .main_height
                    .saturating_sub(NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16);

                if usable_height < number_of_lines {
                    let overflown_number_count: u16 = number_of_lines
                        - (additinal_metadata
                            .main_height
                            .saturating_sub(NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16));

                    let position = calculate_scrollbar_position(
                        number_of_lines,
                        app.main.offset,
                        overflown_number_count,
                    );

                    app.main.scroll_state = app.main.scroll_state.position(position.into());
                }

                reset_request_and_response_body_ui_state(app);

                set_content_length(app);

                app.selected_params_index = 0;

                return Some(Action::SelectTrace(
                    get_currently_selected_trace(app).cloned(),
                ));
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Query) => {
                let next_index = if app.selected_params_index == 0 {
                    0
                } else {
                    app.selected_params_index - 1
                };

                app.selected_params_index = next_index;

                None
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Headers) => {
                let next_index = if app.selected_request_header_index == 0 {
                    0
                } else {
                    app.selected_request_header_index - 1
                };

                let item_length = app.selected_trace.as_ref().unwrap().request_headers.len();

                let usable_height = additinal_metadata
                    .request_body_rectangle_height
                    .checked_sub(RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE as u16)
                    .unwrap_or_default();

                if item_length > usable_height as usize {
                    if next_index < app.request_details.offset {
                        app.request_details.offset -= 1;
                    }

                    let next_position = calculate_scrollbar_position(
                        item_length as u16,
                        app.request_details.offset,
                        item_length as u16 - (usable_height),
                    );

                    app.request_details.scroll_state = app
                        .request_details
                        .scroll_state
                        .position(next_position.into());
                }

                app.selected_request_header_index = next_index;

                None
            }
            (ActiveBlock::ResponseDetails, _) => {
                let next_index = if app.selected_response_header_index == 0 {
                    0
                } else {
                    app.selected_response_header_index - 1
                };

                let item_length = app.selected_trace.as_ref().unwrap().response_headers.len();

                let usable_height = additinal_metadata.response_body_rectangle_height
                    - RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE as u16;

                if item_length > usable_height as usize {
                    if next_index < app.response_details.offset {
                        app.response_details.offset -= 1;
                    }

                    let next_position = calculate_scrollbar_position(
                        item_length as u16,
                        app.response_details.offset,
                        item_length as u16 - (usable_height),
                    );

                    app.response_details.scroll_state = app
                        .response_details
                        .scroll_state
                        .position(next_position.into());
                }

                app.selected_response_header_index = next_index;
                None
            }
            _ => None,
        },
    }
}

pub fn handle_adjust_scroll_bar(
    app: &mut Home,
    additinal_metadata: HandlerMetadata,
) -> Option<Action> {
    let length = app.items.len();

    let usable_height = additinal_metadata
        .main_height
        .saturating_sub(NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE.try_into().unwrap());

    let number_of_lines: u16 = length.try_into().unwrap();

    reset_request_and_response_body_ui_state(app);

    set_content_length(app);

    if usable_height < number_of_lines {
        let overflown_number_count: u16 = number_of_lines.saturating_sub(
            additinal_metadata
                .main_height
                .saturating_sub(NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16),
        );

        let position =
            calculate_scrollbar_position(number_of_lines, app.main.offset, overflown_number_count);

        app.main.scroll_state = app.main.scroll_state.position(position.into());
    }

    app.selected_params_index = 0;

    None
}

// NOTE: Find something like urlSearchParams for JS.
pub fn handle_down(
    app: &mut Home,
    key: KeyEvent,
    additinal_metadata: HandlerMetadata,
) -> Option<Action> {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::RequestDetails => {
                app.active_block = ActiveBlock::ResponseDetails;
                None
            }
            _ => None,
        },
        _ => match (app.active_block, app.request_details_block) {
            (ActiveBlock::TracesBlock, _) => {
                let length = app.items.len();
                let number_of_lines: u16 = length.try_into().unwrap();

                let usable_height = additinal_metadata
                    .main_height
                    .saturating_sub(NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE.try_into().unwrap());

                if app.main.index + 1 < length {
                    if app.main.index > {
                        additinal_metadata
                            .main_height
                            .saturating_sub(NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16)
                            .saturating_sub(2)
                    } as usize
                        && app.main.offset as u16 + usable_height < number_of_lines
                    {
                        app.main.offset += 1;
                    }

                    app.main.index += 1;
                }

                reset_request_and_response_body_ui_state(app);

                set_content_length(app);

                if usable_height < number_of_lines {
                    let overflown_number_count: u16 = number_of_lines.saturating_sub(
                        additinal_metadata
                            .main_height
                            .saturating_sub(NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16),
                    );

                    let position = calculate_scrollbar_position(
                        number_of_lines,
                        app.main.offset,
                        overflown_number_count,
                    );

                    app.main.scroll_state = app.main.scroll_state.position(position.into());
                }

                app.selected_params_index = 0;

                return Some(Action::SelectTrace(
                    get_currently_selected_trace(app).cloned(),
                ));
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Query) => {
                let item = app.selected_trace.as_ref().unwrap();

                let params = parse_query_params(item.uri.clone());

                let next_index = if app.selected_params_index + 1 >= params.len() {
                    params.len() - 1
                } else {
                    app.selected_params_index + 1
                };

                None
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Headers) => {
                let item = app.selected_trace.as_ref().unwrap();

                let item_length = item.request_headers.len();

                let next_index = if app.selected_request_header_index + 1 >= item_length {
                    item_length - 1
                } else {
                    app.selected_request_header_index + 1
                };

                app.selected_request_header_index = next_index;

                let usable_height = additinal_metadata
                    .request_body_rectangle_height
                    .checked_sub(RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE as u16)
                    .unwrap_or_default();

                let requires_scrollbar = item_length as u16 >= usable_height;

                if requires_scrollbar {
                    let current_index_hit_viewport_end =
                        app.selected_request_header_index >= { usable_height as usize };

                    let offset_does_not_intersects_bottom_of_rect =
                        (app.request_details.offset as u16 + usable_height) < item_length as u16;

                    if current_index_hit_viewport_end && offset_does_not_intersects_bottom_of_rect {
                        app.request_details.offset += 1;
                    }

                    let next_position = calculate_scrollbar_position(
                        item_length as u16,
                        app.request_details.offset,
                        item_length as u16 - (usable_height),
                    );

                    app.request_details.scroll_state = app
                        .request_details
                        .scroll_state
                        .position(next_position.into());
                }

                None
            }
            (ActiveBlock::ResponseDetails, _) => {
                let item = app.selected_trace.as_ref().unwrap();

                if item.duration.is_some() {
                    let item_length = item.response_headers.len();

                    let next_index = if app.selected_response_header_index + 1 >= item_length {
                        item_length - 1
                    } else {
                        app.selected_response_header_index + 1
                    };

                    app.selected_response_header_index = next_index;

                    let usable_height = additinal_metadata
                        .response_body_rectangle_height
                        .checked_sub(RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE as u16)
                        .unwrap_or_default();

                    let requires_scrollbar = item_length as u16 >= usable_height;

                    if requires_scrollbar {
                        let current_index_hit_viewport_end =
                            app.selected_response_header_index >= { usable_height as usize };

                        let offset_does_not_intersects_bottom_of_rect =
                            (app.response_details.offset as u16 + usable_height)
                                < item_length as u16;

                        if current_index_hit_viewport_end
                            && offset_does_not_intersects_bottom_of_rect
                        {
                            app.response_details.offset += 1;
                        }

                        let next_position = calculate_scrollbar_position(
                            item_length as u16,
                            app.response_details.offset,
                            item_length as u16 - (usable_height),
                        );

                        app.response_details.scroll_state = app
                            .response_details
                            .scroll_state
                            .position(next_position.into());
                    }
                }

                None
            }
            _ => None,
        },
    }
}

pub fn handle_enter(app: &mut Home) -> Option<Action> {
    if app.active_block == ActiveBlock::TracesBlock {
        app.active_block = ActiveBlock::RequestDetails
    }
    None
}

pub fn handle_esc(app: &mut Home) -> Option<Action> {
    app.active_block = ActiveBlock::TracesBlock;
    None
}

pub fn handle_new_search(app: &mut Home) -> Option<Action> {
    app.search_query.clear();
    app.active_block = ActiveBlock::SearchQuery;
    None
}

pub fn handle_search_push(app: &mut Home, c: char) -> Option<Action> {
    app.search_query.push(c);
    None
}

pub fn handle_search_pop(app: &mut Home) -> Option<Action> {
    app.search_query.pop();
    if app.search_query.is_empty() {
        handle_search_exit(app);
    }
    None
}

pub fn handle_search_exit(app: &mut Home) -> Option<Action> {
    app.active_block = ActiveBlock::TracesBlock;
    None
}

pub fn handle_tab(app: &mut Home) -> Option<Action> {
    let next_block = match app.active_block {
        ActiveBlock::TracesBlock => ActiveBlock::RequestSummary,
        ActiveBlock::RequestSummary => ActiveBlock::RequestDetails,
        ActiveBlock::RequestDetails => ActiveBlock::RequestBody,
        ActiveBlock::RequestBody => ActiveBlock::ResponseDetails,
        ActiveBlock::ResponseDetails => ActiveBlock::ResponseBody,
        ActiveBlock::ResponseBody => ActiveBlock::TracesBlock,
        _ => app.active_block,
    };

    if next_block != app.active_block {
        app.active_block = next_block;
        Some(Action::ActivateBlock(next_block))
    } else {
        None
    }
}

pub fn handle_back_tab(app: &mut Home) -> Option<Action> {
    let next_block = match app.active_block {
        ActiveBlock::TracesBlock => ActiveBlock::ResponseBody,
        ActiveBlock::RequestSummary => ActiveBlock::TracesBlock,
        ActiveBlock::RequestDetails => ActiveBlock::RequestSummary,
        ActiveBlock::RequestBody => ActiveBlock::RequestDetails,
        ActiveBlock::ResponseDetails => ActiveBlock::RequestBody,
        ActiveBlock::ResponseBody => ActiveBlock::ResponseDetails,
        _ => app.active_block,
    };

    if next_block != app.active_block {
        app.active_block = next_block;
        Some(Action::ActivateBlock(next_block))
    } else {
        None
    }
}

pub fn handle_pane_next(app: &mut Home) -> Option<Action> {
    match (app.active_block, app.request_details_block) {
        (ActiveBlock::RequestDetails, RequestDetailsPane::Headers) => {
            app.request_details_block = RequestDetailsPane::Query
        }
        (ActiveBlock::RequestDetails, RequestDetailsPane::Query) => {
            app.request_details_block = RequestDetailsPane::Headers
        }
        (_, _) => {}
    }
    None
}

pub fn handle_pane_prev(app: &mut Home) -> Option<Action> {
    match (app.active_block, app.request_details_block) {
        (ActiveBlock::RequestDetails, RequestDetailsPane::Headers) => {
            app.request_details_block = RequestDetailsPane::Query
        }
        (ActiveBlock::RequestDetails, RequestDetailsPane::Query) => {
            app.request_details_block = RequestDetailsPane::Headers
        }
        (_, _) => {}
    }
    None
}

pub fn handle_yank(app: &mut Home, sender: Option<UnboundedSender<Action>>) -> Option<Action> {
    let trace = app.selected_trace.as_ref().unwrap();

    match app.active_block {
        ActiveBlock::TracesBlock => {
            let cmd = generate_curl_command(&trace);

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
        }
        ActiveBlock::ResponseBody => match &trace.response_body {
            Some(body) => {
                match clippers::Clipboard::get().write_text(pretty_parse_body(body).unwrap()) {
                    Ok(_) => {
                        app.status_message =
                            Some(String::from("Response body copied to clipboard."));
                    }
                    Err(_) => {
                        app.status_message = Some(String::from(
                            "Something went wrong while copying to the clipboard!",
                        ));
                    }
                }
            }
            None => {}
        },
        _ => {}
    };

    app.abort_handlers.iter().for_each(|handler| {
        handler.abort();
    });

    app.abort_handlers.clear();

    if let Some(s) = sender {
        let thread_handler = tokio::spawn(async move {
            sleep(Duration::from_millis(5000)).await;

            s.send(Action::ClearStatusMessage)
        });
        app.abort_handlers.push(thread_handler.abort_handle());
    }

    None
}

pub fn handle_go_to_end(app: &mut Home, additional_metadata: HandlerMetadata) -> Option<Action> {
    match app.active_block {
        ActiveBlock::TracesBlock => {
            let number_of_lines: u16 = app.items.len().try_into().unwrap();

            let usubale_rect_space =
                additional_metadata.main_height - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16;

            app.main.index = number_of_lines as usize - 1;

            let has_overflown = number_of_lines > usubale_rect_space;

            if has_overflown {
                app.main.offset = (number_of_lines - usubale_rect_space) as usize;

                let position = calculate_scrollbar_position(
                    number_of_lines,
                    app.main.offset,
                    number_of_lines - usubale_rect_space,
                );

                app.main.scroll_state = app.main.scroll_state.position(position.into());

                reset_request_and_response_body_ui_state(app);
            }

            return Some(Action::SelectTrace(
                get_currently_selected_trace(app).cloned(),
            ));
        }
        ActiveBlock::RequestBody => {
            let content = get_content_length(app);

            if content.request_body.is_some() {
                let length = content.request_body.unwrap();

                let request_body_content_height = additional_metadata.request_body_rectangle_height
                    - RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE as u16;

                if length.vertical > request_body_content_height {
                    app.request_body.offset =
                        (length.vertical - request_body_content_height) as usize;

                    let overflown_number_count = length.vertical - request_body_content_height;

                    app.request_body.scroll_state = app.request_body.scroll_state.position(
                        calculate_scrollbar_position(
                            length.vertical,
                            app.request_body.offset,
                            overflown_number_count,
                        )
                        .into(),
                    );
                }
            }
        }
        ActiveBlock::ResponseBody => {
            let content = get_content_length(app);

            if content.response_body.is_some() {
                let length = content.response_body.unwrap();

                let response_body_content_height = additional_metadata
                    .response_body_rectangle_height
                    - RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE as u16;

                if length.vertical > response_body_content_height {
                    app.response_body.offset =
                        (length.vertical - response_body_content_height) as usize;

                    let overflown_number_count = length.vertical - response_body_content_height;

                    app.response_body.scroll_state = app.response_body.scroll_state.position(
                        calculate_scrollbar_position(
                            length.vertical,
                            app.response_body.offset,
                            overflown_number_count,
                        )
                        .into(),
                    )
                }
            }
        }
        ActiveBlock::RequestDetails => {
            let item = app.selected_trace.as_ref().unwrap();

            let content = get_content_length(app);

            if item.duration.is_some() {
                let item_length = item.request_headers.len();

                let usable_height = additional_metadata
                    .request_body_rectangle_height
                    .checked_sub(REQUEST_HEADERS_UNUSABLE_VERTICAL_SPACE as u16)
                    .unwrap_or_default();

                let requires_scrollbar = item_length as u16 >= usable_height;

                app.selected_request_header_index = content.request_headers.vertical as usize - 1;

                if requires_scrollbar {
                    let current_index_hit_viewport_end =
                        app.selected_request_header_index >= { usable_height as usize };

                    let offset_does_not_intersects_bottom_of_rect =
                        (app.request_details.offset as u16 + usable_height) < item_length as u16;

                    if current_index_hit_viewport_end && offset_does_not_intersects_bottom_of_rect {
                        app.request_details.offset = item_length - usable_height as usize;
                    }

                    let next_position = calculate_scrollbar_position(
                        item_length as u16,
                        app.request_details.offset,
                        item_length as u16 - (usable_height),
                    );

                    app.request_details.scroll_state = app
                        .request_details
                        .scroll_state
                        .position(next_position.into());
                }
            }
        }
        ActiveBlock::ResponseDetails => {
            let item = app.selected_trace.as_ref().unwrap();
            let content = get_content_length(app);

            if item.duration.is_some() {
                let item_length = item.response_headers.len();

                let usable_height = additional_metadata.response_body_rectangle_height
                    - RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE as u16;

                let requires_scrollbar = item_length as u16 >= usable_height;

                app.selected_response_header_index =
                    content.response_headers.unwrap().vertical as usize - 1;

                if requires_scrollbar {
                    let current_index_hit_viewport_end =
                        app.selected_response_header_index >= { usable_height as usize };

                    let offset_does_not_intersects_bottom_of_rect =
                        (app.response_details.offset as u16 + usable_height) < item_length as u16;

                    if current_index_hit_viewport_end && offset_does_not_intersects_bottom_of_rect {
                        app.response_details.offset = item_length - usable_height as usize;
                    }

                    let next_position = calculate_scrollbar_position(
                        item_length as u16,
                        app.response_details.offset,
                        item_length as u16 - (usable_height),
                    );

                    app.response_details.scroll_state = app
                        .response_details
                        .scroll_state
                        .position(next_position.into());
                }
            }
        }
        _ => {}
    }
    None
}

pub fn handle_go_to_start(app: &mut Home) -> Option<Action> {
    match app.active_block {
        ActiveBlock::TracesBlock => {
            app.main.index = 0;

            app.main.offset = 0;

            app.main.scroll_state = app.main.scroll_state.position(0);

            reset_request_and_response_body_ui_state(app);

            return Some(Action::SelectTrace(
                get_currently_selected_trace(app).cloned(),
            ));
        }
        ActiveBlock::ResponseBody => {
            let content = get_content_length(app);

            if content.response_body.is_some() {
                app.response_body.offset = 0;

                app.response_body.scroll_state = app.response_body.scroll_state.position(0);
            }

            None
        }
        ActiveBlock::RequestBody => {
            let c = get_content_length(app);

            if c.request_body.is_some() {
                app.request_body.offset = 0;

                app.request_body.scroll_state = app.request_body.scroll_state.position(0);
            }

            None
        }
        ActiveBlock::RequestDetails => {
            app.request_details.offset = 0;
            app.selected_request_header_index = 0;

            app.request_details.scroll_state = app.request_details.scroll_state.position(0);

            None
        }
        ActiveBlock::ResponseDetails => {
            let c = get_content_length(app);

            if c.response_headers.is_some() {
                app.response_details.offset = 0;
                app.selected_response_header_index = 0;

                app.response_details.scroll_state = app.response_details.scroll_state.position(0);
            }

            None
        }
        _ => None,
    }
}

pub fn handle_delete_item(app: &mut Home) -> Option<Action> {
    let cloned_items = app.items.clone();
    let items_as_vector = cloned_items.iter().collect::<Vec<&Trace>>();
    let current_trace = items_as_vector.get(app.main.index).copied().unwrap();
    let _ = &app.items.remove(current_trace);

    None
}

pub fn handle_general_status(app: &mut Home, s: String) -> Option<Action> {
    app.status_message = Some(s);
    None
}
