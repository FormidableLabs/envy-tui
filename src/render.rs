use std::io::Stdout;
use std::ops::Deref;

use http::{HeaderName, HeaderValue};
use ratatui::prelude::{Alignment, Constraint, CrosstermBackend, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table, Tabs};
use ratatui::Frame;
use serde_json::Value;

use std::error::Error;

use crate::app::{ActiveBlock, App, Request, RequestDetailsPane, ResponseDetailsPane};
use crate::utils::{parse_query_params, truncate};

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

fn pretty_parse_body(json: &str) -> Result<String, Box<dyn Error>> {
    let potential_json_body = serde_json::from_str::<Value>(json)?;

    let parsed_json = serde_json::to_string_pretty(&potential_json_body)?;

    Ok(parsed_json)
}

fn render_body(app: &App, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let items_as_vector = app.items.iter().collect::<Vec<&Request>>();

    let maybe_selected_item = items_as_vector.get(app.selection_index);

    match maybe_selected_item {
        Some(selected_item) => match &selected_item.response_body {
            Some(response_body) => match pretty_parse_body(response_body.as_str()) {
                Ok(pretty_json) => {
                    let body_to_render = Paragraph::new(pretty_json).style(
                        Style::default()
                            .fg(if app.active_block == ActiveBlock::ResponseDetails {
                                Color::White
                            } else {
                                Color::DarkGray
                            })
                            .add_modifier(Modifier::BOLD),
                    );

                    frame.render_widget(body_to_render, area);
                }
                Err(_) => {}
            },
            _ => {}
        },
        None => {}
    };
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
    let items_as_vector = app.items.iter().collect::<Vec<&Request>>();

    let maybe_selected_item = items_as_vector.get(app.selection_index);

    let active_block = app.active_block;

    let rows = match maybe_selected_item {
        Some(item) => {
            let headers = if header_type == HeaderType::Request {
                &item.request_headers
            } else {
                &item.response_headers
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
        .highlight_symbol(">>");

    frame.render_widget(table, area);
}

pub fn render_request_block(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    let active_block = app.active_block;

    let items_as_vector = app.items.iter().collect::<Vec<&Request>>();

    let maybe_selected_item = items_as_vector.get(app.selection_index);

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
                .border_type(BorderType::Thick),
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
        .borders(Borders::ALL);

    frame.render_widget(main, area);
    frame.render_widget(tabs, inner_layout[0]);

    match app.request_details_block {
        // RequestDetailsPane::Body => {
        //     match maybe_selected_item {
        //         Some(selected_item) => match &selected_item.request_body {
        //             Some(request_body) => match pretty_parse_body(request_body.as_str()) {
        //                 Ok(pretty_json) => {
        //                     let body_to_render = Paragraph::new(pretty_json).style(
        //                         Style::default()
        //                             .fg(if active_block == ActiveBlock::ResponseDetails {
        //                                 Color::White
        //                             } else {
        //                                 Color::DarkGray
        //                             })
        //                             .add_modifier(Modifier::BOLD),
        //                     );
        //
        //                     frame.render_widget(body_to_render, inner_layout[1]);
        //                 }
        //                 Err(_) => {}
        //             },
        //             _ => {}
        //         },
        //         None => {}
        //     };
        // }
        RequestDetailsPane::Query => {
            frame.render_widget(table, inner_layout[1]);
        }
        RequestDetailsPane::Headers => {
            render_headers(app, frame, inner_layout[1], HeaderType::Request)
        }
    }
}

pub fn render_request_body(app: &mut App, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let items_as_vector = app.items.iter().collect::<Vec<&Request>>();

    let maybe_selected_item = items_as_vector.get(app.selection_index);

    match maybe_selected_item {
        Some(selected_item) => match &selected_item.request_body {
            Some(request_body) => match pretty_parse_body(request_body.as_str()) {
                Ok(pretty_json) => {
                    let body_to_render = Paragraph::new(pretty_json)
                        .style(
                            Style::default()
                                .fg(if app.active_block == ActiveBlock::RequestDetails {
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
                                    if app.active_block == ActiveBlock::RequestDetails {
                                        Color::White
                                    } else {
                                        Color::DarkGray
                                    },
                                ))
                                .title("Request body")
                                .border_type(BorderType::Thick),
                        );

                    frame.render_widget(body_to_render, area);
                }
                Err(_) => {}
            },
            _ => {
                let status_bar = Paragraph::new("This request does not have a body")
                    .style(get_text_style(
                        app.active_block == ActiveBlock::RequestDetails,
                    ))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .style(get_border_style(
                                app.active_block == ActiveBlock::RequestDetails,
                            ))
                            .title("Request Body")
                            .border_type(BorderType::Plain),
                    );

                frame.render_widget(status_bar, area);
            }
        },
        None => {}
    };
}

pub fn render_response_block(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    let items_as_vector = app.items.iter().collect::<Vec<&Request>>();

    let maybe_selected_item = items_as_vector.get(app.selection_index);

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

        let tabs = Tabs::new(vec!["Response Body", "Response Header"])
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .style(Style::default().fg(Color::DarkGray))
                    .border_type(BorderType::Thick),
            )
            .select(match app.response_details_block {
                ResponseDetailsPane::Body => 0,
                ResponseDetailsPane::Headers => 1,
            })
            .highlight_style(Style::default().fg(Color::White));

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Max(2), Constraint::Min(1)].as_ref())
            .split(area);

        let main = Block::default()
            .title("Response details")
            .borders(Borders::ALL);

        frame.render_widget(main, area);
        frame.render_widget(tabs, inner_layout[0]);

        match app.response_details_block {
            ResponseDetailsPane::Body => {
                render_body(app, frame, inner_layout[1]);
            }
            ResponseDetailsPane::Headers => {
                render_headers(app, frame, inner_layout[1], HeaderType::Response)
            }
        }
    }
}

pub fn render_network_requests(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    let requests = &app.items;

    let active_block = app.active_block.clone();

    let items_as_vector = requests.iter().collect::<Vec<&Request>>();

    let selected_item = items_as_vector.get(app.selection_index);

    let converted_rows: Vec<(Vec<String>, bool)> = items_as_vector
        .iter()
        .map(|request| {
            let uri = truncate(request.uri.clone().as_str(), 60);

            let method = request.method.clone().to_string();

            let status = match request.status {
                Some(v) => v.to_string(),
                None => "...".to_string(),
            };

            let duration = match request.duration {
                Some(v) => {
                    format!("{} miliseconds", v.to_string())
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
                (true, ActiveBlock::NetworkRequests) => get_row_style(RowStyle::Selected),
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
                // If you want some space between the header and the rest of the rows, you can always
                // specify some margin at the bottom.
                .bottom_margin(1),
        )
        // As any other widget, a Table can be wrapped in a Block.
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(get_border_style(
                    app.active_block == ActiveBlock::NetworkRequests,
                ))
                .title("Network requests")
                .border_type(BorderType::Plain),
        )
        // Columns widths are constrained in the same way as Layout...
        .widths(&[
            Constraint::Percentage(10),
            Constraint::Percentage(20),
            Constraint::Percentage(50),
            Constraint::Length(20),
        ])
        // ...and they can be separated by a fixed spacing.
        // .column_spacing(1)
        // If you wish to highlight a row in any specific way when it is selected...
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        // ...and potentially show a symbol in front of the selection.
        .highlight_symbol(">>");

    frame.render_widget(requests, area);
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
                .border_type(BorderType::Plain),
        );

    frame.render_widget(status_bar, area);
}

pub fn render_request_summary(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    // TODO:
    // let item = &app.items[app.selection_index];

    let items_as_vector = app.items.iter().collect::<Vec<&Request>>();

    let selected_item = items_as_vector.get(app.selection_index);

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
