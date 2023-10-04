use std::time::Duration;

use crossterm::event::{KeyEvent, KeyModifiers};
use futures_channel::mpsc::UnboundedSender;
use tokio::time::sleep;

use crate::app::{ActiveBlock, App, RequestDetailsPane};
use crate::consts::{
    NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE, REQUEST_BODY_UNUSABLE_VERTICAL_SPACE,
    RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE,
};
use crate::parser::{generate_curl_command, pretty_parse_body};
use crate::utils::{
    calculate_scrollbar_position, get_content_length, get_currently_selected_trace,
    parse_query_params, set_content_length,
};
use crate::UIDispatchEvent;

pub struct HandlerMetadata {
    pub main_height: u16,
    pub response_body_rectangle_height: u16,
    pub response_body_rectangle_width: u16,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Direction {
    Up,
    Down,
    Right,
    Left,
}

fn reset_request_and_response_body_ui_state(app: &mut App) {
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
}

fn handle_vertical_response_body_scroll(app: &mut App, rect: usize, direction: Direction) {
    let trace = get_currently_selected_trace(&app).unwrap();

    let response_body_content_height = rect - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE;

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

fn handle_vertical_request_body_scroll(app: &mut App, rect: usize, direction: Direction) {
    let trace = get_currently_selected_trace(&app).unwrap();

    let request_body_content_height = rect - REQUEST_BODY_UNUSABLE_VERTICAL_SPACE;

    let number_of_lines = trace.pretty_response_body_lines.unwrap();

    if number_of_lines > request_body_content_height {
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

fn handle_horizontal_response_body_scroll(app: &mut App, rect: usize, direction: Direction) {
    let (_req, res) = get_content_length(app);

    if res.is_some() {
        let horizontal_content_length = res.unwrap().horizontal;

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

fn handle_horizontal_request_body_scroll(app: &mut App, rect: usize, direction: Direction) {
    let (req, _res) = get_content_length(app);

    if req.is_some() {
        let horizontal_content_length = req.unwrap().horizontal;

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

pub fn handle_up(app: &mut App, key: KeyEvent, additinal_metadata: HandlerMetadata) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::ResponseDetails => app.active_block = ActiveBlock::RequestDetails,
            _ => {}
        },
        _ => match (app.active_block, app.request_details_block) {
            (ActiveBlock::TracesBlock, _) => {
                if app.main.index > 0 {
                    app.main.index = app.main.index - 1;

                    if app.main.index < app.main.offset {
                        app.main.offset -= 1;
                    }
                }

                let number_of_lines: u16 = app.items.len().try_into().unwrap();

                let usable_height = additinal_metadata.main_height
                    - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16;

                if usable_height < number_of_lines {
                    let overflown_number_count: u16 = number_of_lines
                        - (additinal_metadata.main_height
                            - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16);

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

                app.selected_request_header_index = next_index
            }
            (ActiveBlock::RequestBody, _) => {
                handle_vertical_request_body_scroll(
                    app,
                    additinal_metadata.response_body_rectangle_height as usize,
                    Direction::Up,
                );
            }
            (ActiveBlock::ResponseDetails, _) => {
                let next_index = if app.selected_response_header_index == 0 {
                    0
                } else {
                    app.selected_response_header_index - 1
                };

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

// NOTE: Find something like urlSearchParams for JS.
pub fn handle_down(app: &mut App, key: KeyEvent, additinal_metadata: HandlerMetadata) {
    match key.modifiers {
        KeyModifiers::CONTROL => match app.active_block {
            ActiveBlock::RequestDetails => app.active_block = ActiveBlock::ResponseDetails,
            _ => {}
        },
        _ => match (app.active_block, app.request_details_block) {
            (ActiveBlock::TracesBlock, _) => {
                let length = app.items.len();
                let number_of_lines: u16 = length.try_into().unwrap();

                let usable_height = additinal_metadata.main_height
                    - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16;

                if app.main.index + 1 < length {
                    if app.main.index > {
                        additinal_metadata.main_height
                            - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16
                            - 2
                    } as usize
                        && app.main.offset as u16 + usable_height < number_of_lines
                    {
                        app.main.offset += 1;
                    }

                    app.main.index = app.main.index + 1;
                }

                reset_request_and_response_body_ui_state(app);

                set_content_length(app);

                if usable_height < number_of_lines {
                    let overflown_number_count: u16 = number_of_lines
                        - (additinal_metadata.main_height
                            - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16);

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
                let item = get_currently_selected_trace(app).unwrap();

                let params = parse_query_params(item.uri.clone());

                let next_index = if app.selected_params_index + 1 >= params.len() {
                    params.len() - 1
                } else {
                    app.selected_params_index + 1
                };

                app.selected_params_index = next_index
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Headers) => {
                let item = get_currently_selected_trace(app).unwrap();

                let item_length = item.request_headers.len();

                let next_index = if app.selected_request_header_index + 1 >= item_length {
                    item_length - 1
                } else {
                    app.selected_request_header_index + 1
                };

                app.selected_request_header_index = next_index
            }
            (ActiveBlock::ResponseDetails, _) => {
                let item = get_currently_selected_trace(app).unwrap();

                if item.duration.is_some() {
                    let item_length = item.response_headers.len();

                    let next_index = if app.selected_response_header_index + 1 >= item_length {
                        item_length - 1
                    } else {
                        app.selected_response_header_index + 1
                    };

                    app.selected_response_header_index = next_index
                }
            }
            (ActiveBlock::RequestBody, _) => {
                handle_vertical_request_body_scroll(
                    app,
                    additinal_metadata.response_body_rectangle_height as usize,
                    Direction::Up,
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

pub fn handle_left(app: &mut App, _key: KeyEvent, metadata: HandlerMetadata) {
    match app.active_block {
        ActiveBlock::ResponseBody => handle_horizontal_response_body_scroll(
            app,
            metadata.response_body_rectangle_width as usize,
            Direction::Left,
        ),
        ActiveBlock::RequestBody => handle_horizontal_request_body_scroll(
            app,
            metadata.response_body_rectangle_width as usize,
            Direction::Left,
        ),
        _ => {}
    }
}

pub fn handle_right(app: &mut App, _key: KeyEvent, metadata: HandlerMetadata) {
    match &app.active_block {
        ActiveBlock::ResponseBody => {
            handle_horizontal_response_body_scroll(
                app,
                metadata.response_body_rectangle_width as usize,
                Direction::Right,
            );
        }
        ActiveBlock::RequestBody => {
            handle_horizontal_request_body_scroll(
                app,
                metadata.response_body_rectangle_width as usize,
                Direction::Right,
            );
        }
        _ => {}
    };
}

pub fn handle_enter(app: &mut App, _key: KeyEvent) {
    if app.active_block == ActiveBlock::TracesBlock {
        app.active_block = ActiveBlock::RequestDetails
    }
}

pub fn handle_esc(app: &mut App, _key: KeyEvent) {
    app.active_block = ActiveBlock::TracesBlock
}

pub fn handle_tab(app: &mut App, _key: KeyEvent) {
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

pub fn handle_back_tab(app: &mut App, _key: KeyEvent) {
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
    let trace = get_currently_selected_trace(app).unwrap();

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
}

pub fn handle_go_to_end(app: &mut App, additional_metadata: HandlerMetadata) {
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
        _ => {}
    }
}

pub fn handle_go_to_start(app: &mut App, _additional_metadata: HandlerMetadata) {
    match app.active_block {
        ActiveBlock::TracesBlock => {
            app.main.index = 0;

            app.main.offset = 0;

            app.main.scroll_state = app.main.scroll_state.position(0);

            reset_request_and_response_body_ui_state(app);
        }
        _ => {}
    }
}
