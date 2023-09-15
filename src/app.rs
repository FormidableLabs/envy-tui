use std::fmt::Display;

use http::header::{
    ACCEPT_RANGES, ACCESS_CONTROL_ALLOW_ORIGIN, AGE, AUTHORIZATION, CACHE_CONTROL,
    CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, DATE, ETAG, EXPIRES, HOST, USER_AGENT,
};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RequestDetailsPane {
    Query,
    Headers,
    Body,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ResponseDetailsPane {
    Headers,
    Body,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActiveBlock {
    NetworkRequests,
    RequestDetails,
    ResponseDetails,
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
            "Request URL: {:?}, method used: {:?}, response status is {:?}, time took: {:?} milliseconds.",
            self.uri, self.method, self.status, self.duration
        )
    }
}

pub struct App {
    pub active_block: ActiveBlock,
    pub request_details_block: RequestDetailsPane,
    pub response_details_block: ResponseDetailsPane,
    pub mode: Mode,
    pub items: Vec<Request>,
    pub selection_index: usize,
    pub selected_header_index: usize,
    pub selected_response_header_index: usize,
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
        first_request
            .response_headers
            .append(ACCEPT_RANGES, "bytes".parse().unwrap());
        first_request
            .response_headers
            .append(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
        first_request
            .response_headers
            .append(AGE, "0".parse().unwrap());

        first_request
            .response_headers
            .append(CACHE_CONTROL, "max-age=600".parse().unwrap());

        first_request
            .response_headers
            .append(CONTENT_ENCODING, "gzip".parse().unwrap());

        first_request
            .response_headers
            .append(CONTENT_LENGTH, "6530".parse().unwrap());

        first_request
            .response_headers
            .append(CONTENT_TYPE, "text/html; charset=utf-8".parse().unwrap());

        first_request
            .response_headers
            .append(DATE, "Fri, 15 Sep 2023 07:46:09 GMT".parse().unwrap());

        first_request
            .response_headers
            .append(ETAG, r#"W/"65039b6c-7819"#.parse().unwrap());

        first_request
            .response_headers
            .append(EXPIRES, r#"W/"65039b6c-7819"#.parse().unwrap());

        first_request
            .response_headers
            .append(ETAG, r#"W/"65039b6c-7819"#.parse().unwrap());

        first_request
            .response_headers
            .append(ETAG, r#"W/"65039b6c-7819"#.parse().unwrap());
        // Expires:
        // Fri, 15 Sep 2023 07:56:09 GMT
        // Last-Modified:
        // Thu, 14 Sep 2023 23:46:52 GMT
        // Permissions-Policy:
        // interest-cohort=()
        // Server:
        // GitHub.com
        //
        // Strict-Transport-Security:
        // max-age=31556952
        // Vary:
        // Accept-Encoding
        // Via:
        // 1.1 varnish
        // X-Cache:
        // MISS
        // X-Cache-Hits:
        // 0
        // X-Fastly-Request-Id:
        // 02bdf6f717a69d031d4b1861d2f6b00eaf7455d8
        // X-Github-Request-Id:
        // 0F12:111F2:3EF449D:405EA60:65040BC0
        // X-Origin-Cache:
        // HIT
        // X-Proxy-Cache:
        // MISS
        // X-Served-By:
        // cache-fra-eddf8230025-FRA
        // X-Timer:
        // S1694763969.404281,VS0,VE100

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
            request_details_block: RequestDetailsPane::Body,
            response_details_block: ResponseDetailsPane::Body,
            mode: Mode::Normal,
            selection_index: 0,
            selected_params_index: 0,
            selected_header_index: 0,
            selected_response_header_index: 0,
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
