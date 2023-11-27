use std::convert::TryFrom;
use std::error::Error;

use ratatui::prelude::*;
use ratatui::widgets::block::Padding;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::Action;

#[derive(Default)]
pub struct JSONViewer {
    pub action_tx: Option<UnboundedSender<Action>>,
    expanded: bool,
    expanded_idxs: Vec<usize>,
    indent_spacing: usize,
    cursor_position: usize,
}

impl JSONViewer {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            indent_spacing: 4,
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
            Action::CollapseAll => self.expanded = false,
            _ => {}
        }

        Ok(None)
    }

    pub fn render(
        &self,
        f: &mut Frame,
        rect: Rect,
        data: String,
        active: bool,
    ) -> Result<(), Box<dyn Error>> {
        // let copy = if request.duration.is_some() {
        //     "This trace does not have a response body."
        // } else {
        //     "Loading..."
        // };

        let padding = Padding::zero();

        let mut lines = match active {
            true => active_lines(
                data,
                self.expanded_idxs.clone(),
                self.expanded,
                self.cursor_position,
            )?,
            false => raw_lines(data, self.expanded_idxs.clone(), self.expanded)?,
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

        let outer_block = Block::default()
            .borders(Borders::ALL)
            .padding(padding)
            .style(Style::default().fg(if active {
                Color::White
            } else {
                Color::DarkGray
            }))
            .title("Request body")
            .border_type(BorderType::Plain);

        let outer_area = rect;
        let inner_area = outer_block.inner(outer_area);

        let inner_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(4), Constraint::Min(0)])
            .split(inner_area);

        let mut line_counters = vec![];
        let line_count = lines.len() + 1;
        for n in 1..line_count {
            line_counters.push(Line::from(vec![Span::styled(
                format!(
                    "{:>width$}",
                    n,
                    // https://stackoverflow.com/questions/43704758/how-to-idiomatically-convert-between-u32-and-usize
                    width = 2 + usize::try_from(line_count.checked_ilog(10).unwrap_or(2))?
                ),
                Style::new().yellow().on_light_red(),
            )]));
        }

        let json = Paragraph::new(lines).style(
            Style::default()
                .fg(if active {
                    Color::White
                } else {
                    Color::DarkGray
                })
                .add_modifier(Modifier::BOLD),
        );

        f.render_widget(outer_block, outer_area);

        let line_count_paragraph = Paragraph::new(line_counters).alignment(Alignment::Right);

        f.render_widget(line_count_paragraph, inner_layout[0]);
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
    let value = serde_json::from_str(data.as_str())?;
    let mut items = vec![];
    let mut idx = 0;

    // TODO: Display raw, non-object top-level json values which are also valid
    if let serde_json::Value::Object(o) = value {
        items.push(Line::raw("{"));
        idx += 1;

        let len = o.len();

        // TODO: why does this require an into_iter() vs iter() call?
        for (k, v) in o.into_iter() {
            if let serde_json::Value::Object(o) = v {
                if expanded || expanded_idxs.contains(&idx) {
                    let as_str = "{".to_string();
                    items.push(Line::from(vec![
                        r#"""#.into(),
                        k.into(),
                        r#"""#.into(),
                        ": ".into(),
                        as_str.into(),
                        if idx < len.saturating_sub(1) {
                            ",".into()
                        } else {
                            "".into()
                        },
                    ]));
                    idx += 1;
                    for line in obj_lines(o)? {
                        items.push(line);
                        idx += 1;
                    }
                } else {
                    let as_str = "{..}".to_string();
                    items.push(Line::from(vec![
                        r#"""#.into(),
                        k.into(),
                        r#"""#.into(),
                        ": ".into(),
                        as_str.into(),
                        if idx < len.saturating_sub(1) {
                            ",".into()
                        } else {
                            "".into()
                        },
                    ]));
                    idx += 1;
                }
            } else {
                let as_str: String = match v {
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => format!(r#""{}""#, s),
                    serde_json::Value::Null => "null".to_string(),
                    serde_json::Value::Array(a) => format!("{:?}", a),
                    serde_json::Value::Object(_) => "{..}".to_string(),
                };
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

        items.push(Line::raw("}"));
    }

    Ok(items)
}

fn obj_lines(
    value: serde_json::Map<String, serde_json::Value>,
) -> Result<Vec<Line<'static>>, Box<dyn Error>> {
    let mut items = vec![];
    let mut idx = 0;
    let len = value.len();

    for (k, v) in value.into_iter() {
        let as_str: String = match v {
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => format!(r#""{}""#, s),
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Array(a) => format!("{:?}", a),
            serde_json::Value::Object(_) => "{..}".to_string(),
        };
        items.push(Line::from(vec![
            r#"""#.into(),
            k.into(),
            r#"""#.into(),
            ": ".into(),
            as_str.into(),
            if idx < len - 1 { ",".into() } else { "".into() },
        ]));
        idx += 1;
    }

    items.push(Line::raw("}"));

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
