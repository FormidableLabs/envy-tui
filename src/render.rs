use std::io::Stdout;
use std::ops::Deref;

use ratatui::prelude::{Alignment, Constraint, CrosstermBackend, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::{ActiveBlock, App};

pub fn render_request_details(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    let index = app.selection_index.clone();

    let selected_item = app.items.get(index).clone();

    let selected = match selected_item {
        Some(item) => item.deref().to_string(),
        None => String::from("Could not find request."),
    };

    let container = Block::default()
        .borders(Borders::ALL)
        .style(
            Style::default().fg(if app.active_block == ActiveBlock::RequestDetails {
                Color::White
            } else {
                Color::DarkGray
            }),
        )
        .title("Request details")
        .border_type(BorderType::Plain);

    let details = Paragraph::new(selected)
        .style(
            Style::default().fg(if app.active_block == ActiveBlock::RequestDetails {
                Color::White
            } else {
                Color::DarkGray
            }),
        )
        .alignment(Alignment::Center)
        .block(container);

    frame.render_widget(details, area);
}

pub fn render_request_headers(
    app: &mut App,
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
) {
    let index = app.selection_index.clone();

    let selected_item = app.items.get(index).clone();

    let active_block = app.active_block;

    let rows = match selected_item {
        Some(item) => item
            .request_headers
            .iter()
            .map(|(name, value)| {
                let header_name = name.as_str();

                let header_value = match value.to_str() {
                    Ok(v) => v,
                    _ => "Unknown header value",
                };

                Row::new(vec![String::from(header_name), String::from(header_value)])
            })
            .collect(),
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
        // As any other widget, a Table can be wrapped in a Block.
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(
                    Style::default().fg(if active_block == ActiveBlock::RequestHeaders {
                        Color::White
                    } else {
                        Color::DarkGray
                    }),
                )
                .title("Request headers")
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
            let uri = request.uri.clone();
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

    let default_style = Style::default().fg(Color::White);

    let selected_style = Style::default().fg(Color::Black).bg(Color::LightRed);

    // NOTE: Why iter or map gives back ref?
    let _mapped_over: Vec<Paragraph> = [Paragraph::new("title")]
        .iter()
        .map(|x| {
            let cloned = x.deref().clone();

            cloned
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

            Row::new(str_vec).style(if *selected {
                selected_style
            } else {
                default_style
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
                .style(
                    Style::default().fg(if active_block == ActiveBlock::NetworkRequests {
                        Color::White
                    } else {
                        Color::DarkGray
                    }),
                )
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
