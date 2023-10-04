use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use futures_channel::mpsc::UnboundedSender;
use tokio::time::sleep;

use crate::app::{ActiveBlock, App, Request, RequestDetailsPane};
use crate::consts::{
    NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE, RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE,
};
use crate::parser::{generate_curl_command, pretty_parse_body};
use crate::utils::{get_currently_selected_request, parse_query_params};
use crate::UIDispatchEvent;

pub struct HandlerMetadata {
    pub main_height: u16,
    pub response_body_rectangle_height: u16,
    pub response_body_rectangle_width: u16,
}

fn calculate_scrollbar_position(
    content_length: u16,
    offset: usize,
    overflown_number_count: u16,
) -> u16 {
    let content_length_as_float = content_length as f32;
    let overflown_number_count_as_float = overflown_number_count as f32;
    let offset_as_float = offset as f32;

    ({ (content_length_as_float / overflown_number_count_as_float) * offset_as_float } as u16)
}

pub fn handle_up(app: &mut App, key: KeyEvent, additinal_metadata: HandlerMetadata) {
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

                    app.main.scroll_state = app.main.scroll_state.position(
                        { (number_of_lines / overflown_number_count) * app.main.offset as u16 }
                            .try_into()
                            .unwrap(),
                    );
                }

                // TODO: Makes this conditional and remove const.
                // app.request_body.horizontal_scroll_state =
                //     app.request_body.horizontal_scroll_state.content_length(140);

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
            (ActiveBlock::ResponseDetails, _, _) => {
                let next_index = if app.selected_response_header_index == 0 {
                    0
                } else {
                    app.selected_response_header_index - 1
                };

                app.selected_response_header_index = next_index
            }
            (ActiveBlock::ResponseBody, _, _) => {
                match get_currently_selected_request(&app) {
                    Some(request) => {
                        let response_body_content_height =
                            additinal_metadata.response_body_rectangle_height as usize
                                - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE;

                        let number_of_lines = request.pretty_response_body_lines.unwrap();

                        if app.response_body.offset != 0 {
                            app.response_body.offset = app.response_body.offset.saturating_sub(1);
                        }

                        let overflown_number_count = number_of_lines - response_body_content_height;

                        app.response_body.scroll_state = app.response_body.scroll_state.position(
                            {
                                (number_of_lines / overflown_number_count)
                                    * app.response_body.offset
                            }
                            .try_into()
                            .unwrap(),
                        );
                    }
                    None => {}
                };
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
        _ => match (
            app.active_block,
            app.request_details_block,
            app.response_details_block,
        ) {
            (ActiveBlock::NetworkRequests, _, _) => {
                let length = app.items.len();

                if app.main.index + 1 < length {
                    if app.main.index > {
                        additinal_metadata.main_height
                            - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16
                            - 2
                    } as usize
                    {
                        app.main.offset += 1;
                    }

                    app.main.index = app.main.index + 1;
                }

                let number_of_lines: u16 = length.try_into().unwrap();

                let usable_height = additinal_metadata.main_height
                    - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16;

                let request = get_currently_selected_request(&app).unwrap();

                let r_body = &get_currently_selected_request(&app).unwrap().response_body;

                match r_body {
                    Some(_) => {
                        let response_body_content_height =
                            additinal_metadata.response_body_rectangle_height as usize
                                - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE;

                        let number_of_lines = request.pretty_response_body_lines.unwrap();

                        if response_body_content_height + app.response_body.offset < number_of_lines
                        {
                            app.response_body.offset = app.response_body.offset.saturating_add(1);
                        }

                        let requires_scrollbar = number_of_lines > response_body_content_height;

                        if requires_scrollbar {
                            app.response_body.scroll_state = app
                                .response_body
                                .scroll_state
                                .content_length(number_of_lines as u16)
                                .position(0);
                        }
                    }
                    _ => {}
                }

                if usable_height < number_of_lines {
                    let overflown_number_count: u16 = number_of_lines
                        - (additinal_metadata.main_height
                            - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16);

                    app.main.scroll_state = app.main.scroll_state.position(
                        { (number_of_lines / overflown_number_count) * app.main.offset as u16 }
                            .try_into()
                            .unwrap(),
                    );
                }

                app.selected_params_index = 0
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Query, _) => {
                let index = &app.items.iter().collect::<Vec<&Request>>()[app.main.index];

                let params = parse_query_params(index.uri.clone());

                let next_index = if app.selected_params_index + 1 >= params.len() {
                    params.len() - 1
                } else {
                    app.selected_params_index + 1
                };

                app.selected_params_index = next_index
            }
            (ActiveBlock::RequestDetails, RequestDetailsPane::Headers, _) => {
                let item = &app.items.iter().collect::<Vec<&Request>>()[app.main.index];

                let item_length = item.request_headers.len();

                let next_index = if app.selected_request_header_index + 1 >= item_length {
                    item_length - 1
                } else {
                    app.selected_request_header_index + 1
                };

                app.selected_request_header_index = next_index
            }
            (ActiveBlock::ResponseDetails, _, _) => {
                let item = &app.items.iter().collect::<Vec<&Request>>()[app.main.index];

                let item_length = item.response_headers.len();

                let next_index = if app.selected_response_header_index + 1 >= item_length {
                    item_length - 1
                } else {
                    app.selected_response_header_index + 1
                };

                app.selected_response_header_index = next_index
            }
            (ActiveBlock::ResponseBody, _, _) => {
                match get_currently_selected_request(&app) {
                    Some(request) => {
                        let response_body_content_height =
                            additinal_metadata.response_body_rectangle_height as usize
                                - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE;

                        let number_of_lines = request.pretty_response_body_lines.unwrap();

                        if response_body_content_height + app.response_body.offset < number_of_lines
                        {
                            app.response_body.offset = app.response_body.offset.saturating_add(1);
                        }

                        let overflown_number_count = number_of_lines - response_body_content_height;

                        let state = calculate_scrollbar_position(
                            number_of_lines as u16,
                            app.response_body.offset,
                            overflown_number_count as u16,
                        );

                        app.response_body.scroll_state = app
                            .response_body
                            .scroll_state
                            .position(state.try_into().unwrap());
                    }
                    None => {}
                };
            }
            _ => {}
        },
    }
}

pub fn handle_left(app: &mut App, _key: KeyEvent, metadata: HandlerMetadata) {
    let item = get_currently_selected_request(&app);

    match app.active_block {
        ActiveBlock::ResponseBody => match item {
            Some(item) => {
                let lines = &item.pretty_response_body.as_ref().unwrap();

                let longest = lines
                    .lines()
                    .into_iter()
                    .fold(0, |longest: u16, lines: &str| {
                        let len = lines.len() as u16;

                        len.max(longest)
                    });

                let overflown_number_count = longest
                    - metadata.response_body_rectangle_width
                    - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE as u16;

                if app.response_body.horizontal_offset != 0 {
                    app.response_body.horizontal_offset =
                        app.response_body.horizontal_offset.saturating_sub(1);
                }

                let position = calculate_scrollbar_position(
                    longest,
                    app.response_body.horizontal_offset,
                    overflown_number_count,
                );

                app.response_body.horizontal_scroll_state =
                    app.response_body.horizontal_scroll_state.position(position);
            }

            _ => {}
        },
        ActiveBlock::RequestBody => match item {
            Some(item) => {
                let lines = &item.pretty_request_body.as_ref().unwrap();

                let longest = lines
                    .lines()
                    .into_iter()
                    .fold(0, |longest: u16, lines: &str| {
                        let len = lines.len() as u16;

                        len.max(longest)
                    });

                let overflown_number_count = longest
                    - metadata.response_body_rectangle_width
                    - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE as u16;

                if app.request_body.horizontal_offset != 0 {
                    app.request_body.horizontal_offset =
                        app.request_body.horizontal_offset.saturating_sub(1);
                }

                app.request_body.horizontal_scroll_state =
                    app.request_body.horizontal_scroll_state.position(
                        {
                            (longest / overflown_number_count)
                                * app.request_body.horizontal_offset as u16
                        }
                        .try_into()
                        .unwrap(),
                    );
            }

            _ => {}
        },
        _ => {}
    }
}

pub fn handle_right(app: &mut App, _key: KeyEvent, metadata: HandlerMetadata) {
    match &app.active_block {
        ActiveBlock::ResponseBody => match get_currently_selected_request(&app) {
            Some(item) => {
                let lines = &item.pretty_response_body.as_ref().unwrap();

                let longest = lines
                    .lines()
                    .into_iter()
                    .fold(0, |longest: u16, lines: &str| {
                        let len = lines.len() as u16;

                        len.max(longest)
                    });

                let overflown_number_count = longest
                    - metadata.response_body_rectangle_width
                    - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE as u16;

                if metadata.response_body_rectangle_width
                    - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE as u16
                    + (app.response_body.horizontal_offset as u16)
                    < longest
                {
                    app.response_body.horizontal_offset += 1;
                }

                let position = calculate_scrollbar_position(
                    longest,
                    app.response_body.horizontal_offset,
                    overflown_number_count,
                );

                app.response_body.horizontal_scroll_state = app
                    .response_body
                    .horizontal_scroll_state
                    .position(position.try_into().unwrap());

                // println!("poss {}", app.response_body.horizontal_scroll_state.);
            }
            _ => {}
        },
        ActiveBlock::RequestBody => match get_currently_selected_request(&app) {
            Some(item) => {
                let lines = &item.pretty_request_body.as_ref().unwrap();

                let longest = lines
                    .lines()
                    .into_iter()
                    .fold(0, |longest: u16, lines: &str| {
                        let len = lines.len() as u16;

                        len.max(longest)
                    });

                let overflown_number_count = longest
                    - metadata.response_body_rectangle_width
                    - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE as u16;

                if metadata.response_body_rectangle_width
                    - RESPONSE_BODY_UNUSABLE_HORIZONTAL_SPACE as u16
                    + (app.request_body.horizontal_offset as u16)
                    < longest
                {
                    app.request_body.horizontal_offset =
                        app.request_body.horizontal_offset.saturating_add(1);
                }

                app.request_body.horizontal_scroll_state =
                    app.request_body.horizontal_scroll_state.position(
                        {
                            (longest / overflown_number_count)
                                * app.request_body.horizontal_offset as u16
                        }
                        .try_into()
                        .unwrap(),
                    );
            }
            _ => {}
        },
        _ => {}
    };
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
                  if app.search_query.is_empty() {
                    app.active_block = ActiveBlock::NetworkRequests;
                  }
              },
              KeyCode::Enter | KeyCode::Esc => app.active_block = ActiveBlock::NetworkRequests,
              KeyCode::Char(c) => app.search_query.push(c),
              _ => app.active_block = ActiveBlock::NetworkRequests,
            }
        }
        _ => {
            app.search_query.clear();
            app.active_block = ActiveBlock::SearchQuery;
        },
    }
}

pub fn handle_tab(app: &mut App, _key: KeyEvent) {
    match app.active_block {
        ActiveBlock::NetworkRequests => app.active_block = ActiveBlock::RequestSummary,
        ActiveBlock::RequestSummary => app.active_block = ActiveBlock::RequestDetails,
        ActiveBlock::RequestDetails => app.active_block = ActiveBlock::RequestBody,
        ActiveBlock::RequestBody => app.active_block = ActiveBlock::ResponseDetails,
        ActiveBlock::ResponseDetails => app.active_block = ActiveBlock::ResponseBody,
        ActiveBlock::ResponseBody => app.active_block = ActiveBlock::NetworkRequests,
        _ => {}
    }
}

pub fn handle_back_tab(app: &mut App, _key: KeyEvent) {
    match app.active_block {
        ActiveBlock::NetworkRequests => app.active_block = ActiveBlock::ResponseBody,
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
    match get_currently_selected_request(&app) {
        Some(request) => match app.active_block {
            ActiveBlock::NetworkRequests => {
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
            ActiveBlock::ResponseBody => match &request.response_body {
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
        },
        None => {}
    };
}
