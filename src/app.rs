use std::fmt::Display;

use http::header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, HOST, USER_AGENT};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActiveBlock {
    NetworkRequests,
    RequestDetails,
    RequestHeaders,
    ResponseHeaders,
    Summary,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Insert,
    Normal,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Request {
    pub id: String,
    pub method: http::method::Method,
    pub status: http::status::StatusCode,
    pub request_headers: http::HeaderMap,
    pub response_headers: http::HeaderMap,
    pub uri: String,
    pub duration: u32,
    pub body: Option<String>,
}

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Method used: {:?}, response status is {:?}, time took: {:?} miliseconds.",
            self.method, self.status, self.duration
        )
    }
}

pub struct App {
    pub active_block: ActiveBlock,
    pub mode: Mode,
    pub items: Vec<Request>,
    pub selection_index: usize,
    pub selected_header_index: usize,
    pub selected_params_index: usize,
}

impl App {
    pub fn new() -> App {
        let mut first_request = Request {
            status: http::StatusCode::OK,
            method: http::method::Method::GET,
            id: String::from("id"),
            uri: String::from("https://randomdomain.com/randompath?foo=bar&bottle=water"),
            duration: 524,
            request_headers: http::HeaderMap::new(),
            response_headers: http::HeaderMap::new(),
            body: None,
        };

        first_request
            .request_headers
            .append(HOST, "randomdomain.com".parse().unwrap());

        first_request
            .request_headers
            .append(CONTENT_TYPE, "application/json".parse().unwrap());

        first_request
            .request_headers
            .append(CACHE_CONTROL, "max-age=604800".parse().unwrap());

        first_request
            .request_headers
            .append(AUTHORIZATION, "Bearer token".parse().unwrap());

        first_request.request_headers.append(
            USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.9; rv:50.0) Gecko/20100101 Firefox/50.0"
                .parse()
                .unwrap(),
        );

        let mut second_request = Request {
            status: http::StatusCode::OK,
            method: http::method::Method::GET,
            id: String::from("id"),
            uri: String::from(
                "https://randomdomain.com/anotherpath/someresource?cursor=4056&limit=10",
            ),
            duration: 524,
            request_headers: http::HeaderMap::new(),
            response_headers: http::HeaderMap::new(),
            body: None,
        };

        second_request
            .request_headers
            .append(HOST, "randomdomain.com".parse().unwrap());

        second_request
            .request_headers
            .append(CONTENT_TYPE, "application/json".parse().unwrap());

        second_request
            .request_headers
            .append(AUTHORIZATION, "Bearer token".parse().unwrap());

        second_request.request_headers.append(
            USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.9; rv:50.0) Gecko/20100101 Firefox/50.0"
                .parse()
                .unwrap(),
        );

        App {
            active_block: ActiveBlock::NetworkRequests,
            mode: Mode::Normal,
            selection_index: 0,
            selected_params_index: 0,
            selected_header_index: 0,
            items: vec![
                first_request,
                second_request,
                Request {
                    method: http::method::Method::POST,
                    status: http::StatusCode::CREATED,
                    id: String::from("id2"),
                    uri: String::from("https://randomdomain.com/randompath"),
                    duration: 1511,
                    request_headers: http::HeaderMap::new(),
                    response_headers: http::HeaderMap::new(),
                    body: Some(String::from(r#"{}"#)),
                },
                Request {
                    method: http::method::Method::DELETE,
                    status: http::StatusCode::NOT_FOUND,
                    id: String::from("id3"),
                    uri: String::from("https://randomdomain.com/randompath/nestedPath"),
                    duration: 242,
                    request_headers: http::HeaderMap::new(),
                    response_headers: http::HeaderMap::new(),
                    body: None,
                },
            ],
        }
    }
}
