use std::collections::HashMap;

use std::collections::hash_map::RandomState;
use std::error::Error;
use std::str::FromStr;

use http::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use regex::Regex;

use crate::app::{State, Trace};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct HTTPTimings {
    blocked: f32,
    dns: f32,
    connect: f32,
    send: f32,
    wait: f32,
    receive: f32,
    ssl: f32,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct HTTPTrace {
    url: String,
    state: String,
    statusMessage: Option<String>,
    statusCode: Option<usize>,
    method: String,
    host: String,
    httpVersion: Option<String>,
    path: Option<String>,
    port: Option<Value>,
    responseBody: Option<String>,
    requestBody: Option<String>,
    responseHeaders: Option<HashMap<String, Value>>,
    requestHeaders: Option<HashMap<String, Value>>,
    duration: Option<f32>,
    timings: Option<HTTPTimings>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct RawTrace {
    timestamp: u64,
    id: String,
    http: HTTPTrace,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct Payload {
    data: RawTrace,
}

pub fn populate_header_map(
    raw_headers: &Option<HashMap<String, Value, RandomState>>,
    map: &mut HeaderMap,
) {
    match raw_headers {
        Some(raw_request_headers) => {
            raw_request_headers.iter().for_each(|(key, value)| {
                let coerced_name = http::HeaderName::from_str(key);

                match value {
                    Value::Array(array_json_element) => {
                        let first_element = &array_json_element[0];

                        match (first_element, coerced_name) {
                            (Value::String(string_header_value), Ok(valid_header_name)) => {
                                map.append(valid_header_name, string_header_value.parse().unwrap());
                            }
                            (_, _) => (),
                        }
                    }
                    Value::String(string_json_element) => match coerced_name {
                        Ok(valid_header_name) => {
                            map.append(valid_header_name, string_json_element.parse().unwrap());
                        }
                        _ => (),
                    },
                    _ => (),
                }
            });
        }
        None => (),
    }
}

pub fn parse_raw_trace(stringified_json: &str) -> Result<Trace, Box<dyn Error>> {
    let potential_json_body = serde_json::from_str::<Payload>(stringified_json)?;

    let method = http::method::Method::from_str(&potential_json_body.data.http.method)?;

    let status = match potential_json_body.data.http.statusCode {
        Some(code) => {
            let result = http::StatusCode::from_u16(code.try_into().unwrap_or(9999));

            match result {
                Ok(code) => Some(code),
                Err(_) => None,
            }
        }
        None => None,
    };

    let http_version = match potential_json_body.data.http.httpVersion {
        Some(code) => match code.as_str() {
            "HTTP/0.9" => Some(http::Version::HTTP_09),
            "HTTP/1.0" => Some(http::Version::HTTP_10),
            "HTTP/1.1" => Some(http::Version::HTTP_11),
            "HTTP/2.0" => Some(http::Version::HTTP_2),
            "HTTP/3.0" => Some(http::Version::HTTP_3),
            _ => None,
        },
        None => None,
    };

    let state = match potential_json_body.data.http.state.as_str() {
        "received" => State::Received,
        "sent" => State::Sent,
        "timeout" => State::Timeout,
        "aborted" => State::Aborted,
        "blocked" => State::Blocked,
        _ => State::Error,
    };

    let duration = potential_json_body.data.http.duration;

    let duration = if duration.is_some() {
        Some(duration.unwrap() as u32)
    } else {
        None
    };

    let mut request = Trace {
        timestamp: potential_json_body.data.timestamp,
        duration,
        id: potential_json_body.data.id,
        uri: potential_json_body.data.http.url,
        response_headers: http::HeaderMap::new(),
        request_headers: http::HeaderMap::new(),
        method,
        status,
        http_version,
        request_body: None,
        response_body: None,
        pretty_response_body: None,
        pretty_response_body_lines: None,
        pretty_request_body: None,
        pretty_request_body_lines: None,
        state,
        raw: pretty_parse_body(stringified_json)?,
    };

    match potential_json_body.data.http.responseBody {
        Some(raw_response_body) => match pretty_parse_body(&raw_response_body) {
            Ok(pretty_response_body) => {
                let len = pretty_response_body.lines().collect::<Vec<_>>().len();

                request.pretty_response_body_lines = Some(len);
                request.pretty_response_body = Some(pretty_response_body);
                request.response_body = Some(raw_response_body);

                ()
            }
            Err(_) => (),
        },
        None => (),
    };

    match potential_json_body.data.http.requestBody {
        Some(raw_request_body) => match pretty_parse_body(&raw_request_body) {
            Ok(pretty_request_body) => {
                let len = pretty_request_body.lines().collect::<Vec<_>>().len();

                request.pretty_request_body_lines = Some(len);
                request.pretty_request_body = Some(pretty_request_body);
                request.request_body = Some(raw_request_body);

                ()
            }
            Err(_) => (),
        },
        None => (),
    };

    populate_header_map(
        &potential_json_body.data.http.requestHeaders,
        &mut request.request_headers,
    );

    populate_header_map(
        &potential_json_body.data.http.responseHeaders,
        &mut request.response_headers,
    );

    Ok(request)
}

/// Finds all occurances of `\` and `"` in a header value string and add an `\` to it.
/// This escapes those characther making sure it behaves correctly in the command line.
///
/// # Example
///
///
/// let header_value = r#"\ping"#;
///
/// let escaped =escape_header(header_value);
///
/// assert_eq!(escaped, "r#"\\ping"#".to_string());
fn escape_header(value: &str) -> String {
    let regex = Regex::new(r#"(\\|")"#).unwrap();

    let result = regex.replace_all(value, "\\$1");

    result.to_string()
}

pub fn generate_curl_command(request: &Trace) -> String {
    let mut headers_as_curl: String = "".to_owned();

    let mut is_encoded = false;

    request.request_headers.iter().for_each(|(name, value)| {
        let value_str = value.to_str().unwrap();
        let name_str = name.to_string();

        if name != http::header::CONTENT_LENGTH {
            let escaped_value_str = escape_header(&value_str);

            let formatted_header =
                format!(r#"-H "{}: {}" "#, name_str.as_str(), &escaped_value_str);

            headers_as_curl.push_str(&formatted_header);
        }

        if name == http::header::ACCEPT_ENCODING {
            is_encoded = true
        }
    });

    let body_as_curl = match &request.request_body {
        Some(body) => format!("--data-binary '{}'", body),
        None => "".to_string(),
    };

    let compression_as_curl = match is_encoded {
        true => format!("--compressed"),
        _ => "".to_string(),
    };

    format!(
        "curl '{}' -X {} {} {} {}",
        request.uri, request.method, headers_as_curl, body_as_curl, compression_as_curl
    )
}

pub fn pretty_parse_body(json: &str) -> Result<String, Box<dyn Error>> {
    let potential_json_body = serde_json::from_str::<Value>(json)?;

    let parsed_json = serde_json::to_string_pretty(&potential_json_body)?;

    Ok(parsed_json)
}
