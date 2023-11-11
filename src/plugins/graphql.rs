use std::io::Stdout;
use std::ops::Deref;

use ratatui::prelude::{Alignment, Constraint, CrosstermBackend, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::components::home::Home;
use crate::render::{get_border_style, get_row_style, get_text_style, RowStyle};
use crate::services::websocket::Trace;
use crate::utils::TraceSort;

pub struct GraphQLPlugin;

pub trait Plugin {
    fn is_match(&self, trace: &Trace) -> bool;

    fn render(
        &self,
        app: &Home,
        trace: &Trace,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
    );
}

impl GraphQLPlugin {
    pub fn new() -> Self {
        GraphQLPlugin {}
    }
}

impl Plugin for GraphQLPlugin {
    fn is_match(&self, trace: &Trace) -> bool {
        trace.graphql.is_some()
    }

    fn render(
        &self,
        app: &Home,
        trace: &Trace,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
    ) {
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
                let column_a = Cell::from(
                    Line::from(vec![Span::raw(item.clone())]).alignment(Alignment::Left),
                );

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
}
