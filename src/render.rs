use std::io::Stdout;
use std::ops::Deref;

use ratatui::prelude::{Alignment, Constraint, CrosstermBackend, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::{ActiveBlock, App};
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

fn render_headers(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    header_type: HeaderType,
) {
    let index = app.selection_index.clone();

    let selected_item = app.items.get(index).clone();

    let active_block = app.active_block;

    let rows = match selected_item {
        Some(item) => {
            let headers = if header_type == HeaderType::Request {
                &item.request_headers
            } else {
                &item.response_headers
            };

            let rows = headers
                .iter()
                .map(|(name, value)| {
                    let header_name = name.as_str();

                    let header_value = match value.to_str() {
                        Ok(v) => v,
                        _ => "Unknown header value",
                    };

                    Row::new(vec![String::from(header_name), String::from(header_value)])
                })
                .collect();

            rows
        }
        None => vec![Row::new(vec!["No headers found."])],
    };

    let title = if header_type == HeaderType::Request {
        "Request headers"
    } else {
        "Response headers"
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
        // As any other widget, a Table can be wrapped in a Block.
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(match (active_block, header_type) {
                    (ActiveBlock::RequestHeaders, HeaderType::Request) => get_border_style(true),
                    (ActiveBlock::ResponseHeaders, HeaderType::Response) => get_border_style(true),
                    (_, _) => get_border_style(false),
                })
                .title(title)
                .border_type(BorderType::Plain),
        )
        // Columns widths are constrained in the same way as Layout...
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

pub fn render_request_query_params(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    let active_block = app.active_block;

    let selected_item = app.items.get(app.selection_index);

    let uri = match selected_item {
        Some(item) => item.deref().uri.clone(),
        None => String::from("Could not find request."),
    };

    let raw_params = parse_query_params(uri);

    let current_param_selected = &raw_params.get(app.selected_params_index);

    let rows = raw_params
        .iter()
        .map(|param| {
            let (name, value) = param;
            let cloned_name = name.deref().clone();
            let cloned_value = value.deref().clone();

            let is_selected = match current_param_selected {
                Some(v) => v.deref() == param,
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
        // As any other widget, a Table can be wrapped in a Block.
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(
                    Style::default().fg(if active_block == ActiveBlock::RequestDetails {
                        Color::White
                    } else {
                        Color::DarkGray
                    }),
                )
                .title(app.selected_params_index.to_string())
                .title("Request Query Params")
                .border_type(BorderType::Plain),
        )
        // Columns widths are constrained in the same way as Layout...
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

pub fn render_request_headers(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    render_headers(app, frame, area, HeaderType::Request)
}

pub fn render_response_headers(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    render_headers(app, frame, area, HeaderType::Response)
}

pub fn render_network_requests(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    let requests = &app.items;

    let active_block = app.active_block.clone();

    let index = app.selection_index.clone();

    let converted_rows: Vec<(Vec<String>, bool)> = requests
        .iter()
        .map(|request| {
            let uri = truncate(request.uri.clone().as_str(), 40);
            let method = request.method.clone().to_string();
            let status = request.status.clone().to_string();
            let time = request.duration.clone().to_string();
            let id = request.id.clone().to_string();
            let selected_item = requests.get(index).clone();

            let selected = match selected_item {
                Some(item) => {
                    if item.deref() == request {
                        true
                    } else {
                        false
                    }
                }
                None => false,
            };

            (vec![method, status, uri, time, id], selected)
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
            Row::new(vec!["Method", "Status", "Request", "Time"])
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

pub fn render_footer(_app: &mut App, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let status_bar = Paragraph::new("Status Bar")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
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
    let status_bar = Paragraph::new("Request Summary")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(get_border_style(app.active_block == ActiveBlock::Summary))
                .title("Request Summary")
                .border_type(BorderType::Plain),
        );

    frame.render_widget(status_bar, area);
}
