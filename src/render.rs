use core::str::FromStr;
use regex::Regex;
use std::io::Stdout;
use std::ops::Deref;
use std::usize;

use http::{HeaderName, HeaderValue};
use ratatui::prelude::{Alignment, Constraint, CrosstermBackend, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, Padding, Paragraph, Row, Scrollbar,
    ScrollbarOrientation, Table, Tabs,
};
use ratatui::Frame;

use crate::app::{ActiveBlock, App, RequestDetailsPane, Trace, UIState};
use crate::consts::{
    NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE, REQUEST_HEADERS_UNUSABLE_VERTICAL_SPACE,
    RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE, RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE,
};
use crate::utils::{get_currently_selected_trace, parse_query_params, truncate};

#[derive(Clone, Copy, PartialEq, Debug, Hash, Eq)]
enum RowStyle {
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
        .into_iter()
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

pub fn render_response_body(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    match get_currently_selected_trace(&app) {
        Some(request) => match &request.pretty_response_body {
            Some(pretty_json) => {
                render_body(
                    pretty_json.to_string(),
                    &mut app.response_body,
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

fn get_row_style(row_style: RowStyle) -> Style {
    let default_style = Style::default().fg(Color::White);

    let selected_style = Style::default().fg(Color::Black).bg(Color::LightRed);

    let inactive_stlye = Style::default().fg(Color::Black).bg(Color::Gray);

    match row_style {
        RowStyle::Default => default_style,
        RowStyle::Inactive => inactive_stlye,
        RowStyle::Selected => selected_style,
    }
}

fn get_border_style(active: bool) -> Style {
    if active {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn get_text_style(active: bool) -> Style {
    if active {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn render_headers(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    header_type: HeaderType,
) {
    let items_as_vector = app.items.iter().collect::<Vec<&Trace>>();

    let maybe_selected_item = items_as_vector.get(app.main.index);

    let active_block = app.active_block;

    let rows = match maybe_selected_item {
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
                .skip(offset.into())
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

pub fn render_request_block(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    let active_block = app.active_block;

    let items_as_vector = app.items.iter().collect::<Vec<&Trace>>();

    let maybe_selected_item = items_as_vector.get(app.main.index);

    let uri = match maybe_selected_item {
        Some(item) => item.deref().uri.clone(),
        None => String::from("Could not find request."),
    };

    let raw_params = parse_query_params(uri);

    let mut cloned = raw_params.clone();

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

            Row::new(vec![cloned_name, cloned_value]).style(match (is_selected, active_block) {
                (true, ActiveBlock::RequestDetails) => get_row_style(RowStyle::Selected),
                (true, _) => get_row_style(RowStyle::Inactive),
                (_, _) => get_row_style(RowStyle::Default),
            })
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
                .style(
                    Style::default().fg(if active_block == ActiveBlock::RequestDetails {
                        Color::White
                    } else {
                        Color::DarkGray
                    }),
                )
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
                maybe_selected_item.unwrap().request_headers.len()
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

            let trace = get_currently_selected_trace(app);

            let content_length = trace.unwrap().response_headers.len() as u16;

            let viewport_height = area.height - REQUEST_HEADERS_UNUSABLE_VERTICAL_SPACE as u16;

            if content_length > viewport_height {
                frame.render_stateful_widget(
                    vertical_scroll,
                    area.inner(&Margin {
                        horizontal: 0,
                        vertical: 2,
                    }),
                    &mut app.request_details.scroll_state,
                );
            }
        }
    }
}

pub fn render_request_body(app: &mut App, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    match get_currently_selected_trace(&app) {
        Some(request) => match &request.pretty_request_body {
            Some(pretty_json) => {
                render_body(
                    pretty_json.to_string(),
                    &mut app.request_body,
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

pub fn render_response_block(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    let items_as_vector = app.items.iter().collect::<Vec<&Trace>>();

    let maybe_selected_item = items_as_vector.get(app.main.index);

    let uri = match maybe_selected_item {
        Some(item) => item.deref().uri.clone(),
        None => String::from("Could not find request."),
    };

    let raw_params = parse_query_params(uri);

    let is_progress = match maybe_selected_item {
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
        let mut cloned = raw_params.clone();

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
                    maybe_selected_item.unwrap().response_headers.len()
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
            .select(match app.request_details_block {
                RequestDetailsPane::Headers => 0,
                RequestDetailsPane::Query => 1,
            })
            .highlight_style(Style::default().fg(Color::LightMagenta));

        frame.render_widget(main, area);
        frame.render_widget(tabs, inner_layout[0]);

        render_headers(app, frame, inner_layout[1], HeaderType::Response);

        let vertical_scroll = Scrollbar::new(ScrollbarOrientation::VerticalRight);

        let trace = get_currently_selected_trace(app);

        let content_length = trace.unwrap().response_headers.len();

        if content_length > area.height as usize - RESPONSE_HEADERS_UNUSABLE_VERTICAL_SPACE {
            frame.render_stateful_widget(
                vertical_scroll,
                area.inner(&Margin {
                    horizontal: 0,
                    vertical: 2,
                }),
                &mut app.response_details.scroll_state,
            );
        }
    }
}

fn fuzzy_regex(query: String) -> Regex {
    if query.is_empty() {
        return Regex::new(r".*").unwrap();
    }

    let mut fuzzy_query = String::new();

    for c in query.chars() {
        fuzzy_query.extend([c, '.', '*']);
    }

    return Regex::from_str(&fuzzy_query).unwrap();
}

pub fn render_traces(app: &mut App, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let requests = &app.items;
    let re = fuzzy_regex(app.search_query.clone());

    let height = area.height;

    let effective_height = height - NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE as u16;

    let active_block = app.active_block.clone();

    let items_as_vector = requests
        .iter()
        .filter(|i| re.is_match(&i.uri))
        .collect::<Vec<&Trace>>();

    let number_of_lines = items_as_vector.len();

    let selected_item = items_as_vector.get(app.main.index);

    let converted_rows: Vec<(Vec<String>, bool)> = items_as_vector
        .iter()
        .skip(app.main.offset.into())
        .take(effective_height.into())
        .map(|request| {
            let uri = truncate(request.uri.clone().as_str(), 60);

            let method = request.method.clone().to_string();

            let status = match request.status {
                Some(v) => v.as_u16().to_string(),
                None => "...".to_string(),
            };

            let duration = match request.duration {
                Some(v) => {
                    format!("{:.3} s", ((v as f32) / 1000.0))
                }
                None => "...".to_string(),
            };

            let id = request.id.clone().to_string();

            let selected = match selected_item {
                Some(item) => {
                    if item.deref() == request.deref() {
                        true
                    } else {
                        false
                    }
                }
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
                .title("Traces")
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
            &mut app.main.scroll_state,
        );
    }
}

pub fn render_search(app: &mut App, frame: &mut Frame<CrosstermBackend<Stdout>>) {
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

pub fn render_footer(app: &mut App, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let ws_status = match app.ws_server_state {
        crate::app::WsServerState::Open => "🟠 Waiting for connection".to_string(),
        crate::app::WsServerState::Closed => "⭕ Server closed".to_string(),
        crate::app::WsServerState::HasConnections(1) => {
            format!("🟢 {:?} client connected", 1)
        }
        crate::app::WsServerState::HasConnections(clients) => {
            format!("🟢 {:?} clients connected", clients)
        }
    };

    let general_status = match &app.status_message {
        Some(text) => text,
        None => "",
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

    let status_bar = Paragraph::new(format!("{} {}", general_status, ws_status))
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

pub fn render_request_summary(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    let items_as_vector = app.items.iter().collect::<Vec<&Trace>>();

    let selected_item = items_as_vector.get(app.main.index);

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

pub fn render_help(_app: &mut App, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    // TODO: Render different Keybindings that are relevant for the given `active_block`.
    let status_bar = Paragraph::new("Keybindings")
        .style(get_text_style(true))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(get_border_style(true))
                .title("Help")
                .border_type(BorderType::Plain),
        );

    frame.render_widget(status_bar, area);
}

pub fn render_debug(app: &mut App, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let debug_lines = app
        .logs
        .iter()
        .map(|item| ListItem::new(Line::from(Span::raw(item))))
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
