use std::fmt::Display;

use crate::mock::{
    TEST_JSON_1, TEST_JSON_2, TEST_JSON_3, TEST_JSON_4, TEST_JSON_5, TEST_JSON_6, TEST_JSON_7,
    TEST_JSON_8, TEST_JSON_9,
};
use crate::parser::parse_raw_trace;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RequestDetailsPane {
    Query,
    Headers,
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
    RequestSummary,
    Help,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Insert,
    Normal,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WsServerState {
    Closed,
    Open,
    HasConnections(usize),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Request {
    pub id: String,
    pub method: http::method::Method,
    pub status: Option<http::status::StatusCode>,
    pub request_headers: http::HeaderMap,
    pub response_headers: http::HeaderMap,
    pub uri: String,
    pub duration: Option<u32>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
    pub http_version: Option<http::Version>,
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
    pub selected_request_header_index: usize,
    pub selected_response_header_index: usize,
    pub selected_params_index: usize,
    pub ws_server_state: WsServerState,
}

impl App {
    pub fn new() -> App {
        let mut items = vec![];

        vec![
            TEST_JSON_1,
            TEST_JSON_2,
            TEST_JSON_3,
            TEST_JSON_4,
            TEST_JSON_5,
            TEST_JSON_6,
            TEST_JSON_7,
            TEST_JSON_8,
            TEST_JSON_9,
        ]
        .iter()
        .map(|raw_json_string| parse_raw_trace(raw_json_string))
        .for_each(|x| match x {
            Ok(v) => items.push(v),
            Err(_) => {}
        });

        App {
            active_block: ActiveBlock::NetworkRequests,
            request_details_block: RequestDetailsPane::Headers,
            response_details_block: ResponseDetailsPane::Body,
            mode: Mode::Normal,
            selection_index: 0,
            selected_params_index: 0,
            selected_request_header_index: 0,
            selected_response_header_index: 0,
            items,
            ws_server_state: WsServerState::Closed,
        }
    }
}
