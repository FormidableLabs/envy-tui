use crate::app::{Action, ActiveBlock, FilterScreen, RequestDetailsPane};
use crate::components::home::Home;
use crate::consts::{
    NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE, REQUEST_BODY_UNUSABLE_HORIZONTAL_SPACE,
    REQUEST_BODY_UNUSABLE_VERTICAL_SPACE, REQUEST_HEADERS_UNUSABLE_VERTICAL_SPACE,
    RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE, RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE,
    RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE,
};
use crate::parser::{generate_curl_command, pretty_parse_body};
use crate::render::{get_currently_selected_http_trace, get_services_from_traces};
use crate::services::websocket::Trace;
use crate::utils::{
    calculate_scrollbar_position, get_content_length, get_currently_selected_trace,
    get_rendered_items, parse_query_params, set_content_length, Ordering, TraceSort,
};
use crossterm::event::{KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;

use super::home::{FilterSource, MethodFilter, StatusFilter};

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

fn handle_vertical_response_body_scroll(app: &mut Home, rect: usize, direction: Direction) {
    let trace = get_currently_selected_http_trace(app).unwrap();

    let response_body_content_height = rect - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE;

    if trace.pretty_response_body_lines.is_none() {
        return;
    }

    let number_of_lines = trace.pretty_response_body_lines.unwrap();

    if number_of_lines > response_body_content_height {
        let overflown_number_count = number_of_lines - response_body_content_height;

        if response_body_content_height + app.response_body.offset < number_of_lines
            && direction == Direction::Down
        {
            app.response_body.offset = app.response_body.offset.saturating_add(1);
        }

        if app.response_body.offset != 0 && direction == Direction::Up {
            app.response_body.offset = app.response_body.offset.saturating_sub(1);
        }

        let position = calculate_scrollbar_position(
            number_of_lines as u16,
            app.response_body.offset,
            overflown_number_count as u16,
        );

        app.response_body.scroll_state = app.response_body.scroll_state.position(position);
    }
}

fn handle_vertical_request_body_scroll(app: &mut Home, rect: usize, direction: Direction) {
    let trace = get_currently_selected_http_trace(app).unwrap();

    let request_body_content_height = rect - REQUEST_BODY_UNUSABLE_VERTICAL_SPACE;

    if trace.pretty_request_body_lines.is_none() {
        return;
    }

    let number_of_lines = trace.pretty_request_body_lines.unwrap();

    if number_of_lines > request_body_content_height {
        let overflown = number_of_lines > request_body_content_height;

        if overflown {
            let overflown_number_count = number_of_lines - request_body_content_height;

            if request_body_content_height + app.request_body.offset < number_of_lines
                && direction == Direction::Down
            {
                app.request_body.offset = app.request_body.offset.saturating_add(1);
            }

            if app.request_body.offset != 0 && direction == Direction::Up {
                app.request_body.offset = app.request_body.offset.saturating_sub(1);
            }

            let position = calculate_scrollbar_position(
                number_of_lines as u16,
                app.request_body.offset,
                overflown_number_count as u16,
            );

            app.request_body.scroll_state = app.request_body.scroll_state.position(position);
        }
    }
}

fn handle_horizontal_response_body_scroll(app: &mut Home, rect: usize, direction: Direction) {
    let content = get_content_length(app);

    if content.response_body.is_some() {
        let horizontal_content_length = content.response_body.unwrap().horizontal;

        if horizontal_content_length > rect as u16 {
            let overflown_number_count = horizontal_content_length - rect as u16;

            if app.response_body.horizontal_offset != 0 && direction == Direction::Left {
                app.response_body.horizontal_offset =
                    app.response_body.horizontal_offset.saturating_sub(1);
            }

            if rect - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE + app.response_body.horizontal_offset
                < horizontal_content_length as usize
                && direction == Direction::Right
            {
                app.response_body.horizontal_offset += 1;
            }

            let position = calculate_scrollbar_position(
                horizontal_content_length,
                app.response_body.horizontal_offset,
                overflown_number_count,
            );

            app.response_body.horizontal_scroll_state =
                app.response_body.horizontal_scroll_state.position(position);
        }
    }
}

fn handle_horizontal_request_body_scroll(app: &mut Home, rect: usize, direction: Direction) {
    let content = get_content_length(app);

    if content.request_body.is_some() {
        let horizontal_content_length = content.request_body.unwrap().horizontal;

        if horizontal_content_length > rect as u16 {
            let overflown_number_count = horizontal_content_length - rect as u16;

            if app.request_body.horizontal_offset != 0 && direction == Direction::Left {
                app.request_body.horizontal_offset =
                    app.request_body.horizontal_offset.saturating_sub(1);
            }

            if rect - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE + app.request_body.horizontal_offset
                < horizontal_content_length as usize
                && direction == Direction::Right
            {
                app.request_body.horizontal_offset += 1;
            }

            let position = calculate_scrollbar_position(
                horizontal_content_length,
                app.request_body.horizontal_offset,
                overflown_number_count,
            );

            app.request_body.horizontal_scroll_state =
                app.request_body.horizontal_scroll_state.position(position);
        }
    }
}

pub fn handle_debug(app: &mut Home) {
    let current_block = app.active_block;

    app.previous_blocks.push(current_block);

    app.active_block = ActiveBlock::Debug;
}

pub fn handle_help(app: &mut Home) {
    let current_block = app.active_block;

    app.previous_blocks.push(current_block);

    app.active_block = ActiveBlock::Help;
}

pub fn handle_up(app: &mut Home, key: KeyEvent, additinal_metadata: HandlerMetadata) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::ResponseDetails => app.active_block = ActiveBlock::RequestDetails,
            _ => {}
        },
        _ => match (app.active_block, app.request_details_block) {
            (ActiveBlock::Filter(_), _) => match app.filter_index.checked_sub(1) {
                Some(v) => app.filter_index = v,
                _ => {}
            },
            (ActiveBlock::Sort, _) => match app.sort_index.checked_sub(1) {
                Some(v) => app.sort_index = v,
                _ => {}
            },
            (ActiveBlock::TracesBlock, _) => {
                if app.main.index > 0 {
                    app.main.index -= 1;

                    if app.main.index < app.main.offset {
                        app.main.offset -= 1;
                    }
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

                    app.main.scroll_state = app.main.scroll_state.position(position);
                }

                reset_request_and_response_body_ui_state(app);

                set_content_length(app);

                app.selected_params_index = 0
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Query) => {
                let next_index = if app.selected_params_index == 0 {
                    0
                } else {
                    app.selected_params_index - 1
                };

                app.selected_params_index = next_index
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Headers) => {
                let next_index = if app.selected_request_header_index == 0 {
                    0
                } else {
                    app.selected_request_header_index - 1
                };

                let item_length = get_currently_selected_http_trace(app)
                    .unwrap()
                    .request_headers
                    .len();

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

                    app.request_details.scroll_state =
                        app.request_details.scroll_state.position(next_position);
                }

                app.selected_request_header_index = next_index
            }
            (ActiveBlock::RequestBody, _) => {
                handle_vertical_request_body_scroll(
                    app,
                    additinal_metadata.request_body_rectangle_height as usize,
                    Direction::Up,
                );
            }
            (ActiveBlock::ResponseDetails, _) => {
                let next_index = if app.selected_response_header_index == 0 {
                    0
                } else {
                    app.selected_response_header_index - 1
                };

                let item_length = get_currently_selected_http_trace(app)
                    .unwrap()
                    .response_headers
                    .len();

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

                    app.response_details.scroll_state =
                        app.response_details.scroll_state.position(next_position);
                }

                app.selected_response_header_index = next_index
            }
            (ActiveBlock::ResponseBody, _) => {
                handle_vertical_response_body_scroll(
                    app,
                    additinal_metadata.response_body_rectangle_height as usize,
                    Direction::Up,
                );
            }
            _ => {}
        },
    }
}

pub fn handle_adjust_scroll_bar(app: &mut Home, additinal_metadata: HandlerMetadata) {
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

        app.main.scroll_state = app.main.scroll_state.position(position);
    }

    app.selected_params_index = 0
}

// NOTE: Find something like urlSearchParams for JS.
pub fn handle_down(app: &mut Home, key: KeyEvent, additinal_metadata: HandlerMetadata) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::RequestDetails => app.active_block = ActiveBlock::ResponseDetails,
            _ => {}
        },
        _ => match (app.active_block, app.request_details_block) {
            (ActiveBlock::Filter(FilterScreen::FilterMethod), _) => {
                if app.filter_index + 1 < app.method_filters.len() {
                    app.filter_index += 1;
                }
            }
            (ActiveBlock::Filter(FilterScreen::FilterSource), _) => {
                if app.filter_index + 1 < get_services_from_traces(app).len() + 1 {
                    app.filter_index += 1;
                }
            }
            (ActiveBlock::Filter(FilterScreen::FilterMain), _) => {
                if app.filter_index + 1 < 3 {
                    app.filter_index += 1;
                }
            }
            (ActiveBlock::Filter(FilterScreen::FilterStatus), _) => {
                if app.filter_index + 1 < app.status_filters.len() {
                    app.filter_index += 1;
                }
            }
            (ActiveBlock::Sort, _) => {
                if app.sort_index + 1 < 12 {
                    app.sort_index += 1;
                }
            }
            (ActiveBlock::TracesBlock, _) => {
                let length = get_rendered_items(app).len();

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

                    app.main.scroll_state = app.main.scroll_state.position(position);
                }

                app.selected_params_index = 0
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Query) => {
                let item = get_currently_selected_http_trace(app).unwrap();

                let params = parse_query_params(item.uri.clone());

                let next_index = if app.selected_params_index + 1 >= params.len() {
                    params.len() - 1
                } else {
                    app.selected_params_index + 1
                };

                app.selected_params_index = next_index
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Headers) => {
                let item = get_currently_selected_http_trace(app).unwrap();

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

                    app.request_details.scroll_state =
                        app.request_details.scroll_state.position(next_position);
                }
            }
            (ActiveBlock::ResponseDetails, _) => {
                let item = get_currently_selected_http_trace(app).unwrap();

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

                        app.response_details.scroll_state =
                            app.response_details.scroll_state.position(next_position);
                    }
                }
            }
            (ActiveBlock::RequestBody, _) => {
                handle_vertical_request_body_scroll(
                    app,
                    additinal_metadata.request_body_rectangle_height as usize,
                    Direction::Down,
                );
            }
            (ActiveBlock::ResponseBody, _) => {
                handle_vertical_response_body_scroll(
                    app,
                    additinal_metadata.response_body_rectangle_height as usize,
                    Direction::Down,
                );
            }
            _ => {}
        },
    }
}

pub fn handle_go_to_left(app: &mut Home) {
    app.response_body.horizontal_offset = 0;

    app.response_body.horizontal_scroll_state =
        app.response_body.horizontal_scroll_state.position(0);

    return;
}

pub fn handle_left(app: &mut Home, key: KeyEvent, metadata: HandlerMetadata) {
    match app.active_block {
        ActiveBlock::ResponseBody => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.response_body.horizontal_offset = 0;

                app.response_body.horizontal_scroll_state =
                    app.response_body.horizontal_scroll_state.position(0);

                return;
            }

            handle_horizontal_response_body_scroll(
                app,
                metadata.response_body_rectangle_width as usize,
                Direction::Left,
            )
        }
        ActiveBlock::RequestBody => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.request_body.horizontal_offset = 0;

                app.request_body.horizontal_scroll_state =
                    app.request_body.horizontal_scroll_state.position(0);

                return;
            }

            handle_horizontal_request_body_scroll(
                app,
                metadata.response_body_rectangle_width as usize,
                Direction::Left,
            )
        }
        _ => {}
    }
}

pub fn handle_go_to_right(app: &mut Home, metadata: HandlerMetadata) {
    let content = get_content_length(app);

    let content_length = content.response_body.unwrap().horizontal;

    let width =
        metadata.response_body_rectangle_width - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE as u16;

    app.response_body.horizontal_offset = (content_length - width) as usize;

    let overflown_number_count = content_length - width;

    let position = calculate_scrollbar_position(
        content_length,
        app.response_body.horizontal_offset,
        overflown_number_count,
    );

    app.response_body.horizontal_scroll_state =
        app.response_body.horizontal_scroll_state.position(position);

    return;
}

pub fn handle_right(app: &mut Home, key: KeyEvent, metadata: HandlerMetadata) {
    match &app.active_block {
        ActiveBlock::ResponseBody => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                let content = get_content_length(app);

                let content_length = content.response_body.unwrap().horizontal;

                let width = metadata.response_body_rectangle_width
                    - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE as u16;

                app.response_body.horizontal_offset = (content_length - width) as usize;

                let overflown_number_count = content_length - width;

                let position = calculate_scrollbar_position(
                    content_length,
                    app.response_body.horizontal_offset,
                    overflown_number_count,
                );

                app.response_body.horizontal_scroll_state =
                    app.response_body.horizontal_scroll_state.position(position);

                return;
            }
            handle_horizontal_response_body_scroll(
                app,
                metadata.response_body_rectangle_width as usize,
                Direction::Right,
            );
        }
        ActiveBlock::RequestBody => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                let content = get_content_length(app);

                let content_length = content.request_body.unwrap().horizontal;

                let width = metadata.response_body_rectangle_width
                    - REQUEST_BODY_UNUSABLE_HORIZONTAL_SPACE as u16;

                app.request_body.horizontal_offset = (content_length - width) as usize;

                let overflown_number_count = content_length - width;

                let position = calculate_scrollbar_position(
                    content_length,
                    app.request_body.horizontal_offset,
                    overflown_number_count,
                );

                app.request_body.horizontal_scroll_state =
                    app.request_body.horizontal_scroll_state.position(position);

                return;
            }

            handle_horizontal_request_body_scroll(
                app,
                metadata.response_body_rectangle_width as usize,
                Direction::Right,
            );
        }
        _ => {}
    };
}

pub fn handle_enter(app: &mut Home) {
    if app.active_block == ActiveBlock::TracesBlock {
        app.active_block = ActiveBlock::RequestDetails
    }
}

pub fn handle_esc(app: &mut Home) {
    app.active_block = ActiveBlock::TracesBlock
}

pub fn handle_new_search(app: &mut Home) {
    app.search_query.clear();
    app.active_block = ActiveBlock::SearchQuery;
}

pub fn handle_search_push(app: &mut Home, c: char) {
    app.search_query.push(c);
}

pub fn handle_search_pop(app: &mut Home) {
    app.search_query.pop();
    if app.search_query.is_empty() {
        handle_search_exit(app);
    }
}

pub fn handle_search_exit(app: &mut Home) {
    app.active_block = ActiveBlock::TracesBlock
}

pub fn handle_tab(app: &mut Home) {
    match app.active_block {
        ActiveBlock::TracesBlock => app.active_block = ActiveBlock::RequestSummary,
        ActiveBlock::RequestSummary => app.active_block = ActiveBlock::RequestDetails,
        ActiveBlock::RequestDetails => app.active_block = ActiveBlock::RequestBody,
        ActiveBlock::RequestBody => app.active_block = ActiveBlock::ResponseDetails,
        ActiveBlock::ResponseDetails => app.active_block = ActiveBlock::ResponseBody,
        ActiveBlock::ResponseBody => app.active_block = ActiveBlock::TracesBlock,
        _ => {}
    }
}

pub fn handle_back_tab(app: &mut Home) {
    match app.active_block {
        ActiveBlock::TracesBlock => app.active_block = ActiveBlock::ResponseBody,
        ActiveBlock::RequestSummary => app.active_block = ActiveBlock::TracesBlock,
        ActiveBlock::RequestDetails => app.active_block = ActiveBlock::RequestSummary,
        ActiveBlock::RequestBody => app.active_block = ActiveBlock::RequestDetails,
        ActiveBlock::ResponseDetails => app.active_block = ActiveBlock::RequestBody,
        ActiveBlock::ResponseBody => app.active_block = ActiveBlock::ResponseDetails,
        _ => {}
    }
}

pub fn handle_pane_next(app: &mut Home) {
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

pub fn handle_pane_prev(app: &mut Home) {
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

pub fn handle_yank(app: &mut Home, sender: Option<UnboundedSender<Action>>) {
    let trace = get_currently_selected_trace(app).unwrap();

    if let Some(http_trace) = get_currently_selected_http_trace(app) {
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
            ActiveBlock::ResponseBody => match &http_trace.response_body {
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
    }
}

pub fn handle_go_to_end(app: &mut Home, additional_metadata: HandlerMetadata) {
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

                app.main.scroll_state = app.main.scroll_state.position(position);

                reset_request_and_response_body_ui_state(app);
            }
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

                    app.request_body.scroll_state =
                        app.request_body
                            .scroll_state
                            .position(calculate_scrollbar_position(
                                length.vertical,
                                app.request_body.offset,
                                overflown_number_count,
                            ))
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

                    app.response_body.scroll_state =
                        app.response_body
                            .scroll_state
                            .position(calculate_scrollbar_position(
                                length.vertical,
                                app.response_body.offset,
                                overflown_number_count,
                            ))
                }
            }
        }
        ActiveBlock::RequestDetails => {
            let item = get_currently_selected_http_trace(app);

            let item = item.unwrap();

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

                    app.request_details.scroll_state =
                        app.request_details.scroll_state.position(next_position);
                }
            }
        }
        ActiveBlock::ResponseDetails => {
            let item = get_currently_selected_http_trace(app);
            let item = item.unwrap();
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

                    app.response_details.scroll_state =
                        app.response_details.scroll_state.position(next_position);
                }
            }
        }
        _ => {}
    }
}

pub fn handle_go_to_start(app: &mut Home) {
    match app.active_block {
        ActiveBlock::TracesBlock => {
            app.main.index = 0;

            app.main.offset = 0;

            app.main.scroll_state = app.main.scroll_state.position(0);

            reset_request_and_response_body_ui_state(app);
        }
        ActiveBlock::ResponseBody => {
            let content = get_content_length(app);

            if content.response_body.is_some() {
                app.response_body.offset = 0;

                app.response_body.scroll_state = app.response_body.scroll_state.position(0)
            }
        }
        ActiveBlock::RequestBody => {
            let c = get_content_length(app);

            if c.request_body.is_some() {
                app.request_body.offset = 0;

                app.request_body.scroll_state = app.request_body.scroll_state.position(0)
            }
        }
        ActiveBlock::RequestDetails => {
            app.request_details.offset = 0;
            app.selected_request_header_index = 0;

            app.request_details.scroll_state = app.request_details.scroll_state.position(0)
        }
        ActiveBlock::ResponseDetails => {
            let c = get_content_length(app);

            if c.response_headers.is_some() {
                app.response_details.offset = 0;
                app.selected_response_header_index = 0;

                app.response_details.scroll_state = app.response_details.scroll_state.position(0)
            }
        }
        _ => {}
    }
}

pub fn handle_delete_item(app: &mut Home) {
    let cloned_items = app.items.clone();

    let items_as_vector = cloned_items.iter().collect::<Vec<&Trace>>();

    let current_trace = items_as_vector.get(app.main.index).copied().unwrap();

    let _ = &app.items.remove(current_trace);
}

pub fn handle_general_status(app: &mut Home, s: String) {
    app.status_message = Some(s);
}

pub fn handle_select(app: &mut Home) {
    match app.active_block {
        ActiveBlock::Sort => {
            let filter_items = vec![
                ("Method", "Asc"),
                ("Method", "Desc"),
                ("Source", "Asc"),
                ("Source", "Desc"),
                ("Status", "Asc"),
                ("Status", "Desc"),
                ("Timestamp", "Asc"),
                ("Timestamp", "Desc"),
                ("Duration", "Asc"),
                ("Duration", "Desc"),
                ("Url", "Asc"),
                ("Url", "Desc"),
            ];

            let selected_filter = filter_items.iter().nth(app.sort_index).cloned();

            if let Some(selected_filter) = selected_filter {
                match selected_filter {
                    ("Method", "Asc") => app.order = TraceSort::Method(Ordering::Ascending),
                    ("Method", "Desc") => app.order = TraceSort::Method(Ordering::Descending),
                    ("Status", "Asc") => app.order = TraceSort::Status(Ordering::Ascending),
                    ("Status", "Desc") => app.order = TraceSort::Status(Ordering::Descending),
                    ("Timestamp", "Asc") => app.order = TraceSort::Timestamp(Ordering::Ascending),
                    ("Timestamp", "Desc") => app.order = TraceSort::Timestamp(Ordering::Descending),
                    ("Url", "Asc") => app.order = TraceSort::Url(Ordering::Ascending),
                    ("Url", "Desc") => app.order = TraceSort::Url(Ordering::Descending),
                    ("Duration", "Asc") => app.order = TraceSort::Duration(Ordering::Ascending),
                    ("Duration", "Desc") => app.order = TraceSort::Duration(Ordering::Descending),
                    ("Source", "Asc") => app.order = TraceSort::Source(Ordering::Ascending),
                    ("Source", "Desc") => app.order = TraceSort::Source(Ordering::Descending),
                    (_, _) => {}
                }
            }
        }
        ActiveBlock::Filter(crate::app::FilterScreen::FilterMain) => {
            let blocks = vec!["method", "source", "status"];

            let selected_filter = blocks.iter().nth(app.filter_index).cloned();

            if selected_filter.is_none() {
                return;
            }

            match selected_filter.unwrap() {
                "method" => {
                    app.previous_blocks
                        .push(ActiveBlock::Filter(crate::app::FilterScreen::FilterMain));

                    app.active_block = ActiveBlock::Filter(crate::app::FilterScreen::FilterMethod)
                }
                "source" => {
                    app.previous_blocks
                        .push(ActiveBlock::Filter(crate::app::FilterScreen::FilterMain));

                    app.active_block = ActiveBlock::Filter(crate::app::FilterScreen::FilterSource)
                }
                "status" => {
                    app.previous_blocks
                        .push(ActiveBlock::Filter(crate::app::FilterScreen::FilterMain));

                    app.active_block = ActiveBlock::Filter(crate::app::FilterScreen::FilterStatus)
                }
                _ => {}
            };

            app.filter_index = 0;
        }

        ActiveBlock::Filter(crate::app::FilterScreen::FilterStatus) => {
            let current_service = app
                .status_filters
                .iter()
                .map(|(key, _item)| key)
                .nth(app.filter_index);

            if current_service.is_none() {
                return;
            }

            if let Some(filter) = current_service {
                if let Some(status_filter) = app.status_filters.get(filter) {
                    let d = status_filter.clone();

                    app.status_filters.insert(
                        filter.clone(),
                        StatusFilter {
                            name: d.name.clone(),
                            status: d.status.clone(),
                            selected: !d.selected,
                        },
                    );
                }
            };

            reset_request_and_response_body_ui_state(app);

            app.main.index = 0;

            app.main.offset = 0;

            app.main.scroll_state = app.main.scroll_state.position(0);
        }
        ActiveBlock::Filter(crate::app::FilterScreen::FilterMethod) => {
            let current_service = app
                .method_filters
                .iter()
                .map(|(a, _item)| a)
                .nth(app.filter_index);

            if current_service.is_none() {
                return;
            }

            if let Some(filter) = current_service {
                if let Some(d) = app.method_filters.get(filter) {
                    let d = d.clone();

                    app.method_filters.insert(
                        filter.clone(),
                        MethodFilter {
                            name: d.name.clone(),
                            method: d.method.clone(),
                            selected: !d.selected,
                        },
                    );
                }
            };

            reset_request_and_response_body_ui_state(app);

            app.main.index = 0;

            app.main.offset = 0;

            app.main.scroll_state = app.main.scroll_state.position(0);
        }
        ActiveBlock::Filter(crate::app::FilterScreen::FilterSource) => {
            let mut services = get_services_from_traces(app);

            let mut a: Vec<String> = vec!["All".to_string()];

            a.append(&mut services);

            services = a;

            let selected_filter = services.iter().nth(app.filter_index).cloned();

            if selected_filter.is_none() {
                return;
            }

            if let Some(filter) = selected_filter {
                match filter.as_str() {
                    "All" => app.set_filter_source(FilterSource::All),
                    source => match app.get_filter_source() {
                        FilterSource::All => {
                            let mut set = HashSet::new();

                            set.insert(source.to_string());

                            app.set_filter_source(FilterSource::Applied(set))
                        }
                        FilterSource::Applied(applied_sources) => {
                            if applied_sources.contains(&source.to_string()) {
                                let mut set = applied_sources.clone();

                                set.remove(source);

                                app.set_filter_source(FilterSource::Applied(set))
                            } else {
                                let mut set = applied_sources.clone();

                                set.insert(source.to_string());

                                if set.len() == get_services_from_traces(app).len() {
                                    app.set_filter_source(FilterSource::All)
                                } else {
                                    app.set_filter_source(FilterSource::Applied(set))
                                }
                            }
                        }
                    },
                }
            };

            reset_request_and_response_body_ui_state(app);

            app.main.index = 0;

            app.main.offset = 0;

            let items = get_rendered_items(app);

            let length = items.len();

            set_content_length(app);

            app.main.scroll_state = app.main.scroll_state.content_length(length as u16);
        }
        _ => {}
    }
}
