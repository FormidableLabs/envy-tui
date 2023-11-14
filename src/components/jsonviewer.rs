use std::error::Error;
use std::io;
use std::io::Write;

use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::*;
use ratatui::widgets::block::Padding;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use serde::Serialize;
use serde_json::json;
use serde_json::ser::{Formatter, PrettyFormatter};

#[derive(Default)]
struct JSONFormatter {
    pretty: PrettyFormatter<'static>,
    depth: usize,
}

impl Formatter for JSONFormatter {
    fn begin_array<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.pretty.begin_array(w)
    }
    fn end_array<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.pretty.end_array(w)
    }
    fn begin_array_value<W: ?Sized + Write>(&mut self, w: &mut W, first: bool) -> io::Result<()> {
        self.pretty.begin_array_value(w, first)
    }
    fn end_array_value<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.pretty.end_array_value(w)
    }
    fn begin_object<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.depth += 1;
        self.pretty.begin_object(w)
    }
    fn end_object<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.pretty.end_object(w).and_then(|()| {
            self.depth -= 1;
            if self.depth == 0 {
                w.write_all(b"\n~~~\n")
            } else {
                Ok(())
            }
        })
    }
    fn begin_object_key<W: ?Sized + Write>(&mut self, w: &mut W, first: bool) -> io::Result<()> {
        self.pretty.begin_object_key(w, first)
    }
    fn begin_object_value<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.pretty.begin_object_value(w)
    }
    fn end_object_value<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.pretty.end_object_value(w)
    }
}

pub struct JSONViewer {
    value: serde_json::Value,
    testvalue: serde_json::Value,
    serializer: serde_json::Serializer<io::Stdout, JSONFormatter>,
}

impl JSONViewer {
    pub fn new(data: &str) -> Result<Self, Box<dyn Error>> {
        let value = serde_json::from_str(data)?;
        let data = r#"
        {
            "name": "John Doe",
            "empty": null,
            "boolean_a": true,
            "boolean_b": false,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ],
            "age": 43,
            "nested": {
              "name": "Jane Doe"
            }
        }"#;
        let testvalue = serde_json::from_str(data)?;
        let formatter = JSONFormatter::default();
        let serializer = serde_json::Serializer::with_formatter(io::stdout(), formatter);

        Ok(Self {
            value,
            testvalue,
            serializer,
        })
    }

    pub fn render(&mut self, f: &mut Frame, rect: Rect) {
        //let thing = json!(self.testvalue);
        //let value = thing.serialize(&mut self.serializer)?;

        // let mut idx = 0;
        let padding = Padding::zero();
        let line_height = ListItem::new("{").height() as u16;
        // let available_height = rect.height.clone();
        let mut constraints = vec![];
        let mut widgets = vec![];
        // TODO: convert this to a while statement
        if let serde_json::Value::Object(o) = &self.testvalue {
            let object_padding = padding.clone();

            let items = vec![Line::raw("{")];
            constraints.push(Constraint::Length(line_height));
            let widget = Paragraph::new(items)
                .block(
                    Block::default()
                        .borders(Borders::NONE)
                        .padding(object_padding),
                )
                .style(Style::default().fg(Color::White));
            widgets.push(widget);

            let mut list = vec![];
            let mut inner_padding = padding.clone();
            let mut idx = 0;
            let len = o.len();

            inner_padding.left += 5;
            for (k, v) in o.iter() {
                let as_str: String = match v {
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => format!(r#""{}""#, s),
                    serde_json::Value::Null => "null".to_string(),
                    serde_json::Value::Array(a) => format!("{:?}", a),
                    _ => "{}".to_string(),
                };
                list.push(Line::from(vec![
                    r#"""#.into(),
                    k.into(),
                    r#"""#.into(),
                    ": ".into(),
                    as_str.into(),
                    if idx < len - 1 { ",".into() } else { "".into() },
                ]));
                idx += 1;
            }

            constraints.push(Constraint::Length(list.len() as u16 * line_height));
            let widget = Paragraph::new(list)
                .block(
                    Block::default()
                        .borders(Borders::NONE)
                        .padding(inner_padding),
                )
                .style(Style::default().fg(Color::White));
            widgets.push(widget);

            constraints.push(Constraint::Length(line_height));
            let widget = Paragraph::new(vec![Line::raw("}")])
                .block(
                    Block::default()
                        .borders(Borders::NONE)
                        .padding(object_padding),
                )
                .style(Style::default().fg(Color::White));
            widgets.push(widget);
        } else {
            let mut list = vec![];
            list.push(Line::raw(self.testvalue.to_string()));
            let widget = Paragraph::new(list)
                .block(Block::default().borders(Borders::NONE).padding(padding))
                .style(Style::default().fg(Color::White));
            f.render_widget(widget, rect);
        }

        constraints.push(Constraint::Min(0));

        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(rect);

        let mut idx = 0;
        for widget in widgets {
            f.render_widget(widget, areas[idx]);
            idx += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::components::jsonviewer;

    #[test]
    fn exploration() {
        let data = r#"{ "foo": "bar" }"#;
        let result = jsonviewer::JSONViewer::new(data);
        assert!(result.is_ok());
    }
}
