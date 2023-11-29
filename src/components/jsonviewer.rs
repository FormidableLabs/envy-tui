use std::error::Error;

use ratatui::prelude::*;
use ratatui::widgets::block::Padding;
use ratatui::widgets::{
    Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::Action;
use crate::consts::RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE;

#[derive(Default)]
pub struct JSONViewer {
    pub action_tx: Option<UnboundedSender<Action>>,
    expanded: bool,
    expanded_idxs: Vec<usize>,
    indent_spacing: usize,
    cursor_position: usize,
    title: String,
}

impl JSONViewer {
    pub fn new(indent_spacing: usize, title: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            indent_spacing,
            title: title.to_string(),
            ..Self::default()
        })
    }

    pub fn register_action_handler(
        &mut self,
        tx: UnboundedSender<Action>,
    ) -> Result<(), Box<dyn Error>> {
        self.action_tx = Some(tx);
        Ok(())
    }

    pub fn update(&mut self, action: Action) -> Result<Option<Action>, Box<dyn Error>> {
        match action {
            Action::NavigateUp(Some(_)) => {
                self.cursor_position = self.cursor_position.saturating_sub(1)
            }
            Action::NavigateDown(Some(_)) => {
                // TODO: Clamp cursor_position to number of lines
                self.cursor_position = self.cursor_position.saturating_add(1)
            }
            Action::NavigateLeft(Some(_)) => {
                self.expanded_idxs.retain(|&x| x != self.cursor_position)
            }
            Action::NavigateRight(Some(_)) => self.expanded_idxs.push(self.cursor_position),
            Action::ExpandAll => self.expanded = true,
            Action::CollapseAll => {
                self.expanded = false;
                self.expanded_idxs.clear();
            }
            _ => {}
        }

        Ok(None)
    }

    pub fn render(
        &self,
        f: &mut Frame,
        rect: Rect,
        data: String,
        active: bool, // TODO(vandosant): Could this be moved to self.active?
    ) -> Result<(), Box<dyn Error>> {
        // let copy = if request.duration.is_some() {
        //     "This trace does not have a response body."
        // } else {
        //     "Loading..."
        // };

        let padding = Padding::zero();

        let mut lines = if active {
            active_lines(
                data,
                self.expanded_idxs.clone(),
                self.expanded,
                self.cursor_position,
            )?
        } else {
            raw_lines(data, self.expanded_idxs.clone(), self.expanded)?
        };

        let mut indent: usize = 0;
        for line in lines.iter_mut() {
            if line.spans.iter().any(|s| s.content == "{") {
                line.spans.insert(0, Span::raw(" ".repeat(indent)));
                indent = indent.saturating_add(self.indent_spacing);
                continue;
            } else if line.spans.iter().any(|s| s.content == "}") {
                indent = indent.saturating_sub(self.indent_spacing);
            }

            line.spans.insert(0, Span::raw(" ".repeat(indent)));
        }
        let mut line_indicators = vec![];
        for (idx, line) in lines.iter_mut().enumerate() {
            if idx == 0 {
                line_indicators.push(Line::from(vec![Span::styled(
                    "  ",
                    Style::default()
                        .fg(if active {
                            Color::White
                        } else {
                            Color::DarkGray
                        })
                        .add_modifier(Modifier::BOLD),
                )]));
            } else if line.spans.iter().any(|s| s.content == "{..}") {
                line_indicators.push(Line::from(vec![Span::styled(
                    "˃ ",
                    Style::default()
                        .fg(if active {
                            Color::White
                        } else {
                            Color::DarkGray
                        })
                        .add_modifier(Modifier::BOLD),
                )]));
                continue;
            } else if line.spans.iter().any(|s| s.content == "{") {
                line_indicators.push(Line::from(vec![Span::styled(
                    "˅ ",
                    Style::default()
                        .fg(if active {
                            Color::White
                        } else {
                            Color::DarkGray
                        })
                        .add_modifier(Modifier::BOLD),
                )]));
            } else {
                line_indicators.push(Line::from(vec![Span::styled(
                    "  ",
                    Style::default()
                        .fg(if active {
                            Color::White
                        } else {
                            Color::DarkGray
                        })
                        .add_modifier(Modifier::BOLD),
                )]));
            }
        }

        let line_indicators_paragraph = Paragraph::new(line_indicators).alignment(Alignment::Right);

        let outer_block = Block::default()
            .borders(Borders::ALL)
            .padding(padding)
            .style(Style::default().fg(if active {
                Color::White
            } else {
                Color::DarkGray
            }))
            .title(self.title.to_string())
            .border_type(BorderType::Plain);

        let outer_area = rect;
        let inner_area = outer_block.inner(outer_area);

        let inner_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(4), Constraint::Min(0)])
            .split(inner_area);

        let number_of_lines = lines.len() + 1;

        // let has_overflown_x_axis = lines.iter().any(|l| l.width() > rect.width.into());
        let available_height = inner_layout[1]
            .height
            .saturating_sub(RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE.try_into()?);
        let overflown_number_count = available_height.saturating_sub(number_of_lines.try_into()?);
        let has_overflown_y_axis = overflown_number_count > 0;

        let json = Paragraph::new(lines)
            .style(
                Style::default()
                    .fg(if active {
                        Color::White
                    } else {
                        Color::DarkGray
                    })
                    .add_modifier(Modifier::BOLD),
            )
            .scroll((
                self.cursor_position
                    .saturating_sub(available_height.into())
                    .try_into()?,
                0,
            ));

        f.render_widget(outer_block, outer_area);
        if has_overflown_y_axis {
            // TODO: should number of lines be one fewer?
            let mut scrollbar_state =
                ScrollbarState::new(number_of_lines).position(self.cursor_position);

            f.render_stateful_widget(
                Scrollbar::default().orientation(ScrollbarOrientation::VerticalRight),
                outer_area.inner(&Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }

        // if has_overflown_x_axis {
        //     let mut scrollbar_state =
        //         ScrollbarState::new(number_of_lines).position(self.cursor_position);
        //     let horizontal_scroll = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
        //         .begin_symbol(Some("<-"))
        //         .end_symbol(Some("->"));

        //     f.render_stateful_widget(
        //         horizontal_scroll,
        //         outer_area.inner(&Margin {
        //             vertical: 0,
        //             horizontal: 1,
        //         }),
        //         &mut scrollbar_state,
        //     );
        // }

        f.render_widget(line_indicators_paragraph, inner_layout[0]);
        f.render_widget(json, inner_layout[1]);

        Ok(())
    }
}

fn active_lines(
    data: String,
    expanded_idxs: Vec<usize>,
    expanded: bool,
    cursor_position: usize,
) -> Result<Vec<Line<'static>>, Box<dyn Error>> {
    let mut lines = raw_lines(data, expanded_idxs, expanded)?;

    let style = Style::default().fg(Color::Green);

    if let Some(elem) = lines.get_mut(cursor_position) {
        elem.patch_style(style);
    } else if let Some(elem) = lines.last_mut() {
        elem.patch_style(style);
    }

    Ok(lines)
}

fn raw_lines(
    data: String,
    expanded_idxs: Vec<usize>,
    expanded: bool,
) -> Result<Vec<Line<'static>>, Box<dyn Error>> {
    let v = serde_json::from_str(data.as_str())?;
    let mut items = vec![];

    if let serde_json::Value::Object(o) = v {
        for line in obj_lines(o, &expanded_idxs, expanded, None, 0)? {
            items.push(line);
        }
    } else {
        let as_str: String = value_to_string(v);
        items.push(Line::from(vec![
            r#"""#.into(),
            as_str.into(),
            r#"""#.into(),
        ]));
    }

    Ok(items)
}

fn value_to_string(v: serde_json::Value) -> String {
    match v {
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!(r#""{}""#, s),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Array(a) => format!("{:?}", a),
        serde_json::Value::Object(_) => "{..}".to_string(),
    }
}

fn obj_lines(
    v: serde_json::Map<String, serde_json::Value>,
    expanded_idxs: &Vec<usize>,
    expand_all_objects: bool,
    key: Option<String>,
    initial_idx: usize,
) -> Result<Vec<Line<'static>>, Box<dyn Error>> {
    let mut items = vec![];
    let mut idx = initial_idx;
    let mut len = v.len();

    let as_str = "{";
    if let Some(k) = key {
        items.push(Line::from(vec![
            r#"""#.into(),
            k.into(),
            r#"""#.into(),
            ": ".into(),
            as_str.into(),
        ]))
    } else {
        items.push(Line::from(vec![as_str.into()]));
    }
    idx += 1;

    for (k, v) in v.into_iter() {
        if let serde_json::Value::Object(o) = v.clone() {
            if expand_all_objects || expanded_idxs.contains(&idx) {
                let lines = obj_lines(o, expanded_idxs, expand_all_objects, Some(k), idx)?;
                len += lines.len();
                for mut line in lines {
                    if idx < len && !line.spans.iter().any(|span| span.content.ends_with(&"{")) {
                        line.spans.push(",".into())
                    }
                    items.push(line);
                    idx += 1;
                }
            } else {
                let as_str: String = value_to_string(v.clone());
                items.push(Line::from(vec![
                    r#"""#.into(),
                    k.into(),
                    r#"""#.into(),
                    ": ".into(),
                    as_str.into(),
                    if idx < len { ",".into() } else { "".into() },
                ]));
                idx += 1;
            }
        } else {
            let as_str: String = value_to_string(v.clone());
            items.push(Line::from(vec![
                r#"""#.into(),
                k.into(),
                r#"""#.into(),
                ": ".into(),
                as_str.into(),
                if idx < len { ",".into() } else { "".into() },
            ]));
            idx += 1;
        }
    }

    items.push(Line::from(vec![
        "}".into(),
        if idx < len { ",".into() } else { "".into() },
    ]));

    Ok(items)
}

#[cfg(test)]
mod tests {
    use crate::components::jsonviewer;

    #[test]
    fn exploration() {
        let data = r#"{ "foo": "bar" }"#;
        let result = jsonviewer::JSONViewer::new();
        assert!(result.is_ok());
    }
}
