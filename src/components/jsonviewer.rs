use std::error::Error;

use ratatui::prelude::{
    Alignment, Color, Constraint, Direction, Layout, Line, Margin, Modifier, Rect, Span, Style,
};
use ratatui::widgets::{
    block::Padding, Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Wrap,
};
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::{Action, ActiveBlock};
use crate::consts::RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE;

#[derive(Default)]
pub struct JSONViewer {
    pub action_tx: Option<UnboundedSender<Action>>,
    active_block: ActiveBlock,
    is_active: bool,
    is_expanded: bool,
    expanded_idxs: Vec<usize>,
    indent_spacing: usize,
    cursor_position: usize,
    title: String,
    data: Option<String>,
}

impl JSONViewer {
    pub fn new(
        active_block: ActiveBlock,
        indent_spacing: usize,
        title: &str,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            active_block,
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
                if self.is_active {
                    self.cursor_position = self.cursor_position.saturating_sub(1)
                }
            }
            Action::NavigateDown(Some(_)) => {
                if self.is_active {
                    let max_cursor_position = raw_lines(
                        self.data.clone(),
                        self.expanded_idxs.clone(),
                        self.is_expanded,
                    )?
                    .len()
                    .saturating_sub(1);

                    if max_cursor_position > self.cursor_position {
                        self.cursor_position = self.cursor_position.saturating_add(1)
                    }
                }
            }
            Action::NavigateLeft(Some(_)) => {
                if self.is_active {
                    self.expanded_idxs.retain(|&x| x != self.cursor_position)
                }
            }
            Action::NavigateRight(Some(_)) => {
                if self.is_active {
                    let idx = self
                        .expanded_idxs
                        .partition_point(|&x| x < self.cursor_position);

                    // Expanding values above other expanded values
                    // pushes the currently expanded values down.
                    //
                    // Indices of the currently expanded values
                    // are increased by the size of
                    // the value that is being expanded.
                    if idx < self.expanded_idxs.len() {
                        let current_length =
                            raw_lines(self.data.clone(), vec![], self.is_expanded)?.len();

                        let next_length = raw_lines(
                            self.data.clone(),
                            vec![self.cursor_position],
                            self.is_expanded,
                        )?
                        .len();

                        self.expanded_idxs.insert(idx, self.cursor_position);
                        let cascade_len = next_length.saturating_sub(current_length);
                        let after_expanded_idxs =
                            self.expanded_idxs.split_off(idx.saturating_add(1));
                        let mut updated_idxs: Vec<usize> = after_expanded_idxs
                            .into_iter()
                            .map(|i| i.saturating_add(cascade_len))
                            .collect();

                        self.expanded_idxs.append(&mut updated_idxs);
                    } else {
                        self.expanded_idxs.insert(idx, self.cursor_position)
                    }
                }
            }
            Action::ExpandAll => {
                if self.is_active {
                    if !self.is_expanded {
                        self.is_expanded = true;
                        self.expanded_idxs.clear();
                        self.cursor_position = 0;
                    }
                }
            }
            Action::CollapseAll => {
                if self.is_active {
                    if self.is_expanded {
                        self.is_expanded = false;
                        self.expanded_idxs.clear();
                        self.cursor_position = 0;
                    }
                }
            }
            Action::SelectTrace(maybe_trace) => {
                if let Some(trace) = maybe_trace {
                    if ActiveBlock::RequestBody == self.active_block {
                        self.data = trace.http.clone().unwrap_or_default().request_body;
                        self.is_expanded = false;
                        self.expanded_idxs = vec![];
                    }
                    if ActiveBlock::ResponseBody == self.active_block {
                        self.data = trace.http.clone().unwrap_or_default().response_body;
                        self.is_expanded = false;
                        self.expanded_idxs = vec![];
                    }
                }
            }
            Action::ActivateBlock(current_active_block) => {
                self.is_active = current_active_block == self.active_block;
            }
            _ => {}
        }

        Ok(None)
    }

    pub fn render(&self, f: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
        let padding = Padding::zero();

        let outer_area = rect;

        let outer_block = Block::default()
            .borders(Borders::ALL)
            .padding(padding)
            .style(Style::default().fg(if self.is_active {
                Color::White
            } else {
                Color::DarkGray
            }))
            .title(self.title.to_string())
            .border_type(BorderType::Plain);

        let inner_area = outer_block.inner(outer_area);

        let inner_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(4), Constraint::Min(0)])
            .split(inner_area);

        let mut lines = json_to_lines(
            self.data.clone(),
            self.is_active,
            self.is_expanded,
            self.expanded_idxs.clone(),
            self.cursor_position,
        )?;

        let mut indent: usize = 0;
        for line in lines.iter_mut() {
            if line
                .spans
                .iter()
                .any(|s| s.content.ends_with('{') || s.content.ends_with("["))
            {
                line.spans.insert(0, Span::raw(" ".repeat(indent)));
                indent = indent.saturating_add(self.indent_spacing);
            } else if line.spans.iter().any(|s| {
                !s.content.contains("{..}")
                    && !s.content.contains("[..]")
                    && (s.content.ends_with("}")
                        || s.content.ends_with("},")
                        || s.content.ends_with("]")
                        || s.content.ends_with("],"))
            }) {
                indent = indent.saturating_sub(self.indent_spacing);
                line.spans.insert(0, Span::raw(" ".repeat(indent)));
            } else {
                line.spans.insert(0, Span::raw(" ".repeat(indent)));
            }
        }
        let mut line_indicators = vec![];
        for (idx, line) in lines.iter_mut().enumerate() {
            if idx == 0 {
                line_indicators.push(Line::from(vec![Span::styled(
                    "  ",
                    Style::default()
                        .fg(if self.is_active {
                            Color::White
                        } else {
                            Color::DarkGray
                        })
                        .add_modifier(Modifier::BOLD),
                )]));
            } else if line.spans.iter().any(|s| s.content.contains("{..}")) {
                line_indicators.push(Line::from(vec![Span::styled(
                    "˃ ",
                    Style::default()
                        .fg(if self.is_active {
                            Color::White
                        } else {
                            Color::DarkGray
                        })
                        .add_modifier(Modifier::BOLD),
                )]));
                continue;
            } else if line.spans.iter().any(|s| s.content.ends_with("{")) {
                line_indicators.push(Line::from(vec![Span::styled(
                    "˅ ",
                    Style::default()
                        .fg(if self.is_active {
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
                        .fg(if self.is_active {
                            Color::White
                        } else {
                            Color::DarkGray
                        })
                        .add_modifier(Modifier::BOLD),
                )]));
            }
        }

        let number_of_lines = lines.len();
        let available_height = inner_layout[1]
            .height
            .saturating_sub(RESPONSE_BODY_UNUSABLE_VERTICAL_SPACE.try_into()?);

        let json = Paragraph::new(lines)
            .style(
                Style::default()
                    .fg(if self.is_active {
                        Color::White
                    } else {
                        Color::DarkGray
                    })
                    .add_modifier(Modifier::BOLD),
            )
            .scroll((
                self.cursor_position
                    .saturating_sub(available_height.into())
                    .saturating_sub(1)
                    .try_into()?,
                0,
            ))
            .wrap(Wrap { trim: false });

        let line_indicators_paragraph = Paragraph::new(line_indicators)
            .alignment(Alignment::Right)
            .scroll((
                self.cursor_position
                    .saturating_sub(available_height.into())
                    .saturating_sub(1)
                    .try_into()?,
                0,
            ));

        f.render_widget(outer_block, outer_area);
        f.render_widget(json, inner_layout[1]);
        f.render_widget(line_indicators_paragraph, inner_layout[0]);

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

        Ok(())
    }
}

fn json_to_lines(
    maybe_data: Option<String>,
    is_active: bool,
    is_expanded: bool,
    expanded_idxs: Vec<usize>,
    cursor_position: usize,
) -> Result<Vec<Line<'static>>, Box<dyn Error>> {
    if is_active {
        active_lines(maybe_data, expanded_idxs, is_expanded, cursor_position)
    } else {
        raw_lines(maybe_data, expanded_idxs, is_expanded)
    }
}

fn active_lines(
    maybe_data: Option<String>,
    expanded_idxs: Vec<usize>,
    expanded: bool,
    cursor_position: usize,
) -> Result<Vec<Line<'static>>, Box<dyn Error>> {
    let mut lines = raw_lines(maybe_data, expanded_idxs, expanded)?;

    let style = Style::default().fg(Color::Green);

    if let Some(elem) = lines.get_mut(cursor_position) {
        elem.patch_style(style);
    } else if let Some(elem) = lines.last_mut() {
        elem.patch_style(style);
    }

    Ok(lines)
}

fn raw_lines(
    maybe_data: Option<String>,
    expanded_idxs: Vec<usize>,
    expanded: bool,
) -> Result<Vec<Line<'static>>, Box<dyn Error>> {
    let mut items = vec![];

    if let Some(data) = maybe_data {
        let v = serde_json::from_str(data.as_str())?;
        if let serde_json::Value::Object(o) = v {
            for line in obj_lines(o, &expanded_idxs, expanded, None, 0)? {
                items.push(line);
            }
        } else {
            let as_str: String = value_to_string(v)?;
            items.push(Line::raw(as_str));
        }
    }

    Ok(items)
}

fn value_to_string(v: serde_json::Value) -> Result<String, serde_json::Error> {
    match v {
        serde_json::Value::Array(_) => Ok("[..]".to_string()),
        serde_json::Value::Object(_) => Ok("{..}".to_string()),
        _ => Ok(v.to_string()),
    }
}

fn array_lines(
    v: Vec<serde_json::Value>,
    key: Option<String>,
) -> Result<Vec<Line<'static>>, Box<dyn Error>> {
    let mut items = vec![];

    if let Some(k) = key {
        items.push(Line::raw(format!(r#""{key}": ["#, key = k,)))
    } else {
        items.push(Line::raw("["));
    }

    for (_idx, item) in v.into_iter().enumerate() {
        items.push(Line::raw(value_to_string(item)?));
    }
    items.push(Line::raw("]"));

    Ok(items)
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
    let len = v.len();

    if let Some(k) = key {
        items.push(Line::raw(format!(r#""{key}": {{"#, key = k,)))
    } else {
        items.push(Line::raw("{"));
    }
    idx += 1;

    for (obj_idx, (k, v)) in v.into_iter().enumerate() {
        match v.clone() {
            serde_json::Value::Object(o) => {
                if expand_all_objects || expanded_idxs.contains(&idx) {
                    let lines = obj_lines(o, expanded_idxs, expand_all_objects, Some(k), idx)?;
                    let mut lineiter = lines.iter().peekable();
                    while let Some(lineref) = lineiter.next() {
                        let mut line = lineref.clone();
                        if let Some(span) = line.spans.last_mut() {
                            // Opening lines never end with commas
                            if !span.content.ends_with("{")
                                && !span.content.ends_with("[")
                                // Lines with commas never need additional commas
                                && !span.content.ends_with(",")
                            {
                                match lineiter.peek() {
                                    // If the next line is not the ending line, append a comma
                                    Some(next_line) => {
                                        if let Some(next_span) = next_line.spans.last() {
                                            if next_span.content.ends_with("{..}")
                                                || next_span.content.ends_with("[..]")
                                                || (!next_span.content.ends_with("}")
                                                    && !next_span.content.ends_with("},")
                                                    && !next_span.content.ends_with("]")
                                                    && !next_span.content.ends_with("],"))
                                            {
                                                *span = Span::raw(String::from(
                                                    span.content.clone() + ",",
                                                ));
                                            }
                                        }
                                    }
                                    // This is the last line of the object: `}`;
                                    // unless this is the last value in the parent object
                                    // append a comma
                                    None => {
                                        if obj_idx < len.saturating_sub(1) {
                                            *span =
                                                Span::raw(String::from(span.content.clone() + ","));
                                        }
                                    }
                                }
                            }
                        }

                        items.push(line);
                        idx += 1;
                    }
                } else {
                    let obj_as_str: String = value_to_string(v.clone())?;
                    if idx < len {
                        items.push(Line::raw(format!(
                            r#""{key}": {value},"#,
                            key = k,
                            value = obj_as_str,
                        )));
                    } else {
                        items.push(Line::raw(format!(
                            r#""{key}": {value}"#,
                            key = k,
                            value = obj_as_str,
                        )));
                    }
                    idx += 1;
                }
            }
            serde_json::Value::Array(a) => {
                if expand_all_objects || expanded_idxs.contains(&idx) {
                    let lines = array_lines(a, Some(k))?;
                    let mut lineiter = lines.iter().peekable();
                    while let Some(lineref) = lineiter.next() {
                        let mut line = lineref.clone();
                        if let Some(span) = line.spans.last_mut() {
                            if !span.content.ends_with("{")
                                && !span.content.ends_with("[")
                                && !span.content.ends_with(",")
                            {
                                match lineiter.peek() {
                                    Some(next_line) => {
                                        if let Some(next_span) = next_line.spans.last() {
                                            if next_span.content.ends_with("{..}")
                                                || next_span.content.ends_with("[..]")
                                                || (!next_span.content.ends_with("}")
                                                    && !next_span.content.ends_with("},")
                                                    && !next_span.content.ends_with("]")
                                                    && !next_span.content.ends_with("],"))
                                            {
                                                *span = Span::raw(String::from(
                                                    span.content.clone() + ",",
                                                ));
                                            }
                                        }
                                    }
                                    // This is the last line of the array: `]`;
                                    // unless this is the last value in the parent object
                                    // append a comma
                                    None => {
                                        if obj_idx < len.saturating_sub(1) {
                                            *span =
                                                Span::raw(String::from(span.content.clone() + ","));
                                        }
                                    }
                                }
                            }
                        }

                        items.push(line);
                        idx += 1;
                    }
                } else {
                    let array_as_str: String = value_to_string(v.clone())?;
                    if idx < len {
                        items.push(Line::raw(format!(
                            r#""{key}": {value},"#,
                            key = k,
                            value = array_as_str,
                        )));
                    } else {
                        items.push(Line::raw(format!(
                            r#""{key}": {value}"#,
                            key = k,
                            value = array_as_str,
                        )));
                    }
                    idx += 1;
                }
            }
            _ => {
                let value_as_str: String = value_to_string(v.clone())?;
                if idx < len {
                    items.push(Line::raw(format!(
                        r#""{key}": {value},"#,
                        key = k,
                        value = value_as_str,
                    )));
                } else {
                    items.push(Line::raw(format!(
                        r#""{key}": {value}"#,
                        key = k,
                        value = value_as_str,
                    )));
                }
                idx += 1;
            }
        }
    }

    items.push(Line::raw("}"));

    Ok(items)
}

#[cfg(test)]
mod tests {
    use crate::components::jsonviewer;
    use pretty_assertions::assert_eq;
    use ratatui::prelude::{Color, Line, Style};
    use std::error::Error;

    #[test]
    fn test_raw_empty() -> Result<(), Box<dyn Error>> {
        let result = jsonviewer::raw_lines(None, vec![], true)?;

        let expected: Vec<Line> = vec![];

        assert_eq!(expected, result);

        Ok(())
    }

    #[test]
    fn test_raw_object() -> Result<(), Box<dyn Error>> {
        let input = serde_json::json!({
            "code": 200,
        });

        let result = jsonviewer::raw_lines(Some(input.to_string()), vec![], true)?;

        assert_eq!(
            vec![Line::raw("{"), Line::raw("\"code\": 200"), Line::raw("}"),],
            result
        );

        Ok(())
    }

    #[test]
    fn test_raw_non_object_value() -> Result<(), Box<dyn Error>> {
        let input = serde_json::json!(200);

        let result = jsonviewer::raw_lines(Some(input.to_string()), vec![], true)?;

        assert_eq!(vec![Line::raw("200")], result);

        Ok(())
    }

    #[test]
    fn test_active_styles() -> Result<(), Box<dyn Error>> {
        let input = serde_json::json!({
            "code": 200,
        });

        let result = jsonviewer::active_lines(Some(input.to_string()), vec![], true, 1)?;

        let default_style = Style::default();
        let selected_style = Style::default().fg(Color::Green);

        assert_eq!(
            vec![
                Line::styled("{", default_style),
                Line::styled("\"code\": 200", selected_style),
                Line::styled("}", default_style),
            ],
            result
        );

        Ok(())
    }

    #[test]
    fn test_simple() -> Result<(), Box<dyn Error>> {
        let input = serde_json::json!({
            "one": {
                "a": 1,
                "b": 2
            },
            "two": [
                3,
                4
            ]
        });

        let result =
            jsonviewer::obj_lines(input.as_object().unwrap().clone(), &vec![], false, None, 0)?;

        assert_eq!(
            vec![
                Line::raw("{"),
                Line::raw("\"one\": {..},"),
                Line::raw("\"two\": [..]"),
                Line::raw("}"),
            ],
            result,
        );

        Ok(())
    }

    #[test]
    fn test_simple_objects() -> Result<(), Box<dyn Error>> {
        let input = serde_json::json!({
            "one": {
                "a": 1,
                "b": 2
            },
            "two": {
                "c": 3,
                "d": 4
            }
        });

        let result =
            jsonviewer::obj_lines(input.as_object().unwrap().clone(), &vec![], false, None, 0)?;

        assert_eq!(
            vec![
                Line::raw("{"),
                Line::raw("\"one\": {..},"),
                Line::raw("\"two\": {..}"),
                Line::raw("}"),
            ],
            result,
        );

        Ok(())
    }

    #[test]
    fn test_simple_arrays() -> Result<(), Box<dyn Error>> {
        let input = serde_json::json!({
            "one": [
                1,
                2
            ],
            "two": [
                3,
                4
            ]
        });

        let result =
            jsonviewer::obj_lines(input.as_object().unwrap().clone(), &vec![], false, None, 0)?;

        assert_eq!(
            vec![
                Line::raw("{"),
                Line::raw("\"one\": [..],"),
                Line::raw("\"two\": [..]"),
                Line::raw("}"),
            ],
            result,
        );

        Ok(())
    }

    #[test]
    fn test_all_expanded() -> Result<(), Box<dyn Error>> {
        let input = serde_json::json!({
            "one": {
                "a": 1,
                "b": 2
            },
            "two": [
                3,
                4
            ],
            "three": {
                "c": 5,
                "d": 6
            },
            "four": [
                7,
                8
            ]
        });

        let result =
            jsonviewer::obj_lines(input.as_object().unwrap().clone(), &vec![], true, None, 0)?;

        assert_eq!(
            vec![
                Line::raw("{"),
                Line::raw("\"one\": {"),
                Line::raw("\"a\": 1,"),
                Line::raw("\"b\": 2"),
                Line::raw("},"),
                Line::raw("\"two\": ["),
                Line::raw("3,"),
                Line::raw("4"),
                Line::raw("],"),
                Line::raw("\"three\": {"),
                Line::raw("\"c\": 5,"),
                Line::raw("\"d\": 6"),
                Line::raw("},"),
                Line::raw("\"four\": ["),
                Line::raw("7,"),
                Line::raw("8"),
                Line::raw("]"),
                Line::raw("}"),
            ],
            result,
        );

        Ok(())
    }

    #[test]
    fn test_all_expanded_deep() -> Result<(), Box<dyn Error>> {
        let input = serde_json::json!({
            "one": {
                "a": 1,
                "b": 2,
                "two": {
                    "c": 3,
                    "d": 4
                },
                "three": [
                    5,
                    6
                ]
            }
        });

        let result =
            jsonviewer::obj_lines(input.as_object().unwrap().clone(), &vec![], true, None, 0)?;

        assert_eq!(
            vec![
                Line::raw("{"),
                Line::raw("\"one\": {"),
                Line::raw("\"a\": 1,"),
                Line::raw("\"b\": 2,"),
                Line::raw("\"two\": {"),
                Line::raw("\"c\": 3,"),
                Line::raw("\"d\": 4"),
                Line::raw("},"),
                Line::raw("\"three\": ["),
                Line::raw("5,"),
                Line::raw("6"),
                Line::raw("]"),
                Line::raw("}"),
                Line::raw("}"),
            ],
            result,
        );

        Ok(())
    }

    #[test]
    fn test_expanded_by_index() -> Result<(), Box<dyn Error>> {
        let input = serde_json::json!({
            "one": {
              "a": 1,
              "b": 2
            },
           "two": {
              "c": 3,
              "d": 4
           },
           "three": {
              "e": 5,
              "f": 6
           }
        });

        let result =
            jsonviewer::obj_lines(input.as_object().unwrap().clone(), &vec![2], false, None, 0)?;

        assert_eq!(
            vec![
                Line::raw("{"),
                Line::raw("\"one\": {..},"),
                Line::raw("\"two\": {"),
                Line::raw("\"c\": 3,"),
                Line::raw("\"d\": 4"),
                Line::raw("},"),
                Line::raw("\"three\": {..}"),
                Line::raw("}"),
            ],
            result,
        );

        Ok(())
    }

    #[test]
    fn test_expanded_deep_by_index() -> Result<(), Box<dyn Error>> {
        let input = serde_json::json!({
            "one": {
                "a": 1,
                "b": 2,
                "two": {
                    "c": 3,
                    "d": 4
                },
                "three": [
                    5,
                    6
                ]
            }
        });

        let result =
            jsonviewer::obj_lines(input.as_object().unwrap().clone(), &vec![1], false, None, 0)?;

        assert_eq!(
            vec![
                Line::raw("{"),
                Line::raw("\"one\": {"),
                Line::raw("\"a\": 1,"),
                Line::raw("\"b\": 2,"),
                Line::raw("\"two\": {..},"),
                Line::raw("\"three\": [..]"),
                Line::raw("}"),
                Line::raw("}"),
            ],
            result,
        );

        Ok(())
    }
}
