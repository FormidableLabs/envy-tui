use std::collections::HashSet;
use std::ops::Deref;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Span};
use ratatui::{
    style::{Modifier, Style},
    symbols,
    symbols::border,
    widgets::{
        block::{Position, Title},
        canvas, Block, BorderType, Borders, Cell, Clear, List, ListItem, Padding, Paragraph, Row,
        Scrollbar, ScrollbarOrientation, Table, Tabs, Widget,
    },
    Frame,
};

use crate::app::{Action, ActiveBlock, DetailsPane};
use crate::components::actionable_list::ActionableList;
use crate::components::home::{FilterSource, Home};
use crate::config::Colors;
use crate::consts::NETWORK_REQUESTS_UNUSABLE_VERTICAL_SPACE;
use crate::services::websocket::Trace;
use crate::utils::{get_rendered_items, truncate, TraceSort};

#[derive(Clone, Copy, PartialEq, Debug, Hash, Eq)]
pub enum RowStyle {
    Default,
    Selected,
    Active,
    Inactive,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum HeaderType {
    Request,
    Response,
}

pub fn get_row_style(row_style: RowStyle, colors: &Colors) -> Style {
    let default_style = Style::default().fg(colors.text.unselected);

    let active_style = Style::default().fg(colors.text.default);

    let selected_style = Style::default()
        .fg(colors.text.selected)
        .bg(colors.surface.selected);

    let inactive_style = Style::default()
        .fg(colors.text.selected)
        .bg(colors.surface.unselected);

    match row_style {
        RowStyle::Default => default_style,
        RowStyle::Active => active_style,
        RowStyle::Inactive => inactive_style,
        RowStyle::Selected => selected_style,
    }
}

pub fn get_border_style(active: bool, colors: &Colors) -> Style {
    if active {
        Style::default().fg(colors.surface.selected)
    } else {
        Style::default().fg(colors.surface.unselected)
    }
}

fn get_text_style(active: bool, colors: &Colors) -> Style {
    if active {
        Style::default().fg(colors.text.default)
    } else {
        Style::default().fg(colors.text.unselected)
    }
}

pub fn details(app: &mut Home, frame: &mut Frame, area: Rect) {
    let mut cells: Vec<Rect> = vec![];

    match app.details_panes.len() {
        0 => cells.push(area),
        1 => cells.extend(
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)].as_ref())
                .split(area)
                .iter(),
        ),
        2 => {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)].as_ref())
                .split(area);

            cells.push(columns[0]);
            cells.extend(
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)].as_ref())
                    .split(columns[1])
                    .iter(),
            )
        }
        3 => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)].as_ref())
                .split(area);

            let top_row_columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)].as_ref())
                .split(rows[0]);

            cells.push(top_row_columns[0]);
            cells.extend(
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
                    .split(top_row_columns[1])
                    .iter(),
            );

            // add bottom row
            cells.push(rows[1]);
        }
        4 => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)].as_ref())
                .split(area);

            let top_row_columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)].as_ref())
                .split(rows[0]);

            cells.push(top_row_columns[0]);
            cells.extend(
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
                    .split(top_row_columns[1])
                    .iter(),
            );

            // split bottom row
            cells.extend(
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)].as_ref())
                    .split(rows[1])
                    .iter(),
            );
        }
        5 => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)].as_ref())
                .split(area);

            let top_row_columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)].as_ref())
                .split(rows[0]);

            cells.push(top_row_columns[0]);
            cells.extend(
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
                    .split(top_row_columns[1])
                    .iter(),
            );

            // split bottom row
            cells.extend(
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)].as_ref())
                    .constraints(
                        [
                            Constraint::Ratio(1, 3),
                            Constraint::Ratio(1, 3),
                            Constraint::Ratio(1, 3),
                        ]
                        .as_ref(),
                    )
                    .split(rows[1])
                    .iter(),
            );
        }
        _ => cells.push(area),
    };

    details_tabs(app, frame, cells[0]);

    for (idx, &cell) in cells[1..].iter().enumerate() {
        details_pane(app, frame, cell, idx);
    }
}

pub fn details_pane(app: &mut Home, frame: &mut Frame, area: Rect, pane_idx: usize) {
    if let Some(selected_trace) = &app.selected_trace {
        if let Some(pane) = &app.details_panes.get(pane_idx) {
            let is_active = app.active_block == ActiveBlock::Details && app.details_block == **pane;

            let inner_layout = Layout::default()
                .vertical_margin(2)
                .horizontal_margin(3)
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)].as_ref())
                .split(area);

            let actionable_list = match pane {
                DetailsPane::RequestDetails => &mut app.request_details_list,
                DetailsPane::QueryParams => &mut app.query_params_list,
                DetailsPane::RequestHeaders => &mut app.request_headers_list,
                DetailsPane::ResponseDetails => &mut app.response_details_list,
                DetailsPane::ResponseHeaders => &mut app.response_headers_list,
                DetailsPane::Timing => &mut app.timing_list,
            };

            let details_block = Block::default()
                .title(format!("  {}  ", pane))
                .title(
                    Title::from(format!(
                        "  {} OF {}  ",
                        actionable_list.state.selected().unwrap_or(0) + 1,
                        actionable_list.items.len(),
                    ))
                    .position(Position::Bottom)
                    .alignment(Alignment::Right),
                )
                .border_style(get_border_style(is_active, &app.colors))
                .border_type(BorderType::Plain)
                .borders(Borders::ALL);

            frame.render_widget(details_block, area);

            if pane == &&DetailsPane::Timing {
                render_timing_chart(
                    selected_trace,
                    actionable_list,
                    inner_layout[0],
                    frame,
                    &app.colors,
                    is_active,
                );
            } else {
                render_actionable_list(
                    actionable_list,
                    frame,
                    inner_layout[0],
                    &app.colors,
                    is_active,
                );
            }
        }
    }
}

pub fn details_tabs(app: &mut Home, frame: &mut Frame, area: Rect) {
    if let Some(selected_trace) = &app.selected_trace {
        let is_active = app.active_block == ActiveBlock::Details
            && app.details_tabs.contains(&app.details_block);

        let tabs = Tabs::new(app.details_tabs.iter().map(|t| t.to_string()).collect())
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(get_border_style(is_active, &app.colors))
                    .border_type(BorderType::Plain)
                    .border_set(border::DOUBLE),
            )
            .select(app.details_tab_index)
            .style(Style::default().fg(if is_active {
                app.colors.text.accent_1
            } else {
                app.colors.text.unselected
            }))
            .highlight_style(Style::default().fg(if is_active {
                app.colors.text.accent_2
            } else {
                app.colors.text.unselected
            }));

        let inner_layout = Layout::default()
            .vertical_margin(2)
            .horizontal_margin(3)
            .direction(Direction::Vertical)
            .constraints([Constraint::Max(2), Constraint::Min(1)].as_ref())
            .split(area);

        let tab_block = app
            .details_tabs
            .get(app.details_tab_index)
            .unwrap_or(&app.details_tabs[0]);

        let actionable_list = match tab_block {
            DetailsPane::RequestDetails => &mut app.request_details_list,
            DetailsPane::QueryParams => &mut app.query_params_list,
            DetailsPane::RequestHeaders => &mut app.request_headers_list,
            DetailsPane::ResponseDetails => &mut app.response_details_list,
            DetailsPane::ResponseHeaders => &mut app.response_headers_list,
            DetailsPane::Timing => &mut app.timing_list,
        };

        let details_block = Block::default()
            .title("  DETAILS  ")
            .title(
                Title::from(format!(
                    "  {} OF {}  ",
                    actionable_list.state.selected().unwrap_or(0) + 1,
                    actionable_list.items.len(),
                ))
                .position(Position::Bottom)
                .alignment(Alignment::Right),
            )
            .border_style(get_border_style(is_active, &app.colors))
            .border_type(BorderType::Plain)
            .borders(Borders::ALL);

        frame.render_widget(details_block, area);
        frame.render_widget(tabs, inner_layout[0]);

        if *tab_block == DetailsPane::Timing {
            render_timing_chart(
                selected_trace,
                actionable_list,
                inner_layout[1],
                frame,
                &app.colors,
                app.active_block == ActiveBlock::Details && app.details_block == *tab_block,
            );
        } else {
            render_actionable_list(
                actionable_list,
                frame,
                inner_layout[1],
                &app.colors,
                app.active_block == ActiveBlock::Details && app.details_block == *tab_block,
            );
        }
    }
}

fn render_actionable_list(
    actionable_list: &mut ActionableList,
    frame: &mut Frame,
    area: Rect,
    colors: &Colors,
    active: bool,
) {
    let actionable_item_style = Style::default().fg(colors.text.accent_2);
    let active_item_style = get_row_style(RowStyle::Active, colors);
    let default_item_style = get_row_style(RowStyle::Default, colors);

    let items: Vec<ListItem> = actionable_list
        .items
        .iter()
        .map(|item| {
            ListItem::new(Line::from(vec![
                Span::raw(format!("{:<15}", item.label)),
                " ".into(),
                Span::styled(
                    item.value.clone().unwrap_or_default().to_string(),
                    if active && item.action.is_some() {
                        actionable_item_style
                    } else if active {
                        active_item_style
                    } else {
                        default_item_style
                    },
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().fg(if active {
            colors.text.accent_1
        } else {
            colors.text.unselected
        }))
        .highlight_style(if active {
            get_row_style(RowStyle::Selected, colors)
        } else {
            get_row_style(RowStyle::Inactive, colors)
        });

    frame.render_stateful_widget(list, area, &mut actionable_list.state)
}

fn render_timing_chart(
    trace: &Trace,
    actionable_list: &mut ActionableList,
    area: Rect,
    frame: &mut Frame,
    colors: &Colors,
    active: bool,
) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(10), Constraint::Percentage(90)].as_ref())
        .split(area);

    render_actionable_list(actionable_list, frame, layout[0], colors, active);

    if let Some(http) = &trace.http {
        if let Some(timings) = &http.timings {
            let timings_vec: Vec<f64> = vec![
                timings.blocked.into(),
                timings.dns.into(),
                timings.connect.into(),
                timings.ssl.into(),
                timings.send.into(),
                timings.wait.into(),
                timings.receive.into(),
            ];
            let total = timings_vec.clone().iter().fold(0.0, |a, b| a + b);

            let chart_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(8), Constraint::Min(0)].as_ref())
                .split(layout[1]);

            canvas::Canvas::default()
                .marker(symbols::Marker::HalfBlock)
                .x_bounds([0.0, total + 4.0])
                .y_bounds([timings_vec.len() as f64 * -1.0, 1.0])
                .paint(|ctx| {
                    for (i, &v) in timings_vec.iter().enumerate() {
                        let float_i = i as f64;
                        ctx.draw(&canvas::Rectangle {
                            x: timings_vec[0..i].iter().fold(0.0, |a, b| a + b),
                            y: -float_i,
                            width: v,
                            height: 0.5,
                            color: if active {
                                colors.surface.null
                            } else {
                                colors.surface.unselected
                            },
                        });
                        ctx.print(
                            v + timings_vec[0..i].iter().fold(0.0, |a, b| a + b) + 1.0,
                            -float_i,
                            Line::styled(
                                format!("{:.2}", v),
                                Style::default().fg(if active {
                                    colors.text.accent_1
                                } else {
                                    colors.text.unselected
                                }),
                            ),
                        )
                    }
                })
                .render(chart_layout[0], frame.buffer_mut());
        }
    }
}

pub fn render_traces(app: &Home, frame: &mut Frame, area: Rect) {
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
                (true, ActiveBlock::Traces) => get_row_style(RowStyle::Selected, &app.colors),
                (false, ActiveBlock::Traces) => get_row_style(RowStyle::Active, &app.colors),
                (true, _) => get_row_style(RowStyle::Inactive, &app.colors),
                (false, _) => get_row_style(RowStyle::Default, &app.colors),
            })
        })
        .collect();

    let requests = Table::new(styled_rows)
        // You can set the style of the entire Table.
        .style(Style::default().fg(app.colors.surface.selected))
        // It has an optional header, which is simply a Row always visible at the top.
        .header(
            Row::new(vec!["Method", "Status", "Request", "Duration"])
                .style(Style::default().fg(app.colors.text.accent_1))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(get_border_style(
                    app.active_block == ActiveBlock::Traces,
                    &app.colors,
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

pub fn render_search(app: &Home, frame: &mut Frame) {
    if app.active_block == ActiveBlock::SearchQuery {
        let area = overlay_area(frame.size());
        let widget = Paragraph::new(format!("/{}", &app.search_query))
            .style(
                Style::default()
                    .fg(app.colors.text.selected)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Left);

        frame.render_widget(Clear, area);
        frame.render_widget(widget, area);
    }
}

pub fn render_footer(app: &Home, frame: &mut Frame, area: Rect) {
    let general_status = match app.status_message.clone() {
        Some(text) => text,
        None => "".to_string(),
    };

    let help_text = Paragraph::new("For help, press ?")
        .style(
            Style::default()
                .fg(app.colors.text.accent_1)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.colors.surface.unselected))
                .title("Status Bar")
                .padding(Padding::new(1, 0, 0, 0))
                .border_type(BorderType::Plain),
        );

    let wss_status_message = match app.wss_state {
        crate::components::home::WebSockerInternalState::Connected(1) => {
            "🟢 1 client connected".to_string()
        }
        crate::components::home::WebSockerInternalState::Connected(v) => {
            format!("🟢 {:?} clients connected", v)
        }
        crate::components::home::WebSockerInternalState::Closed => "⭕ Server closed".to_string(),

        _ => "🟠 Waiting for connection".to_string(),
    };

    let status_bar = Paragraph::new(format!("{} {}", general_status, wss_status_message))
        .style(
            Style::default()
                .fg(app.colors.text.selected)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Right)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.colors.surface.unselected))
                .title("Status Bar")
                .padding(Padding::new(0, 1, 0, 0))
                .border_type(BorderType::Plain),
        );

    frame.render_widget(status_bar, area);

    frame.render_widget(help_text, area);
}

pub fn render_help(app: &Home, frame: &mut Frame, area: Rect) {
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
                Action::NextDetailsTab => "Focus On Next Tab",
                Action::PreviousDetailsTab => "Go To Previous Tab",
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

            Row::new(vec![column_a, column_b]).style(get_row_style(RowStyle::Default, &app.colors))
        })
        .collect::<Vec<_>>();

    let list = Table::new(debug_lines)
        .style(get_text_style(true, &app.colors))
        .header(
            Row::new(vec!["Action", "Map"])
                .style(Style::default().fg(app.colors.text.accent_1))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(get_border_style(true, &app.colors))
                .title("Key Mappings")
                .border_type(BorderType::Plain),
        )
        .widths(&[Constraint::Percentage(40), Constraint::Percentage(60)])
        .column_spacing(10);

    frame.render_widget(list, area);
}

pub fn render_debug(app: &Home, frame: &mut Frame, area: Rect) {
    let debug_lines = app
        .logs
        .iter()
        .map(|item| ListItem::new(Line::from(Span::raw(item))))
        .collect::<Vec<_>>();

    // TODO: Render different Keybindings that are relevant for the given `active_block`.
    let list = List::new(debug_lines)
        .style(get_text_style(true, &app.colors))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(get_border_style(true, &app.colors))
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

pub fn render_filters_source(app: &Home, frame: &mut Frame, area: Rect) {
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
                        .style(get_row_style(row_style, &app.colors));
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
                        .style(get_row_style(row_style, &app.colors));
                }
            };
        })
        .collect::<Vec<_>>();

    let list = Table::new([rows].concat())
        .style(get_text_style(true, &app.colors))
        .header(
            Row::new(vec!["Selected", "Type", "Value"])
                .style(Style::default().fg(app.colors.text.accent_1))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(get_border_style(true, &app.colors))
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

pub fn render_filters_status(app: &Home, frame: &mut Frame, area: Rect) {
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

            Row::new(vec![column_b, h, column_a]).style(get_row_style(row_style, &app.colors))
        })
        .collect::<Vec<_>>();

    let list = Table::new([rows1].concat())
        .style(get_text_style(true, &app.colors))
        .header(
            Row::new(vec!["Selected", "Type", "Value"])
                .style(Style::default().fg(app.colors.text.accent_1))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(get_border_style(true, &app.colors))
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

pub fn render_filters_method(app: &Home, frame: &mut Frame, area: Rect) {
    let current_service = app
        .method_filters
        .iter()
        .map(|(_a, b)| b.name.clone())
        .nth(app.filter_index);

    let rows1 = app
        .method_filters
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

            Row::new(vec![column_b, h, column_a]).style(get_row_style(row_style, &app.colors))
        })
        .collect::<Vec<_>>();

    let list = Table::new([rows1].concat())
        .style(get_text_style(true, &app.colors))
        .header(
            Row::new(vec!["Selected", "Type", "Value"])
                .style(Style::default().fg(app.colors.text.accent_1))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(get_border_style(true, &app.colors))
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

pub fn render_filters(app: &Home, frame: &mut Frame, area: Rect) {
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

            Row::new(vec![column_b, middle, column_a]).style(get_row_style(row_style, &app.colors))
        })
        .collect::<Vec<_>>();

    let list = Table::new([filter_item_rows].concat())
        .style(get_text_style(true, &app.colors))
        .header(
            Row::new(vec!["Selected", "Type", "Value"])
                .style(Style::default().fg(app.colors.text.accent_1))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(get_border_style(true, &app.colors))
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

pub fn render_sort(app: &Home, frame: &mut Frame, area: Rect) {
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
            .style(get_row_style(row_style, &app.colors))
        })
        .collect::<Vec<_>>();

    let list = Table::new([filter_item_rows].concat())
        .style(get_text_style(true, &app.colors))
        .header(
            Row::new(vec!["Selected", "Type", "Value", "Order"])
                .style(Style::default().fg(app.colors.text.accent_1))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(get_border_style(true, &app.colors))
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
