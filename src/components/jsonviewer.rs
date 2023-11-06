use serde_json::json;
use std::error::Error;

use ratatui::prelude::*;
use ratatui::Frame;

#[derive(Default)]
pub struct JSONViewer {
    value: serde_json::Value,
}

impl JSONViewer {
    fn new(data: &str) -> Result<Self, Box<dyn Error>> {
        let value = serde_json::from_str(data)?;

        Ok(Self { value })
    }

    fn draw(&mut self, f: &mut Frame, rect: Rect) {
        let john = json!({
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        });
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
