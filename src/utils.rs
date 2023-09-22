use http::Uri;

use crate::app::{App, Request};

pub enum UIDispatchEvent {
    ClearStatusMessage,
}

// NOTE: [stackoverflow](https://stackoverflow.com/questions/38461429/how-can-i-truncate-a-string-to-have-at-most-n-characters)
pub fn truncate(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        None => s.to_owned(),
        Some((idx, _)) => {
            let mut e = s[..idx].to_owned();

            let _ = &e.push('.');

            e.clone()
        }
    }
}

pub fn parse_query_params(url: String) -> Vec<(String, String)> {
    let uri = url.parse::<Uri>();

    match uri {
        Ok(value) => match value.query().map(|v| (v).split("&")) {
            Some(v) => v
                .map(|query_param_entry| {
                    let query_param_entry_in_vector =
                        query_param_entry.split("=").collect::<Vec<&str>>();

                    (
                        String::from(query_param_entry_in_vector[0]),
                        String::from(query_param_entry_in_vector[1]),
                    )
                })
                .collect(),
            _ => vec![],
        },
        Err(_) => vec![],
    }
}

pub fn get_currently_selected_request(app: &App) -> Option<&Request> {
    let items_as_vector = app.items.iter().collect::<Vec<&Request>>();

    items_as_vector.get(app.main.index).copied()
}
