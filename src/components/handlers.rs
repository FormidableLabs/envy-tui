use crate::app::{
    Action, ActiveBlock, DetailsPane, FilterScreen, MethodFilter, SortScreen, SourceFilter,
    StatusFilter,
};
use crate::components::home::Home;
use crate::consts::{
    NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE, REQUEST_HEADERS_UNUSABLE_VERTICAL_SPACE,
    RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE, RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE,
};
use crate::parser::{generate_curl_command, pretty_parse_body};
use crate::render::get_services_from_traces;
use crate::services::websocket::Trace;
use crate::utils::{
    calculate_scrollbar_position, get_content_length, get_currently_selected_trace,
    get_rendered_items, set_content_length,
};
use crossterm::event::{KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

    app.response_headers_list.reset();

    app.request_details.offset = 0;

    app.request_details.scroll_state = app.request_details.scroll_state.position(0);

    app.request_headers_list.reset();
}

pub fn handle_debug(app: &mut Home) -> Option<Action> {
    let current_block = app.active_block;

    app.previous_blocks.push(current_block);

    app.active_block = ActiveBlock::Debug;

    None
}

pub fn handle_help(app: &mut Home) -> Option<Action> {
    let current_block = app.active_block;

    app.previous_blocks.push(current_block);

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
            ActiveBlock::Details => {
                app.active_block = ActiveBlock::Traces;

                None
            }
            _ => None,
        },
        _ => match (app.active_block, app.details_block) {
            (ActiveBlock::Filter(FilterScreen::Main), _) => {
                match app.filter_source_index.checked_sub(1) {
                    Some(v) => {
                        app.filter_source_index = v;

                        None
                    }
                    _ => None,
                }
            }
            (ActiveBlock::Filter(FilterScreen::Actions), _) => {
                app.filter_actions.previous();

                None
            }
            (ActiveBlock::Filter(_), _) => match app.filter_value_index.checked_sub(1) {
                Some(v) => {
                    app.filter_value_index = v;

                    None
                }
                _ => None,
            },
            (ActiveBlock::Sort(SortScreen::Source), _) => {
                app.sort_sources.previous();

                None
            }
            (ActiveBlock::Sort(SortScreen::Direction), _) => {
                app.sort_directions.previous();

                None
            }
            (ActiveBlock::Sort(SortScreen::Actions), _) => {
                app.sort_actions.previous();

                None
            }
            (ActiveBlock::Traces, _) => {
                if app.main.index > 0 {
                    app.main.index -= 1;

                    if app.main.index < app.main.offset {
                        app.main.offset -= 1;
                    }

                    Some(Action::SelectTrace(get_currently_selected_trace(app)))
                } else {
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

                    app.query_params_list.reset();

                    Some(Action::SelectTrace(get_currently_selected_trace(app)))
                }
            }
            (ActiveBlock::Details, DetailsPane::RequestDetails) => {
                app.request_details_list.previous();

                None
            }
            (ActiveBlock::Details, DetailsPane::QueryParams) => {
                app.query_params_list.previous();

                None
            }
            (ActiveBlock::Details, DetailsPane::RequestHeaders) => {
                app.request_headers_list.previous();

                None
            }
            (ActiveBlock::Details, DetailsPane::ResponseDetails) => {
                app.response_details_list.previous();

                None
            }
            (ActiveBlock::Details, DetailsPane::ResponseHeaders) => {
                app.response_headers_list.previous();

                None
            }
            (ActiveBlock::Details, DetailsPane::Timing) => {
                app.timing_list.previous();

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

    app.query_params_list.reset();

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
            ActiveBlock::Details => {
                app.active_block = ActiveBlock::Traces;

                None
            }
            _ => None,
        },
        _ => match (app.active_block, app.details_block) {
            (ActiveBlock::Filter(FilterScreen::Method), _) => {
                if app.filter_value_index + 1 < app.selected_filters.method.len() {
                    app.filter_value_index += 1;
                }

                None
            }
            (ActiveBlock::Filter(FilterScreen::Source), _) => {
                if app.filter_value_index + 1 < get_services_from_traces(app).len() + 1 {
                    app.filter_value_index += 1;
                }

                None
            }
            (ActiveBlock::Filter(FilterScreen::Main), _) => {
                if app.filter_source_index + 1 < 3 {
                    app.filter_source_index += 1;
                }

                None
            }
            (ActiveBlock::Filter(FilterScreen::Status), _) => {
                if app.filter_value_index + 1 < app.selected_filters.status.len() {
                    app.filter_value_index += 1;
                }

                None
            }
            (ActiveBlock::Filter(FilterScreen::Actions), _) => {
                app.filter_actions.next();

                None
            }
            (ActiveBlock::Sort(SortScreen::Source), _) => {
                app.sort_sources.next();

                None
            }
            (ActiveBlock::Sort(SortScreen::Direction), _) => {
                app.sort_directions.next();

                None
            }
            (ActiveBlock::Sort(SortScreen::Actions), _) => {
                app.sort_actions.next();

                None
            }
            (ActiveBlock::Traces, _) => {
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

                    app.main.scroll_state = app.main.scroll_state.position(position.into());
                }

                app.query_params_list.reset();

                Some(Action::SelectTrace(get_currently_selected_trace(app)))
            }
            (ActiveBlock::Details, DetailsPane::RequestDetails) => {
                app.request_details_list.next();

                None
            }
            (ActiveBlock::Details, DetailsPane::QueryParams) => {
                app.query_params_list.next();

                None
            }
            (ActiveBlock::Details, DetailsPane::RequestHeaders) => {
                app.request_headers_list.next();

                None
            }
            (ActiveBlock::Details, DetailsPane::ResponseDetails) => {
                app.response_details_list.next();

                None
            }
            (ActiveBlock::Details, DetailsPane::ResponseHeaders) => {
                app.response_headers_list.next();

                None
            }
            (ActiveBlock::Details, DetailsPane::Timing) => {
                app.timing_list.next();

                None
            }
            _ => None,
        },
    }
}

pub fn handle_enter(app: &mut Home) -> Option<Action> {
    if app.active_block == ActiveBlock::Traces {
        app.active_block = ActiveBlock::Details;
        None
    } else if app.active_block == ActiveBlock::Details {
        match app.details_block {
            DetailsPane::RequestDetails => app.request_details_list.action(),
            DetailsPane::QueryParams => app.query_params_list.action(),
            DetailsPane::RequestHeaders => app.request_headers_list.action(),
            DetailsPane::ResponseDetails => app.response_details_list.action(),
            DetailsPane::ResponseHeaders => app.response_headers_list.action(),
            DetailsPane::Timing => app.timing_list.action(),
        }
    } else {
        None
    }
}

pub fn handle_esc(app: &mut Home) -> Option<Action> {
    app.active_block = ActiveBlock::Traces;

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
    app.active_block = ActiveBlock::Traces;

    None
}

pub fn handle_tab(app: &mut Home) -> Option<Action> {
    if app.active_block == ActiveBlock::Traces {
        return select_active_details_block(app);
    }

    if app.active_block == ActiveBlock::Details {
        return select_next_details_block(app);
    }

    let next_block = match app.active_block {
        ActiveBlock::Traces => ActiveBlock::Details,
        ActiveBlock::Details => ActiveBlock::ResponseBody,
        ActiveBlock::ResponseBody => ActiveBlock::RequestBody,
        ActiveBlock::RequestBody => ActiveBlock::Traces,
        ActiveBlock::Filter(screen) => match screen {
            FilterScreen::Main => ActiveBlock::Filter(FilterScreen::Actions),
            FilterScreen::Source => ActiveBlock::Filter(FilterScreen::Actions),
            FilterScreen::Method => ActiveBlock::Filter(FilterScreen::Actions),
            FilterScreen::Status => ActiveBlock::Filter(FilterScreen::Actions),
            FilterScreen::Actions => ActiveBlock::Filter(FilterScreen::Main),
        },
        ActiveBlock::Sort(screen) => match screen {
            SortScreen::Source => ActiveBlock::Sort(SortScreen::Direction),
            SortScreen::Direction => ActiveBlock::Sort(SortScreen::Actions),
            SortScreen::Actions => ActiveBlock::Sort(SortScreen::Source),
        },
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
    if app.active_block == ActiveBlock::Details {
        return select_previous_details_block(app);
    }

    let next_block = match app.active_block {
        ActiveBlock::Traces => ActiveBlock::RequestBody,
        ActiveBlock::Details => ActiveBlock::Traces,
        ActiveBlock::RequestBody => ActiveBlock::ResponseBody,
        ActiveBlock::ResponseBody => ActiveBlock::Details,
        ActiveBlock::Filter(screen) => match screen {
            FilterScreen::Main => ActiveBlock::Filter(FilterScreen::Actions),
            FilterScreen::Source => ActiveBlock::Filter(FilterScreen::Main),
            FilterScreen::Method => ActiveBlock::Filter(FilterScreen::Main),
            FilterScreen::Status => ActiveBlock::Filter(FilterScreen::Main),
            FilterScreen::Actions => ActiveBlock::Filter(FilterScreen::Main),
        },
        ActiveBlock::Sort(screen) => match screen {
            SortScreen::Source => ActiveBlock::Sort(SortScreen::Actions),
            SortScreen::Direction => ActiveBlock::Sort(SortScreen::Source),
            SortScreen::Actions => ActiveBlock::Sort(SortScreen::Direction),
        },
        _ => app.active_block,
    };

    if next_block != app.active_block {
        app.active_block = next_block;

        Some(Action::ActivateBlock(next_block))
    } else {
        None
    }
}

pub fn select_active_details_block(app: &mut Home) -> Option<Action> {
    if let Some(active_tab) = app.details_tabs.get(app.details_tab_index) {
        app.details_block = *active_tab;
    } else {
        if let Some(first_tab) = app.details_tabs.first() {
            app.details_block = *first_tab;
        }
    }
    app.active_block = ActiveBlock::Details;

    None
}

pub fn select_next_details_block(app: &mut Home) -> Option<Action> {
    // the tabs are selected, so advance to the first pane
    if app.details_tabs.contains(&app.details_block) {
        if let Some(first_pane) = app.details_panes.first() {
            app.details_block = *first_pane;

            return None;
        }
    }

    let mut iter = app.details_panes.iter();

    // advance iterator to the current block
    iter.find(|&&v| app.details_block == v);

    if let Some(next_pane) = iter.next() {
        app.details_block = *next_pane;

        None
    } else {
        app.active_block = ActiveBlock::ResponseBody;

        Some(Action::ActivateBlock(ActiveBlock::ResponseBody))
    }
}

pub fn select_previous_details_block(app: &mut Home) -> Option<Action> {
    if app.details_panes.len() == 0 {
        app.active_block = ActiveBlock::Traces;

        return Some(Action::ActivateBlock(ActiveBlock::Traces));
    }

    if app.details_tabs.contains(&app.details_block) {
        app.active_block = ActiveBlock::Traces;

        return Some(Action::ActivateBlock(ActiveBlock::Traces));
    }

    let mut iter = app.details_panes.iter().rev();

    // advance iterator to the current block
    iter.find(|&&v| app.details_block == v);

    if let Some(next_pane) = iter.next() {
        app.details_block = *next_pane;

        None
    } else {
        app.details_block = *app
            .details_tabs
            .get(app.details_tab_index)
            .unwrap_or(&DetailsPane::RequestDetails);

        None
    }
}

pub fn handle_details_tab_next(app: &mut Home) -> Option<Action> {
    if app.details_tab_index == app.details_tabs.len() - 1 {
        app.details_tab_index = 0;
    } else {
        app.details_tab_index += 1;
    }

    app.details_block = *app
        .details_tabs
        .get(app.details_tab_index)
        .unwrap_or(&DetailsPane::RequestDetails);

    None
}

pub fn handle_details_tab_prev(app: &mut Home) -> Option<Action> {
    if app.details_tab_index == 0 {
        app.details_tab_index = app.details_tabs.len() - 1;
    } else {
        app.details_tab_index -= 1;
    }

    app.details_block = *app
        .details_tabs
        .get(app.details_tab_index)
        .unwrap_or(&DetailsPane::RequestDetails);

    None
}

pub fn handle_yank(app: &mut Home, sender: Option<UnboundedSender<Action>>) -> Option<Action> {
    if let Some(trace) = app.selected_trace.clone() {
        match app.active_block {
            ActiveBlock::Traces => {
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
            ActiveBlock::ResponseBody => match trace.http.unwrap_or_default().response_body {
                Some(body) => {
                    match clippers::Clipboard::get().write_text(pretty_parse_body(&body).unwrap()) {
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

    None
}

pub fn handle_go_to_end(app: &mut Home, additional_metadata: HandlerMetadata) -> Option<Action> {
    match app.active_block {
        ActiveBlock::Traces => {
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

            Some(Action::SelectTrace(get_currently_selected_trace(app)))
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

            None
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

            None
        }
        ActiveBlock::Details => match app.details_block {
            DetailsPane::RequestDetails => {
                if let Some(item) = app.selected_trace.clone() {
                    if item.http.clone().unwrap_or_default().duration.is_some() {
                        let item_length = item.http.unwrap_or_default().request_headers.len();

                        let usable_height = additional_metadata
                            .request_body_rectangle_height
                            .checked_sub(REQUEST_HEADERS_UNUSABLE_VERTICAL_SPACE as u16)
                            .unwrap_or_default();

                        let requires_scrollbar = item_length as u16 >= usable_height;

                        app.request_headers_list.previous();

                        if requires_scrollbar {
                            let current_index_hit_viewport_end =
                                app.request_headers_list.scroll_state.offset() >= {
                                    usable_height as usize
                                };

                            let offset_does_not_intersects_bottom_of_rect =
                                (app.request_details.offset as u16 + usable_height)
                                    < item_length as u16;

                            if current_index_hit_viewport_end
                                && offset_does_not_intersects_bottom_of_rect
                            {
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

                None
            }
            DetailsPane::ResponseDetails => {
                if let Some(item) = &app.selected_trace {
                    if item.http.clone().unwrap_or_default().duration.is_some() {
                        let item_length =
                            item.http.clone().unwrap_or_default().response_headers.len();

                        let usable_height = additional_metadata.response_body_rectangle_height
                            - RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE as u16;

                        let requires_scrollbar = item_length as u16 >= usable_height;

                        app.response_headers_list.previous();

                        if requires_scrollbar {
                            let current_index_hit_viewport_end =
                                app.response_headers_list.scroll_state.offset() >= {
                                    usable_height as usize
                                };

                            let offset_does_not_intersects_bottom_of_rect =
                                (app.response_details.offset as u16 + usable_height)
                                    < item_length as u16;

                            if current_index_hit_viewport_end
                                && offset_does_not_intersects_bottom_of_rect
                            {
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

                None
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn handle_go_to_start(app: &mut Home) -> Option<Action> {
    match app.active_block {
        ActiveBlock::Traces => {
            app.main.index = 0;

            app.main.offset = 0;

            app.main.scroll_state = app.main.scroll_state.position(0);

            reset_request_and_response_body_ui_state(app);

            return Some(Action::SelectTrace(get_currently_selected_trace(app)));
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
        ActiveBlock::Details => match app.details_block {
            DetailsPane::RequestDetails => {
                app.request_details.offset = 0;
                app.request_headers_list.reset();

                app.request_details.scroll_state = app.request_details.scroll_state.position(0);

                None
            }
            DetailsPane::ResponseDetails => {
                let c = get_content_length(app);

                if c.response_headers.is_some() {
                    app.response_details.offset = 0;
                    app.response_headers_list.reset();

                    app.response_details.scroll_state =
                        app.response_details.scroll_state.position(0);
                }

                None
            }
            _ => None,
        },
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

pub fn handle_select(app: &mut Home) -> Option<Action> {
    match app.active_block {
        ActiveBlock::Sort(SortScreen::Source) => app.sort_sources.action(),
        ActiveBlock::Sort(SortScreen::Direction) => app.sort_directions.action(),
        ActiveBlock::Sort(SortScreen::Actions) => app.sort_actions.action(),
        ActiveBlock::Filter(FilterScreen::Actions) => app.filter_actions.action(),
        ActiveBlock::Filter(FilterScreen::Main) => {
            let blocks = vec!["method", "source", "status"];

            let maybe_selected_filter = blocks.iter().nth(app.filter_source_index).cloned();

            if let Some(selected_filter) = maybe_selected_filter {
                let screen = match selected_filter {
                    "method" => FilterScreen::Method,
                    "source" => FilterScreen::Source,
                    "status" => FilterScreen::Status,
                    _ => FilterScreen::default(),
                };

                app.filter_value_screen = screen;
                app.filter_value_index = 0;
                app.active_block = ActiveBlock::Filter(screen);
            };

            None
        }

        ActiveBlock::Filter(FilterScreen::Status) => {
            let current_service = app
                .selected_filters
                .status
                .iter()
                .map(|(key, _item)| key)
                .nth(app.filter_value_index);

            if current_service.is_none() {
                return None;
            }

            if let Some(filter) = current_service {
                if let Some(d) = app.selected_filters.status.get(filter) {
                    app.selected_filters.status.insert(
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

            None
        }
        ActiveBlock::Filter(FilterScreen::Method) => {
            let current_service = app
                .selected_filters
                .method
                .iter()
                .map(|(a, _item)| a)
                .nth(app.filter_value_index);

            if current_service.is_none() {
                return None;
            }

            if let Some(filter) = current_service {
                if let Some(d) = app.selected_filters.method.get(filter) {
                    app.selected_filters.method.insert(
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

            None
        }
        ActiveBlock::Filter(FilterScreen::Source) => {
            let mut services = get_services_from_traces(app);

            let mut a: Vec<String> = vec!["All".to_string()];

            a.append(&mut services);

            services = a;

            let selected_filter = services.iter().nth(app.filter_value_index).cloned();

            if let Some(filter) = selected_filter {
                match filter.as_str() {
                    "All" => app.selected_filters.source = SourceFilter::All,
                    source => match &app.selected_filters.source {
                        SourceFilter::All => {
                            let mut set = HashSet::new();

                            set.insert(source.to_string());

                            app.selected_filters.source = SourceFilter::Applied(set)
                        }
                        SourceFilter::Applied(applied_sources) => {
                            if applied_sources.contains(&source.to_string()) {
                                let mut set = applied_sources.clone();

                                set.remove(source);

                                app.selected_filters.source = SourceFilter::Applied(set)
                            } else {
                                let mut set = applied_sources.clone();

                                set.insert(source.to_string());

                                if set.len() == get_services_from_traces(app).len() {
                                    app.selected_filters.source = SourceFilter::All
                                } else {
                                    app.selected_filters.source = SourceFilter::Applied(set)
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

            app.main.scroll_state = app.main.scroll_state.content_length(length.into());

            None
        }
        _ => None,
    }
}
