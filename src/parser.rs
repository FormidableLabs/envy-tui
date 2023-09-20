use std::collections::HashMap;

use std::collections::hash_map::RandomState;
use std::error::Error;
use std::str::FromStr;

use http::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::app::Request;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct RawTrace {
    timestamp: u64,
    id: String,
    url: String,
    statusMessage: Option<String>,
    statusCode: Option<usize>,
    method: String,
    duration: Option<u32>,
    host: String,
    httpVersion: Option<String>,
    path: Option<String>,
    port: Option<usize>,
    responseBody: Option<String>,
    requestBody: Option<String>,
    responseHeaders: Option<HashMap<String, Value>>,
    requestHeaders: Option<HashMap<String, Value>>,
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

pub fn parse_raw_trace(stringified_json: &str) -> Result<Request, Box<dyn Error>> {
    let potential_json_body = serde_json::from_str::<RawTrace>(stringified_json)?;

    let method = http::method::Method::from_str(&potential_json_body.method)?;

    let status = match potential_json_body.statusCode {
        Some(code) => {
            let result = http::StatusCode::from_u16(code.try_into().unwrap_or(9999));

            match result {
                Ok(code) => Some(code),
                Err(_) => None,
            }
        }
        None => None,
    };

    let http_version = match potential_json_body.httpVersion {
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

    let mut request = Request {
        timestamp: potential_json_body.timestamp,
        duration: potential_json_body.duration,
        id: potential_json_body.id,
        uri: potential_json_body.url,
        response_headers: http::HeaderMap::new(),
        request_headers: http::HeaderMap::new(),
        method,
        status,
        http_version,
        request_body: potential_json_body.requestBody,
        response_body: potential_json_body.responseBody,
    };

    populate_header_map(
        &potential_json_body.requestHeaders,
        &mut request.request_headers,
    );

    populate_header_map(
        &potential_json_body.responseHeaders,
        &mut request.response_headers,
    );

    Ok(request)
}
