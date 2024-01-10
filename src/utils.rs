use core::str::FromStr;
use http::Uri;
use regex::Regex;

use crate::app::{SortOrder, SortSource, TraceSort};
use crate::components::home::{FilterSource, Home};
use crate::services::websocket::Trace;

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
        Ok(value) => match value.query().map(|v| (v).split('&')) {
            Some(v) => v
                .map(|query_param_entry| {
                    let query_param_entry_in_vector =
                        query_param_entry.split('=').collect::<Vec<&str>>();

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

fn fuzzy_regex(query: String) -> Regex {
    let mut fuzzy_query = String::new();

    for c in query.chars() {
        fuzzy_query.extend([c, '.', '*']);
    }

    return Regex::from_str(&fuzzy_query).unwrap();
}

pub fn get_rendered_items(app: &Home) -> Vec<&Trace> {
    let mut maybe_re: Option<Regex> = None;
    if !app.search_query.is_empty() {
        maybe_re = Some(fuzzy_regex(app.search_query.clone()));
    }

    let no_applied_method_filter = app
        .method_filters
        .iter()
        .filter(|(_key, method_filter)| method_filter.selected == true)
        .collect::<Vec<_>>()
        .is_empty();

    let no_applied_statud_filter = app
        .status_filters
        .iter()
        .filter(|(_key, method_filter)| method_filter.selected == true)
        .collect::<Vec<_>>()
        .is_empty();

    let mut items_as_vector = app
        .items
        .iter()
        .filter(|trace| {
            if let Some(re) = &maybe_re {
                re.is_match(&trace.http.as_ref().unwrap().uri)
            } else {
                true
            }
        })
        .filter(
            |trace| match (app.get_filter_source(), trace.service_name.as_ref()) {
                (FilterSource::All, _) => true,
                (FilterSource::Applied(sources), Some(trace_source)) => {
                    sources.contains(trace_source)
                }
                _ => false,
            },
        )
        .filter(|trace| {
            let method = &trace.http.as_ref().unwrap().status;

            if method.is_none() {
                return true;
            }

            let method_as_string = method.as_ref().unwrap().clone().as_u16().to_string();

            let first_char = method_as_string.chars().nth(0).unwrap();

            let matcher = match first_char {
                '1' => "1xx",
                '2' => "2xx",
                '3' => "3xx",
                '4' => "4xx",
                '5' => "5xx",
                _ => "",
            };

            match (no_applied_statud_filter, app.status_filters.get(matcher)) {
                (true, _) => true,
                (_, Some(status_filter)) => status_filter.selected.clone(),
                (_, _) => false,
            }
        })
        .filter(|trace| {
            match (
                no_applied_method_filter,
                app.method_filters.get(&trace.http.as_ref().unwrap().method),
            ) {
                (true, _) => true,
                (_, Some(method_filter)) => method_filter.selected.clone(),
                (_, _) => false,
            }
        })
        .collect::<Vec<&Trace>>();

    items_as_vector.sort_by(|a, b| match &app.sort {
        TraceSort {
            source: SortSource::Duration,
            order: SortOrder::Ascending,
        } => a
            .http
            .as_ref()
            .unwrap()
            .duration
            .unwrap_or(0)
            .cmp(&b.http.as_ref().unwrap().duration.unwrap_or(0)),
        TraceSort {
            source: SortSource::Duration,
            order: SortOrder::Descending,
        } => b
            .http
            .as_ref()
            .unwrap()
            .duration
            .unwrap_or(0)
            .cmp(&a.http.as_ref().unwrap().duration.unwrap_or(0)),
        TraceSort {
            source: SortSource::Timestamp,
            order: SortOrder::Ascending,
        } => a.timestamp.cmp(&b.timestamp),
        TraceSort {
            source: SortSource::Timestamp,
            order: SortOrder::Descending,
        } => b.timestamp.cmp(&a.timestamp),
        TraceSort {
            source: SortSource::Status,
            order: SortOrder::Descending,
        } => {
            let a_has = a.http.as_ref().unwrap().status.is_some();
            let b_has = b.http.as_ref().unwrap().status.is_some();

            if a_has && b_has {
                b.http
                    .as_ref()
                    .unwrap()
                    .status
                    .unwrap()
                    .as_u16()
                    .cmp(&a.http.as_ref().unwrap().status.unwrap().as_u16())
            } else if a_has {
                std::cmp::Ordering::Less
            } else if b_has {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }
        TraceSort {
            source: SortSource::Status,
            order: SortOrder::Ascending,
        } => {
            let a_has = a.http.as_ref().unwrap().status.is_some();
            let b_has = b.http.as_ref().unwrap().status.is_some();

            if a_has && b_has {
                a.http
                    .as_ref()
                    .unwrap()
                    .status
                    .unwrap()
                    .as_u16()
                    .cmp(&b.http.as_ref().unwrap().status.unwrap().as_u16())
            } else if a_has {
                std::cmp::Ordering::Less
            } else if b_has {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }
        TraceSort {
            source: SortSource::Url,
            order: SortOrder::Descending,
        } => {
            let url = &a.http.as_ref().unwrap().uri;
            let urlb = &b.http.as_ref().unwrap().uri;

            url.cmp(&urlb)
        }
        TraceSort {
            source: SortSource::Url,
            order: SortOrder::Ascending,
        } => a.timestamp.cmp(&b.timestamp),
        TraceSort {
            source: SortSource::Method,
            order: SortOrder::Ascending,
        } => {
            let a_has = a.http.as_ref().unwrap().method.to_string();
            let b_has = b.http.as_ref().unwrap().method.to_string();

            a_has.cmp(&b_has)
        }
        TraceSort {
            source: SortSource::Method,
            order: SortOrder::Descending,
        } => {
            let a_has = a.http.as_ref().unwrap().method.to_string();
            let b_has = b.http.as_ref().unwrap().method.to_string();

            b_has.cmp(&a_has)
        }
        TraceSort {
            source: SortSource::Source,
            order: SortOrder::Ascending,
        } => {
            let a_has = &a.service_name;
            let b_has = &b.service_name;

            a_has.cmp(&b_has)
        }
        TraceSort {
            source: SortSource::Source,
            order: SortOrder::Descending,
        } => {
            let a_has = &a.service_name;
            let b_has = &b.service_name;

            b_has.cmp(&a_has)
        }
    });

    items_as_vector
}

pub fn get_currently_selected_trace(app: &Home) -> Option<Trace> {
    let items_as_vector = get_rendered_items(app);

    let trace = items_as_vector.get(app.main.index).copied();

    trace.cloned()
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

pub struct ContentLengthElements {
    pub request_headers: ContentLength,
    pub response_headers: Option<ContentLength>,
    pub request_body: Option<ContentLength>,
    pub response_body: Option<ContentLength>,
}

pub fn get_content_length(app: &Home) -> ContentLengthElements {
    let mut content_length = ContentLengthElements {
        request_body: None,
        response_body: None,
        response_headers: None,
        request_headers: ContentLength {
            vertical: 0,
            horizontal: 0,
        },
    };

    if app.selected_trace.is_none() {
        return content_length;
    }

    let http_trace = app.selected_trace.clone().unwrap_or_default().http;

    if http_trace.is_none() {
        return content_length;
    }

    let item = http_trace.unwrap();

    if !item.response_headers.is_empty() {
        content_length.response_headers = Some(ContentLength {
            vertical: item.response_headers.len() as u16,
            horizontal: 0,
        })
    }

    if !item.request_headers.is_empty() {
        content_length.request_headers.vertical = item.request_headers.len() as u16;
    }

    let response_lines = &item.pretty_response_body.as_ref();

    let request_lines = &item.pretty_request_body.as_ref();

    if response_lines.is_some() {
        let response_lines = response_lines.unwrap();

        let response_longest = response_lines.lines().fold(0, |longest: u16, lines: &str| {
            let len = lines.len() as u16;

            len.max(longest)
        });

        let response_vertical_content_length: u16 = response_lines
            .lines()
            .collect::<Vec<_>>()
            .len()
            .try_into()
            .unwrap();

        content_length.response_body = Some(ContentLength {
            vertical: response_vertical_content_length,
            horizontal: response_longest,
        });
    }

    if request_lines.is_some() {
        let request_lines = request_lines.unwrap();

        let request_longest = request_lines.lines().fold(0, |longest: u16, lines: &str| {
            let len = lines.len() as u16;

            len.max(longest)
        });

        let request_vertical_content_length: u16 = request_lines
            .lines()
            .collect::<Vec<_>>()
            .len()
            .try_into()
            .unwrap();

        content_length.request_body = Some(ContentLength {
            vertical: request_vertical_content_length,
            horizontal: request_longest,
        });
    }

    content_length
}

pub fn set_content_length(app: &mut Home) {
    let content_length_elements = get_content_length(app);

    let response_details_content_length = content_length_elements
        .response_headers
        .unwrap_or(ContentLength {
            vertical: 0,
            horizontal: 0,
        })
        .vertical;

    let res = content_length_elements.response_body;

    app.request_details.scroll_state = app
        .request_details
        .scroll_state
        .content_length(content_length_elements.request_headers.vertical.into());

    app.response_details.scroll_state = app
        .response_details
        .scroll_state
        .content_length(response_details_content_length.into());

    if res.is_some() {
        let res = res.unwrap();

        app.response_body.scroll_state = app
            .response_body
            .scroll_state
            .content_length(res.vertical.into());

        app.response_body.horizontal_scroll_state = app
            .response_body
            .horizontal_scroll_state
            .content_length(res.horizontal.into());
    }

    let req = content_length_elements.request_body;

    if req.is_some() {
        let req = req.unwrap();

        app.request_body.scroll_state = app
            .request_body
            .scroll_state
            .content_length(req.vertical.into());

        app.request_body.horizontal_scroll_state = app
            .request_body
            .horizontal_scroll_state
            .content_length(req.horizontal.into());
    }
}
