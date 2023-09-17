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
    pub selected_header_index: usize,
    pub selected_response_header_index: usize,
    pub selected_params_index: usize,
}

impl App {
    pub fn new() -> App {
        let res_1 = parse_raw_trace(TEST_JSON_1);

        let res_2 = parse_raw_trace(TEST_JSON_2);

        let res_3 = parse_raw_trace(TEST_JSON_3);

        let res_4 = parse_raw_trace(TEST_JSON_4);

        let res_5 = parse_raw_trace(TEST_JSON_5);

        let res_6 = parse_raw_trace(TEST_JSON_6);

        let res_7 = parse_raw_trace(TEST_JSON_7);

        let res_8 = parse_raw_trace(TEST_JSON_8);

        let res_9 = parse_raw_trace(TEST_JSON_9);

        let mut items = vec![];

        vec![
            &res_1, &res_2, &res_3, &res_4, &res_5, &res_6, &res_7, &res_8, &res_9,
        ]
        .iter()
        .for_each(|x| match x {
            Ok(v) => {
                let cloned = v.clone();

                items.push(cloned)
            }
            Err(_) => {}
        });

        App {
            active_block: ActiveBlock::NetworkRequests,
            request_details_block: RequestDetailsPane::Body,
            response_details_block: ResponseDetailsPane::Body,
            mode: Mode::Normal,
            selection_index: 0,
            selected_params_index: 0,
            selected_header_index: 0,
            selected_response_header_index: 0,
            items,
        }
    }
}
