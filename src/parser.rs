use std::io::{Error, ErrorKind};
use std::ops::Deref;
use std::str::FromStr;

use http::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use regex::Regex;

use crate::app::{State, Trace};

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

pub fn populate_header_map(raw_headers: &Map<String, Value>, map: &mut HeaderMap) {
    raw_headers.iter().for_each(|(key, value)| {
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

pub enum Payload {
    Trace(Trace),
    Connection(String),
}

pub fn parse_raw_trace(stringified_json: &str) -> Result<Payload, Box<dyn std::error::Error>> {
    let potential_json_body: Value = serde_json::from_str(stringified_json)?;

    let type_property = &potential_json_body["type"];

    let type_property = match type_property {
        Value::String(s) => {
            if s.deref() == "connections".to_string() || s.deref() == "trace".to_string() {
                Ok(s)
            } else {
                Err("".to_string())
            }
        }
        _ => Err("".to_string()),
    }?;

    match type_property.as_str() {
        "connections" => Ok(Payload::Connection("".to_string())),
        "trace" => {
            let data = &potential_json_body["data"];

            let http = &data["http"];

            let method = &http["method"];

            let method = match method {
                Value::String(v) => Ok(v),
                _ => Err("Method must be a string.".to_string()),
            }?;

            let method = http::method::Method::from_str(&method)?;

            let status_code = &http["statusCode"];

            let status_code = match status_code {
                Value::Number(v) => {
                    let result =
                        http::StatusCode::from_u16(v.as_u64().unwrap().try_into().unwrap_or(9999));

                    match result {
                        Ok(code) => Some(code),
                        Err(_) => None,
                    }
                }
                _ => None,
            };

            let http_version = match &http["httpVersion"] {
                Value::String(code) => match code.as_str() {
                    "HTTP/0.9" => Some(http::Version::HTTP_09),
                    "HTTP/1.0" => Some(http::Version::HTTP_10),
                    "HTTP/1.1" => Some(http::Version::HTTP_11),
                    "HTTP/2.0" => Some(http::Version::HTTP_2),
                    "HTTP/3.0" => Some(http::Version::HTTP_3),
                    _ => None,
                },
                _ => None,
            };

            let state = match &http["state"] {
                Value::String(g) => match g.as_str() {
                    "received" => State::Received,
                    "sent" => State::Sent,
                    "timeout" => State::Timeout,
                    "aborted" => State::Aborted,
                    "blocked" => State::Blocked,
                    _ => State::Error,
                },
                _ => State::Error,
            };

            let duration = &http["duration"];

            let duration = match duration {
                Value::String(v) => f32::from_str(v.as_str()).map_err(|_| "".to_string()),
                Value::Number(v) => {
                    let as_float = v.as_f64();

                    let as_f32 = as_float.map(|n| n as f32);

                    let converted = as_f32.ok_or("".to_string());

                    converted
                }
                _ => Err("Duration must be a number.".to_string()),
            }
            .ok()
            .map(|f| f as u32);

            let timestamp = &data["timestamp"];

            let timestamp = match timestamp {
                Value::String(v) => u64::from_str(v.as_str()).map_err(|_| "".to_string()),
                Value::Number(v) => Ok(v.as_u64().unwrap()),
                _ => Err("Must be a number.".to_string()),
            }
            .ok()
            .or(Some(0))
            .unwrap();

            let id = &data["id"];

            let id = match id {
                Value::String(k) => Ok(k.to_string()),
                _ => Err("".to_string()),
            }
            .ok()
            .expect("Id is mandatory.");

            let url = &http["url"];

            let uri = match url {
                Value::String(k) => Ok(k.to_string()),
                _ => Err("".to_string()),
            }
            .ok()
            .expect("Url is mandatory");

            let port = &http["port"];

            let port = match port {
                Value::String(k) => Ok(k.to_string()),
                _ => Err("".to_string()),
            }
            .ok();

            let timings = serde_json::from_value::<HTTPTimings>(http["timings"].clone()).ok();

            let mut request = Trace {
                port,
                timestamp,
                duration,
                id,
                uri,
                response_headers: http::HeaderMap::new(),
                request_headers: http::HeaderMap::new(),
                method,
                status: status_code,
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

            match &http["responseBody"] {
                Value::String(raw_response_body) => match pretty_parse_body(&raw_response_body) {
                    Ok(pretty_response_body) => {
                        let len = pretty_response_body.lines().collect::<Vec<_>>().len();

                        request.pretty_response_body_lines = Some(len);
                        request.pretty_response_body = Some(pretty_response_body);
                        request.response_body = Some(raw_response_body.deref().to_string());

                        ()
                    }
                    _ => {}
                },
                _ => {}
            };

            match &http["requestBody"] {
                Value::String(raw_request_body) => match pretty_parse_body(&raw_request_body) {
                    Ok(pretty_request_body) => {
                        let len = pretty_request_body.lines().collect::<Vec<_>>().len();

                        request.pretty_request_body_lines = Some(len);
                        request.pretty_request_body = Some(pretty_request_body);
                        request.request_body = Some(raw_request_body.to_string());

                        ()
                    }
                    Err(_) => (),
                },
                _ => (),
            };

            match &http["requestHeaders"] {
                Value::Object(k) => {
                    populate_header_map(&k, &mut request.request_headers);
                }
                _ => {}
            }

            match &http["responseHeaders"] {
                Value::Object(k) => {
                    populate_header_map(&k, &mut request.response_headers);
                }
                _ => {}
            }

            Ok(Payload::Trace(request))
        }
        _ => {
            let err = Error::new(ErrorKind::Other, "oh no!");

            Err(Box::new(err) as Box<dyn std::error::Error>)
        }
    }
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

pub fn pretty_parse_body(json: &str) -> Result<String, Box<dyn std::error::Error>> {
    let potential_json_body = serde_json::from_str::<Value>(json)?;

    let parsed_json = serde_json::to_string_pretty(&potential_json_body)?;

    Ok(parsed_json)
}
