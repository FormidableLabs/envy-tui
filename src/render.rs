use crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashSet;
use std::io::Stdout;
use std::ops::Deref;
use std::usize;

use http::{HeaderName, HeaderValue};
use ratatui::prelude::{Alignment, Constraint, CrosstermBackend, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, List, ListItem, Padding, Paragraph, Row, Scrollbar,
    ScrollbarOrientation, Table, Tabs,
};
use ratatui::Frame;

use crate::app::{Action, ActiveBlock, RequestDetailsPane, UIState};
use crate::components::home::{FilterSource, Home};
use crate::consts::{
    NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE, REQUEST_HEADERS_UNUSABLE_VERTICAL_SPACE,
    RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE, RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE,
};
use crate::services::websocket::HTTPTrace;
use crate::utils::{
    get_currently_selected_trace, get_rendered_items, parse_query_params, truncate, TraceSort,
};

#[derive(Clone, Copy, PartialEq, Debug, Hash, Eq)]
pub enum RowStyle {
    Default,
    Selected,
    Inactive,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum HeaderType {
    Request,
    Response,
}

pub fn render_body(
    pretty_body: String,
    ui_state: &mut UIState,
    active_block: ActiveBlock,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    block: ActiveBlock,
) {
    let mut longest_line_length = 0;

    let lines = pretty_body
        .lines()
        .map(|lines| {
            let len = lines.len();

            longest_line_length = len.max(longest_line_length);

            Line::from(lines)
        })
        .collect::<Vec<_>>();

    let number_of_lines = lines.len();

    let has_overflown_x_axis = longest_line_length as u16 > area.width;

    let has_overflown_y_axis =
        number_of_lines as u16 > area.height - RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE as u16;

    let body_to_render = Paragraph::new(lines)
        .style(
            Style::default()
                .fg(if active_block == block {
                    Color::White
                } else {
                    Color::DarkGray
                })
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(if active_block == block {
                    Color::White
                } else {
                    Color::DarkGray
                }))
                .title(format!(
                    "{} body",
                    if block == ActiveBlock::RequestBody {
                        "Request"
                    } else {
                        "Response"
                    },
                ))
                .border_type(BorderType::Plain),
        )
        .scroll((ui_state.offset as u16, ui_state.horizontal_offset as u16));

    frame.render_widget(body_to_render, area);

    if has_overflown_y_axis {
        let vertical_scroll = Scrollbar::new(ScrollbarOrientation::VerticalRight);

        frame.render_stateful_widget(
            vertical_scroll,
            area.inner(&Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut ui_state.scroll_state,
        );
    }

    if has_overflown_x_axis {
        let horizontal_scroll = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
            .begin_symbol(Some("<-"))
            .end_symbol(Some("->"));

        frame.render_stateful_widget(
            horizontal_scroll,
            area.inner(&Margin {
                vertical: 0,
                horizontal: 1,
            }),
            &mut ui_state.horizontal_scroll_state,
        );
    }
}

pub fn get_currently_selected_http_trace(app: &Home) -> Option<HTTPTrace> {
    let trace = get_currently_selected_trace(&app);

    if trace.is_none() {
        return None::<HTTPTrace>;
    }

    let trace = trace.unwrap();

    trace.http
}

pub fn render_response_body(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let trace = get_currently_selected_trace(app);

    let matching_plugin = app.plugins.iter().find(|plugin| {
        if let Some(trace) = &trace {
            plugin.is_match(trace)
        } else {
            false
        }
    });

    if let Some(plugin) = matching_plugin {
        return plugin.render_request_body(app, &trace.unwrap(), frame, area);
    }

    match get_currently_selected_http_trace(app) {
        Some(request) => match &request.pretty_response_body {
            Some(pretty_json) => {
                render_body(
                    pretty_json.to_string(),
                    &mut app.response_body.clone(),
                    app.active_block,
                    frame,
                    area,
                    ActiveBlock::ResponseBody,
                );
            }
            _ => {
                let copy = if request.duration.is_some() {
                    "This trace does not have a response body."
                } else {
                    "Loading..."
                };

                let body_to_render = Paragraph::new(copy)
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .fg(if app.active_block == ActiveBlock::ResponseBody {
                                Color::White
                            } else {
                                Color::DarkGray
                            })
                            .add_modifier(Modifier::BOLD),
                    )
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .style(Style::default().fg(
                                if app.active_block == ActiveBlock::ResponseBody {
                                    Color::White
                                } else {
                                    Color::DarkGray
                                },
                            ))
                            .title("Request body")
                            .border_type(BorderType::Plain),
                    );
                frame.render_widget(body_to_render, area)
            }
        },
        _ => {}
    }
}

pub fn get_row_style(row_style: RowStyle) -> Style {
    let default_style = Style::default().fg(Color::White);

    let selected_style = Style::default().fg(Color::Black).bg(Color::LightRed);

    let inactive_stlye = Style::default().fg(Color::Black).bg(Color::Gray);

    match row_style {
        RowStyle::Default => default_style,
        RowStyle::Inactive => inactive_stlye,
        RowStyle::Selected => selected_style,
    }
}

pub fn get_border_style(active: bool) -> Style {
    if active {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

pub fn get_text_style(active: bool) -> Style {
    if active {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn render_headers(
    app: &Home,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    header_type: HeaderType,
) {
    let active_block = app.active_block;

    let rows = match get_currently_selected_http_trace(app) {
        Some(item) => {
            let headers = if header_type == HeaderType::Request {
                &item.request_headers
            } else {
                &item.response_headers
            };

            let offset = if header_type == HeaderType::Request {
                app.request_details.offset
            } else {
                app.response_details.offset
            };

            let index = if header_type == HeaderType::Request {
                app.selected_request_header_index
            } else {
                app.selected_response_header_index
            };

            let mut cloned = headers.iter().collect::<Vec<(&HeaderName, &HeaderValue)>>();

            cloned.sort_by(|a, b| {
                let (name_a, _) = a;
                let (name_b, _) = b;

                name_a.to_string().cmp(&name_b.to_string())
            });

            let current_header_selected = match cloned.iter().nth(index) {
                Some((name, _)) => name.deref().to_string(),
                None => "".to_string(),
            };

            let rows = cloned
                .iter()
                .skip(offset)
                .map(|(name, value)| {
                    let header_name = name.as_str();

                    let header_value = match value.to_str() {
                        Ok(v) => v,
                        _ => "Unknown header value",
                    };

                    let is_active_header = header_name == current_header_selected;

                    Row::new(vec![String::from(header_name), String::from(header_value)]).style(
                        match (is_active_header, active_block, header_type) {
                            (true, ActiveBlock::RequestDetails, HeaderType::Request) => {
                                get_row_style(RowStyle::Selected)
                            }
                            (true, ActiveBlock::ResponseDetails, HeaderType::Response) => {
                                get_row_style(RowStyle::Selected)
                            }
                            (true, _, _) => get_row_style(RowStyle::Inactive),
                            (_, _, _) => get_row_style(RowStyle::Default),
                        },
                    )
                })
                .collect();

            rows
        }
        None => vec![Row::new(vec!["No headers found."])],
    };

    let table = Table::new(rows)
        // You can set the style of the entire Table.
        .style(Style::default().fg(Color::White))
        // It has an optional header, which is simply a Row always visible at the top.
        .header(
            Row::new(vec!["Header name", "Header value"])
                .style(Style::default().fg(Color::Yellow))
                // If you want some space between the header and the rest of the rows, you can always
                // specify some margin at the bottom.
                .bottom_margin(1),
        )
        .widths(&[Constraint::Percentage(40), Constraint::Percentage(60)])
        // ...and they can be separated by a fixed spacing.
        // .column_spacing(1)
        // If you wish to highlight a row in any specific way when it is selected...
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        // ...and potentially show a symbol in front of the selection.
        .highlight_symbol(">>");

    frame.render_widget(table, area);
}

pub fn render_request_block(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let active_block = app.active_block;

    match get_currently_selected_http_trace(app) {
        Some(maybe_selected_item) => {
            let uri = maybe_selected_item.uri.clone();

            let raw_params = parse_query_params(uri);

            let mut cloned = raw_params;

            cloned.sort_by(|a, b| {
                let (name_a, _) = a;
                let (name_b, _) = b;

                name_a.cmp(name_b)
            });

            let current_param_selected = cloned.get(app.selected_params_index);

            let rows = cloned
                .iter()
                .map(|param| {
                    let (name, value) = param;
                    let cloned_name = name.deref().clone();
                    let cloned_value = value.deref().clone();

                    let is_selected = match current_param_selected {
                        Some(v) => {
                            let (current_name, _) = v;

                            current_name.deref() == name
                        }
                        None => false,
                    };

                    Row::new(vec![cloned_name, cloned_value]).style(
                        match (is_selected, active_block) {
                            (true, ActiveBlock::RequestDetails) => {
                                get_row_style(RowStyle::Selected)
                            }
                            (true, _) => get_row_style(RowStyle::Inactive),
                            (_, _) => get_row_style(RowStyle::Default),
                        },
                    )
                })
                .collect::<Vec<Row>>();

            let table = Table::new(rows)
                // You can set the style of the entire Table.
                .style(Style::default().fg(Color::White))
                // It has an optional header, which is simply a Row always visible at the top.
                .header(
                    Row::new(vec!["Query name", "Query Param value"])
                        .style(Style::default().fg(Color::Yellow))
                        // If you want some space between the header and the rest of the rows, you can always
                        // specify some margin at the bottom.
                        .bottom_margin(1),
                )
                .widths(&[
                    Constraint::Percentage(10),
                    Constraint::Percentage(70),
                    Constraint::Length(20),
                ])
                // ...and they can be separated by a fixed spacing.
                // .column_spacing(1)
                // If you wish to highlight a row in any specific way when it is selected...
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                // ...and potentially show a symbol in front of the selection.
                //
                //
                .highlight_symbol(">>");

            let tabs = Tabs::new(vec!["Request Header", "Request Params"])
                .block(
                    Block::default()
                        .borders(Borders::BOTTOM)
                        .style(Style::default().fg(
                            if active_block == ActiveBlock::RequestDetails {
                                Color::White
                            } else {
                                Color::DarkGray
                            },
                        ))
                        .border_type(BorderType::Plain),
                )
                .select(match app.request_details_block {
                    RequestDetailsPane::Headers => 0,
                    RequestDetailsPane::Query => 1,
                })
                .highlight_style(Style::default().fg(Color::LightMagenta));

            let inner_layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Max(2), Constraint::Min(1)].as_ref())
                .split(area);

            let main = Block::default()
                .title("Request details")
                .title(
                    Title::from(format!(
                        "{} of {}",
                        app.selected_request_header_index + 1,
                        maybe_selected_item.request_headers.len()
                    ))
                    .position(Position::Bottom)
                    .alignment(Alignment::Right),
                )
                .style(
                    Style::default().fg(if active_block == ActiveBlock::RequestDetails {
                        Color::White
                    } else {
                        Color::DarkGray
                    }),
                )
                .border_type(BorderType::Plain)
                .borders(Borders::ALL);

            frame.render_widget(main, area);
            frame.render_widget(tabs, inner_layout[0]);

            match app.request_details_block {
                RequestDetailsPane::Query => {
                    frame.render_widget(table, inner_layout[1]);
                }
                RequestDetailsPane::Headers => {
                    render_headers(app, frame, inner_layout[1], HeaderType::Request);

                    let vertical_scroll = Scrollbar::new(ScrollbarOrientation::VerticalRight);

                    let trace = get_currently_selected_http_trace(app);

                    let content_length = trace.unwrap().response_headers.len() as u16;

                    let viewport_height =
                        area.height - REQUEST_HEADERS_UNUSABLE_VERTICAL_SPACE as u16;

                    if content_length > viewport_height {
                        frame.render_stateful_widget(
                            vertical_scroll,
                            area.inner(&Margin {
                                horizontal: 0,
                                vertical: 2,
                            }),
                            &mut app.request_details.scroll_state.clone(),
                        );
                    }
                }
            }
        }
        None => (),
    };
}

pub fn render_request_body(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    match get_currently_selected_http_trace(app) {
        Some(request) => match &request.pretty_request_body {
            Some(pretty_json) => {
                render_body(
                    pretty_json.to_string(),
                    &mut app.request_body.clone(),
                    app.active_block,
                    frame,
                    area,
                    ActiveBlock::RequestBody,
                );
            }
            _ => {
                let body_to_render = Paragraph::new("This trace does not have a request body.")
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .fg(if app.active_block == ActiveBlock::RequestBody {
                                Color::White
                            } else {
                                Color::DarkGray
                            })
                            .add_modifier(Modifier::BOLD),
                    )
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .style(Style::default().fg(
                                if app.active_block == ActiveBlock::RequestBody {
                                    Color::White
                                } else {
                                    Color::DarkGray
                                },
                            ))
                            .title("Request body")
                            .border_type(BorderType::Plain),
                    );
                frame.render_widget(body_to_render, area)
            }
        },
        _ => {}
    }
}

pub fn render_response_block(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let uri = match get_currently_selected_http_trace(app) {
        Some(item) => item.uri.clone(),
        None => String::from("Could not find request."),
    };

    let raw_params = parse_query_params(uri);

    let is_progress = match get_currently_selected_http_trace(app) {
        Some(v) => v.duration.is_none(),
        None => true,
    };

    if is_progress {
        let status_bar = Paragraph::new("Loading...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(get_border_style(
                        app.active_block == ActiveBlock::ResponseDetails,
                    ))
                    .title("Response details")
                    .border_type(BorderType::Plain),
            );

        frame.render_widget(status_bar, area);
    } else {
        let mut cloned = raw_params;

        cloned.sort_by(|a, b| {
            let (name_a, _) = a;
            let (name_b, _) = b;

            name_a.cmp(name_b)
        });

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Max(2), Constraint::Min(1)].as_ref())
            .split(area);

        let main = Block::default()
            .title("Response details")
            .title(
                Title::from(format!(
                    "{} of {}",
                    app.selected_response_header_index + 1,
                    get_currently_selected_http_trace(app)
                        .unwrap()
                        .response_headers
                        .len()
                ))
                .position(Position::Bottom)
                .alignment(Alignment::Right),
            )
            .style(
                Style::default().fg(if app.active_block == ActiveBlock::ResponseDetails {
                    Color::White
                } else {
                    Color::DarkGray
                }),
            )
            .border_type(BorderType::Plain)
            .borders(Borders::ALL);

        let tabs = Tabs::new(vec!["Response Header"])
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .style(Style::default().fg(Color::White))
                    .border_type(BorderType::Plain),
            )
            .select(0)
            .highlight_style(Style::default().fg(Color::LightMagenta));

        frame.render_widget(main, area);
        frame.render_widget(tabs, inner_layout[0]);

        render_headers(app, frame, inner_layout[1], HeaderType::Response);

        let vertical_scroll = Scrollbar::new(ScrollbarOrientation::VerticalRight);

        let trace = get_currently_selected_http_trace(app);

        let content_length = trace.unwrap().response_headers.len();

        if content_length > area.height as usize - RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE {
            frame.render_stateful_widget(
                vertical_scroll,
                area.inner(&Margin {
                    horizontal: 0,
                    vertical: 2,
                }),
                &mut app.response_details.scroll_state.clone(),
            );
        }
    }
}

pub fn render_traces(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let height = area.height;

    let effective_height = height - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16;

    let active_block = app.active_block;

    let items_as_vector = get_rendered_items(app);

    let number_of_lines = items_as_vector.len();

    let selected_item = items_as_vector.get(app.main.index);

    let method_len = app.method_filters.iter().fold(0, |sum, (_key, item)| {
        let mut result = sum;

        if item.selected {
            result += 1;
        }

        return result;
    });

    let status_len = app.status_filters.iter().fold(0, |sum, (_key, item)| {
        let mut result = sum;

        if item.selected {
            result += 1;
        }

        return result;
    });

    let filter_message = match status_len + method_len {
        0 => String::from("No filters selected"),
        _ => {
            let mut filters_text = format!("Active filter(s): ");

            app.method_filters.iter().for_each(|(_a, filter_method)| {
                if filter_method.selected {
                    filters_text.push_str((format!(" {} (Method)", filter_method.name)).as_str());
                }
            });

            app.status_filters.iter().for_each(|(_a, filter_status)| {
                if filter_status.selected {
                    filters_text.push_str((format!(" {} (Status)", filter_status.name)).as_str());
                }
            });

            filters_text
        }
    };

    let sort_message = format!("Active sort: {}", &app.order);

    let title = format!("Traces - [{}] - [{}]", filter_message, sort_message);

    let converted_rows: Vec<(Vec<String>, bool)> = items_as_vector
        .iter()
        .skip(app.main.offset)
        .take(effective_height.into())
        .map(|request| {
            let uri = truncate(request.http.as_ref().unwrap().uri.clone().as_str(), 60);

            let method = request.http.as_ref().unwrap().method.clone().to_string();

            let status = request.http.as_ref().unwrap().status;
            let duration = request.http.as_ref().unwrap().duration;

            let status = match status {
                Some(v) => v.as_u16().to_string(),
                None => "...".to_string(),
            };

            let duration = match duration {
                Some(v) => {
                    format!("{:.3} s", ((v as f32) / 1000.0))
                }
                None => "...".to_string(),
            };

            let id = request.id.clone();

            let selected = match selected_item {
                Some(item) => item.deref() == request.deref(),
                None => false,
            };

            (vec![method, status, uri, duration, id], selected)
        })
        .collect();

    let styled_rows: Vec<Row> = converted_rows
        .iter()
        .map(|(row, selected)| {
            let str_vec: Vec<&str> = row
                .iter()
                .map(|x| x.as_str())
                .collect::<Vec<&str>>()
                .clone();

            Row::new(str_vec).style(match (*selected, active_block) {
                (true, ActiveBlock::TracesBlock) => get_row_style(RowStyle::Selected),
                (true, _) => get_row_style(RowStyle::Inactive),
                (_, _) => get_row_style(RowStyle::Default),
            })
        })
        .collect();

    let requests = Table::new(styled_rows)
        // You can set the style of the entire Table.
        .style(Style::default().fg(Color::White))
        // It has an optional header, which is simply a Row always visible at the top.
        .header(
            Row::new(vec!["Method", "Status", "Request", "Duration"])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(get_border_style(
                    app.active_block == ActiveBlock::TracesBlock,
                ))
                .title(title)
                .title(
                    Title::from(format!("{} of {}", app.main.index + 1, number_of_lines))
                        .position(Position::Bottom)
                        .alignment(Alignment::Right),
                )
                .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(60),
            Constraint::Length(20),
        ]);

    let vertical_scroll = Scrollbar::new(ScrollbarOrientation::VerticalRight);

    frame.render_widget(requests, area);

    let usable_height = area.height - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16;

    if number_of_lines > usable_height.into() {
        frame.render_stateful_widget(
            vertical_scroll,
            area.inner(&Margin {
                horizontal: 0,
                vertical: 2,
            }),
            &mut app.main.scroll_state.clone(),
        );
    }
}

pub fn render_search(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>) {
    if app.active_block == ActiveBlock::SearchQuery {
        let area = overlay_area(frame.size());
        let widget = Paragraph::new(format!("/{}", &app.search_query))
            .style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Left);

        frame.render_widget(Clear, area);
        frame.render_widget(widget, area);
    }
}

pub fn render_footer(app: &Home, frame: &mut Frame<'_, CrosstermBackend<Stdout>>, area: Rect) {
    let general_status = match app.status_message.clone() {
        Some(text) => text,
        None => "".to_string(),
    };

    let help_text = Paragraph::new("For help, press ?")
        .style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::DarkGray))
                .title("Status Bar")
                .padding(Padding::new(1, 0, 0, 0))
                .border_type(BorderType::Plain),
        );

    let wss_status_message = match app.wss.len() {
        0 => "🟠 Waiting for connection".to_string(),
        1 => "🟢 1 client connected".to_string(),
        v => {
            format!("🟢 {:?} clients connected", v)
        }
    };

    let status_bar = Paragraph::new(format!("{} {}", general_status, wss_status_message))
        .style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Right)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::DarkGray))
                .title("Status Bar")
                .padding(Padding::new(0, 1, 0, 0))
                .border_type(BorderType::Plain),
        );

    frame.render_widget(status_bar, area);

    frame.render_widget(help_text, area);
}

pub fn render_request_summary(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let selected_item = get_currently_selected_trace(app);

    let message = match selected_item {
        Some(item) => item.to_string(),
        None => "No item found".to_string(),
    };

    let status_bar = Paragraph::new(message)
        .style(get_text_style(
            app.active_block == ActiveBlock::RequestSummary,
        ))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(get_border_style(
                    app.active_block == ActiveBlock::RequestSummary,
                ))
                .title("Request Summary")
                .border_type(BorderType::Plain),
        );

    frame.render_widget(status_bar, area);
}

pub fn render_help(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let mut entry_list: Vec<(KeyEvent, Action)> = vec![];
    for (k, v) in app.key_map.iter() {
        entry_list.push((*k, v.clone()));
    }

    let key_mappings: Vec<(String, String)> = entry_list
        .iter()
        .map(|(key_event, action)| {
            let description_str = match action {
                Action::CopyToClipBoard => "Copy selection to OS clipboard",
                Action::FocusOnTraces => "Focus on traces section OR exit current window",
                Action::NavigateUp(_) => "Move up and select an entry one above",
                Action::NavigateDown(_) => "Move down and select entry below",
                Action::NavigateLeft(_) => "Move cursor left",
                Action::NavigateRight(_) => "Move cursor right",
                Action::GoToRight => "Abs cursor right",
                Action::GoToLeft => "Abs cursor left",
                Action::NextSection => "Focus on next section",
                Action::GoToEnd => "Move to bottom of section",
                Action::GoToStart => "Move to top of section",
                Action::PreviousSection => "Focus on previous section",
                Action::Quit => "Quit",
                Action::NewSearch => "Search",
                Action::ExitSearch => "Cancel Search",
                Action::UpdateSearchQuery(_) => "Update Search Query",
                Action::DeleteSearchQuery => "Delete Last Search Char",
                Action::Help => "Open Help Window",
                Action::ToggleDebug => "Toggle Debug Window",
                Action::DeleteItem => "Delete Trace",
                Action::ShowTraceDetails => "Focus On Trace",
                Action::NextPane => "Focus On Next Pane",
                Action::PreviousPane => "Go To Previous Pane",
                Action::StartWebSocketServer => "Start the Collector Server",
                Action::StopWebSocketServer => "Stop the Collector Server",
                _ => "",
            };
            let description = format!("{}:", description_str);

            let mut b = [0; 2];
            let key_code_str = match key_event.code {
                KeyCode::PageUp => "Page Up",
                KeyCode::PageDown => "Page Down",
                KeyCode::Down => "Down arrow",
                KeyCode::Up => "Up arrow",
                KeyCode::Esc => "Esc",
                KeyCode::Tab => "Tab",
                KeyCode::BackTab => "Tab + Shift",
                KeyCode::Char('/') => "/{pattern}[/]<CR>",
                KeyCode::Char(c) => c.encode_utf8(&mut b),
                _ => "Default",
            };
            let key_code = format!(r#""{}""#, key_code_str);

            (description, key_code)
        })
        .collect();

    let mut grouped: Vec<(String, Vec<String>)> =
        key_mappings.iter().fold(vec![], |mut acc, (rk, rv)| {
            for (lk, lv) in acc.iter_mut() {
                if lk.eq(&rk) {
                    lv.push(rv.to_string());
                    return acc;
                }
            }

            acc.push((rk.to_string(), vec![rv.to_string()]));

            acc
        });

    grouped.sort();

    let debug_lines = grouped
        .iter()
        .map(|(description, key_code)| {
            let column_a =
                Cell::from(Line::from(vec![Span::raw(description)]).alignment(Alignment::Right));
            let column_b = Cell::from(key_code.join(", "));

            Row::new(vec![column_a, column_b]).style(get_row_style(RowStyle::Default))
        })
        .collect::<Vec<_>>();

    let list = Table::new(debug_lines)
        .style(get_text_style(true))
        .header(
            Row::new(vec!["Action", "Map"])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(get_border_style(true))
                .title("Key Mappings")
                .border_type(BorderType::Plain),
        )
        .widths(&[Constraint::Percentage(40), Constraint::Percentage(60)])
        .column_spacing(10);

    frame.render_widget(list, area);
}

pub fn render_debug(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    // let debug_lines = app
    //     .logs
    //     .iter()
    //     .map(|item| ListItem::new(Line::from(Span::raw(item))))
    //     .collect::<Vec<_>>();

    let debug_lines = app
        .wss
        .iter()
        .map(|item| ListItem::new(Line::from(Span::raw(item.path.clone()))))
        .collect::<Vec<_>>();

    // TODO: Render different Keybindings that are relevant for the given `active_block`.
    let list = List::new(debug_lines).style(get_text_style(true)).block(
        Block::default()
            .borders(Borders::ALL)
            .style(get_border_style(true))
            .title("Debug logs")
            .border_type(BorderType::Plain),
    );

    frame.render_widget(list, area);
}

/// helper function to create an overlay rect `r`
fn overlay_area(r: Rect) -> Rect {
    let overlay_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100), Constraint::Min(0)].as_ref())
        .split(overlay_layout[1])[0]
}

pub fn get_services_from_traces(app: &Home) -> Vec<String> {
    let services = app
        .items
        .iter()
        .filter(|trace| trace.service_name.is_some())
        .map(|trace| trace.service_name.as_ref().unwrap().clone())
        .collect::<HashSet<_>>();

    let mut services_as_vec = services.iter().cloned().collect::<Vec<String>>();

    services_as_vec.sort();

    services_as_vec.clone()
}

pub fn render_filters_source(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let mut services = get_services_from_traces(app);

    let mut a: Vec<String> = vec!["All".to_string()];

    a.append(&mut services);

    services = a;

    let current_service = services.iter().nth(app.filter_index).cloned();

    let rows = services
        .iter()
        .map(|item| {
            let column_a =
                Cell::from(Line::from(vec![Span::raw(item.clone())]).alignment(Alignment::Left));

            match app.get_filter_source() {
                FilterSource::All => {
                    let column_b = Cell::from(
                        Line::from(vec![Span::raw("[x]".to_string())]).alignment(Alignment::Left),
                    );

                    let row_style = if current_service.is_some()
                        && current_service.clone().unwrap() == item.deref().clone()
                    {
                        RowStyle::Selected
                    } else {
                        RowStyle::Default
                    };

                    let middle = Cell::from(
                        Line::from(vec![Span::raw("Source".to_string())])
                            .alignment(Alignment::Left),
                    );

                    return Row::new(vec![column_b, middle, column_a])
                        .style(get_row_style(row_style));
                }
                FilterSource::Applied(applied) => {
                    // let column_b = if applied.contains(current_service.as_ref().unwrap()) {
                    let column_b = if applied.contains(item) {
                        Cell::from(
                            Line::from(vec![Span::raw("[x]".to_string())])
                                .alignment(Alignment::Left),
                        )
                    } else {
                        Cell::from(
                            Line::from(vec![Span::raw("[ ]".to_string())])
                                .alignment(Alignment::Left),
                        )
                    };

                    let row_style = if current_service.is_some()
                        && current_service.clone().unwrap() == item.deref().clone()
                    {
                        RowStyle::Selected
                    } else {
                        RowStyle::Default
                    };

                    let middle = Cell::from(
                        Line::from(vec![Span::raw("Source".to_string())])
                            .alignment(Alignment::Left),
                    );

                    return Row::new(vec![column_b, middle, column_a])
                        .style(get_row_style(row_style));
                }
            };
        })
        .collect::<Vec<_>>();

    let list = Table::new([rows].concat())
        .style(get_text_style(true))
        .header(
            Row::new(vec!["Selected", "Type", "Value"])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(get_border_style(true))
                .title("[Filters - Sources]")
                .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(60),
        ])
        .column_spacing(10);

    frame.render_widget(list.clone(), area);
}

pub fn render_filters_status(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let current_service = app.status_filters.iter().nth(app.filter_index);

    let rows1 = app
        .status_filters
        .iter()
        .map(|(_a, item)| {
            let column_a = Cell::from(
                Line::from(vec![Span::raw(item.name.clone())]).alignment(Alignment::Left),
            );

            let column_b = if item.selected {
                Cell::from(
                    Line::from(vec![Span::raw("[x]".to_string())]).alignment(Alignment::Left),
                )
            } else {
                Cell::from(
                    Line::from(vec![Span::raw("[ ]".to_string())]).alignment(Alignment::Left),
                )
            };

            let (_key, status_filter) = current_service.clone().unwrap();

            let row_style =
                if current_service.is_some() && status_filter.status == item.name.clone() {
                    RowStyle::Selected
                } else {
                    RowStyle::Default
                };

            let h = Cell::from(
                Line::from(vec![Span::raw("Status".to_string())]).alignment(Alignment::Left),
            );

            Row::new(vec![column_b, h, column_a]).style(get_row_style(row_style))
        })
        .collect::<Vec<_>>();

    let list = Table::new([rows1].concat())
        .style(get_text_style(true))
        .header(
            Row::new(vec!["Selected", "Type", "Value"])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(get_border_style(true))
                .title("[Filters - Status]")
                .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(60),
        ])
        .column_spacing(10);

    frame.render_widget(list.clone(), area);
}

pub fn render_filters_method(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let current_service = app
        .method_filters
        .iter()
        .map(|(a, b)| b.name.clone())
        .nth(app.filter_index);

    let rows1 = app
        .method_filters
        .iter()
        .map(|(a, item)| {
            let column_a = Cell::from(
                Line::from(vec![Span::raw(item.name.clone())]).alignment(Alignment::Left),
            );

            let column_b = if item.selected {
                Cell::from(
                    Line::from(vec![Span::raw("[x]".to_string())]).alignment(Alignment::Left),
                )
            } else {
                Cell::from(
                    Line::from(vec![Span::raw("[ ]".to_string())]).alignment(Alignment::Left),
                )
            };

            let row_style = if current_service.is_some()
                && current_service.clone().unwrap() == item.name.clone()
            {
                RowStyle::Selected
            } else {
                RowStyle::Default
            };

            let h = Cell::from(
                Line::from(vec![Span::raw("Method".to_string())]).alignment(Alignment::Left),
            );

            Row::new(vec![column_b, h, column_a]).style(get_row_style(row_style))
        })
        .collect::<Vec<_>>();

    let list = Table::new([rows1].concat())
        .style(get_text_style(true))
        .header(
            Row::new(vec!["Selected", "Type", "Value"])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(get_border_style(true))
                .title("[Filters - Method]")
                .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(60),
        ])
        .column_spacing(10);

    frame.render_widget(list.clone(), area);
}

pub fn render_filters(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let filter_items = vec!["method", "source", "status"];

    let current_service = filter_items.iter().nth(app.filter_index).cloned();

    let filter_item_rows = filter_items
        .iter()
        .map(|item| {
            let column_a =
                Cell::from(Line::from(vec![Span::raw(item.clone())]).alignment(Alignment::Left));

            let column_b = Cell::from(
                Line::from(vec![Span::raw("[x]".to_string())]).alignment(Alignment::Left),
            );

            let row_style = if current_service.is_some()
                && current_service.clone().unwrap() == item.deref().clone()
            {
                RowStyle::Selected
            } else {
                RowStyle::Default
            };

            let middle = Cell::from(
                Line::from(vec![Span::raw("Method".to_string())]).alignment(Alignment::Left),
            );

            Row::new(vec![column_b, middle, column_a]).style(get_row_style(row_style))
        })
        .collect::<Vec<_>>();

    let list = Table::new([filter_item_rows].concat())
        .style(get_text_style(true))
        .header(
            Row::new(vec!["Selected", "Type", "Value"])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(get_border_style(true))
                .title("[Filters]")
                .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(60),
        ])
        .column_spacing(10);

    frame.render_widget(list.clone(), area);
}

pub fn render_sort(app: &Home, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let filter_items = vec![
        (
            "Method",
            "Asc",
            TraceSort::Method(crate::utils::Ordering::Ascending),
        ),
        (
            "Method",
            "Desc",
            TraceSort::Method(crate::utils::Ordering::Descending),
        ),
        (
            "Source",
            "Asc",
            TraceSort::Source(crate::utils::Ordering::Ascending),
        ),
        (
            "Source",
            "Desc",
            TraceSort::Source(crate::utils::Ordering::Descending),
        ),
        (
            "Status",
            "Asc",
            TraceSort::Status(crate::utils::Ordering::Ascending),
        ),
        (
            "Status",
            "Desc",
            TraceSort::Status(crate::utils::Ordering::Descending),
        ),
        (
            "Timestamp",
            "Asc",
            TraceSort::Timestamp(crate::utils::Ordering::Ascending),
        ),
        (
            "Timestamp",
            "Desc",
            TraceSort::Timestamp(crate::utils::Ordering::Descending),
        ),
        (
            "Duration",
            "Asc",
            TraceSort::Duration(crate::utils::Ordering::Ascending),
        ),
        (
            "Duration",
            "Desc",
            TraceSort::Duration(crate::utils::Ordering::Descending),
        ),
        (
            "Url",
            "Asc",
            TraceSort::Url(crate::utils::Ordering::Ascending),
        ),
        (
            "Url",
            "Desc",
            TraceSort::Url(crate::utils::Ordering::Descending),
        ),
    ];

    let current_service = filter_items.iter().nth(app.sort_index).cloned();

    let filter_item_rows = filter_items
        .iter()
        .map(|(item, order, sort_enum)| {
            let column_a =
                Cell::from(Line::from(vec![Span::raw(item.clone())]).alignment(Alignment::Left));

            let current_sort = &app.order;

            let column_b = if current_sort == sort_enum {
                Cell::from(
                    Line::from(vec![Span::raw("[x]".to_string())]).alignment(Alignment::Left),
                )
            } else {
                Cell::from(
                    Line::from(vec![Span::raw("[ ]".to_string())]).alignment(Alignment::Left),
                )
            };

            let (sort_type, sort_order, _enum) = current_service.clone().unwrap();

            let row_style = if current_service.is_some()
                && sort_type == item.to_string()
                && sort_order == order.deref()
            {
                RowStyle::Selected
            } else {
                RowStyle::Default
            };

            let middle = Cell::from(
                Line::from(vec![Span::raw("Method".to_string())]).alignment(Alignment::Left),
            );

            let order1 = Cell::from(
                Line::from(vec![Span::raw(order.to_string())]).alignment(Alignment::Left),
            );

            Row::new(vec![
                column_b.clone(),
                middle.clone(),
                column_a.clone(),
                order1,
            ])
            .style(get_row_style(row_style))
        })
        .collect::<Vec<_>>();

    let list = Table::new([filter_item_rows].concat())
        .style(get_text_style(true))
        .header(
            Row::new(vec!["Selected", "Type", "Value", "Order"])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(get_border_style(true))
                .title("[Sort traces by]")
                .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(40),
        ])
        .column_spacing(10);

    frame.render_widget(list.clone(), area);
}
