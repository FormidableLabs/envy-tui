use http::Uri;

use crate::app::{App, Trace};

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

pub fn get_currently_selected_request(app: &App) -> Option<&Trace> {
    let items_as_vector = app.items.iter().collect::<Vec<&Trace>>();

    items_as_vector.get(app.main.index).copied()
}

pub fn calculate_scrollbar_position(
    content_length: u16,
    offset: usize,
    overflown_number_count: u16,
) -> u16 {
    let content_length_as_float = content_length as f32;
    let overflown_number_count_as_float = overflown_number_count as f32;
    let offset_as_float = offset as f32;

    ({ (content_length_as_float / overflown_number_count_as_float) * offset_as_float } as u16)
}

pub struct ContentLength {
    pub vertical: u16,
    pub horizontal: u16,
}

pub fn get_content_length(app: &App) -> (Option<ContentLength>, Option<ContentLength>) {
    let trace = get_currently_selected_request(&app);

    if trace.is_none() {
        return (None, None);
    }

    let item = trace.unwrap();

    let response_lines = &item.pretty_response_body.as_ref();

    let request_lines = &item.pretty_request_body.as_ref();

    match (response_lines, request_lines) {
        (Some(response_lines), Some(request_lines)) => {
            let response_longest =
                response_lines
                    .lines()
                    .into_iter()
                    .fold(0, |longest: u16, lines: &str| {
                        let len = lines.len() as u16;

                        len.max(longest)
                    });

            let request_longest =
                request_lines
                    .lines()
                    .into_iter()
                    .fold(0, |longest: u16, lines: &str| {
                        let len = lines.len() as u16;

                        len.max(longest)
                    });

            let response_vertical_content_length: u16 = response_lines
                .lines()
                .into_iter()
                .collect::<Vec<_>>()
                .len()
                .try_into()
                .unwrap();

            let request_vertical_content_length: u16 = request_lines
                .lines()
                .into_iter()
                .collect::<Vec<_>>()
                .len()
                .try_into()
                .unwrap();
            (
                Some(ContentLength {
                    vertical: request_vertical_content_length,
                    horizontal: request_longest,
                }),
                Some(ContentLength {
                    vertical: response_vertical_content_length,
                    horizontal: response_longest,
                }),
            )
        }
        (Some(response_lines), None) => {
            let response_longest =
                response_lines
                    .lines()
                    .into_iter()
                    .fold(0, |longest: u16, lines: &str| {
                        let len = lines.len() as u16;

                        len.max(longest)
                    });

            let response_vertical_content_length: u16 = response_lines
                .lines()
                .into_iter()
                .collect::<Vec<_>>()
                .len()
                .try_into()
                .unwrap();

            (
                None,
                Some(ContentLength {
                    vertical: response_vertical_content_length,
                    horizontal: response_longest,
                }),
            )
        }
        (None, Some(request_lines)) => {
            let request_longest =
                request_lines
                    .lines()
                    .into_iter()
                    .fold(0, |longest: u16, lines: &str| {
                        let len = lines.len() as u16;

                        len.max(longest)
                    });

            let request_vertical_content_length: u16 = request_lines
                .lines()
                .into_iter()
                .collect::<Vec<_>>()
                .len()
                .try_into()
                .unwrap();
            (
                Some(ContentLength {
                    vertical: request_vertical_content_length,
                    horizontal: request_longest,
                }),
                None,
            )
        }
        _ => (None, None),
    }
}

pub fn set_content_length(app: &mut App) {
    let (req, res) = get_content_length(&app);

    if res.is_some() {
        let res = res.unwrap();

        app.response_body.scroll_state =
            app.response_body.scroll_state.content_length(res.vertical);

        app.response_body.horizontal_scroll_state = app
            .response_body
            .horizontal_scroll_state
            .content_length(res.horizontal);
    }

    if req.is_some() {
        let req = req.unwrap();

        app.request_body.scroll_state = app.request_body.scroll_state.content_length(req.vertical);

        app.request_body.horizontal_scroll_state = app
            .request_body
            .horizontal_scroll_state
            .content_length(req.horizontal);
    }
}
