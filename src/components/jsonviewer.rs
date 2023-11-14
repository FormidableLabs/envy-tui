use std::error::Error;

use ratatui::prelude::*;
use ratatui::widgets::block::Padding;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct JSONViewer {
    value: serde_json::Value,
    testvalue: serde_json::Value,
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

        Ok(Self { value, testvalue })
    }

    fn lines(&mut self) -> Vec<Line> {
        let mut items = vec![];
        // TODO: convert this to a while statement
        if let serde_json::Value::Object(o) = &self.testvalue {
            items.push(Line::raw("{"));

            let mut idx = 0;
            let len = o.len();

            for (k, v) in o.iter() {
                let as_str: String = match v {
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => format!(r#""{}""#, s),
                    serde_json::Value::Null => "null".to_string(),
                    serde_json::Value::Array(a) => format!("{:?}", a),
                    _ => "{}".to_string(),
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
        } else {
            items.push(Line::raw(self.testvalue.to_string()));
        }

        items
    }

    pub fn render(&mut self, f: &mut Frame, rect: Rect) {
        let padding = Padding::zero();
        let widget = Paragraph::new(self.lines())
            .block(Block::default().borders(Borders::NONE).padding(padding))
            .style(Style::default().fg(Color::White));
        f.render_widget(widget, rect);
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
