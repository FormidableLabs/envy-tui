use std::collections::BTreeSet;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

use ratatui::widgets::ScrollbarState;
use tokio::task::AbortHandle;

use crate::mock::{
    TEST_JSON_1, TEST_JSON_10, TEST_JSON_11, TEST_JSON_12, TEST_JSON_13, TEST_JSON_14,
    TEST_JSON_15, TEST_JSON_16, TEST_JSON_17, TEST_JSON_18, TEST_JSON_2, TEST_JSON_3, TEST_JSON_4,
    TEST_JSON_5, TEST_JSON_6, TEST_JSON_7, TEST_JSON_8, TEST_JSON_9,
};
use crate::parser::parse_raw_trace;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RequestDetailsPane {
    Query,
    Headers,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ResponseDetailsPane {
    Body,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActiveBlock {
    NetworkRequests,
    RequestDetails,
    ResponseDetails,
    ResponseBody,
    RequestSummary,
    Help,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WsServerState {
    Closed,
    Open,
    HasConnections(usize),
}

#[derive(Clone, Debug)]
pub struct Request {
    pub id: String,
    pub timestamp: u64,
    pub method: http::method::Method,
    pub status: Option<http::status::StatusCode>,
    pub request_headers: http::HeaderMap,
    pub response_headers: http::HeaderMap,
    pub uri: String,
    pub duration: Option<u32>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
    pub pretty_response_body: Option<String>,
    pub pretty_response_body_lines: Option<usize>,
    pub http_version: Option<http::Version>,
}

impl PartialEq<Request> for Request {
    fn eq(&self, other: &Request) -> bool {
        self.id == *other.id
    }
}

impl Eq for Request {}

impl PartialOrd for Request {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.timestamp.cmp(&other.timestamp))
    }
}

impl Ord for Request {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl Hash for Request {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ID: {:?}, Request URL: {:?}, method used: {:?}, response status is {:?}, time took: {:?} milliseconds.",
            self.id, self.uri, self.method, self.status, self.duration
        )
    }
}

pub struct UIState {
    pub index: usize,
    pub offset: usize,
    pub h_offset: usize,
    pub scroll_state: ScrollbarState,
    pub h_scroll_state: ScrollbarState,
}

pub struct App {
    pub active_block: ActiveBlock,
    pub request_details_block: RequestDetailsPane,
    pub response_details_block: ResponseDetailsPane,
    pub items: BTreeSet<Request>,
    pub selection_index: usize,
    pub selected_request_header_index: usize,
    pub selected_response_header_index: usize,
    pub selected_params_index: usize,
    pub ws_server_state: WsServerState,
    pub status_message: Option<String>,
    pub abort_handlers: Vec<AbortHandle>,
    pub main: UIState,
    pub response_body: UIState,
}

impl App {
    pub fn new() -> App {
        let mut items: BTreeSet<Request> = BTreeSet::new();

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
            TEST_JSON_10,
            TEST_JSON_11,
            TEST_JSON_12,
            TEST_JSON_13,
            TEST_JSON_14,
            TEST_JSON_15,
            TEST_JSON_16,
            TEST_JSON_17,
            TEST_JSON_18,
        ]
        .iter()
        .map(|raw_json_string| parse_raw_trace(raw_json_string))
        .for_each(|x| match x {
            Ok(v) => {
                items.insert(v);
            }
            Err(err) => {
                println!(
                    "Something went wrong while inserting to the Tree, {:?}",
                    err
                )
            }
        });

        App {
            active_block: ActiveBlock::NetworkRequests,
            request_details_block: RequestDetailsPane::Headers,
            response_details_block: ResponseDetailsPane::Body,
            selection_index: 0,
            selected_params_index: 0,
            selected_request_header_index: 0,
            selected_response_header_index: 0,
            items,
            ws_server_state: WsServerState::Closed,
            status_message: None,
            abort_handlers: vec![],
            main: UIState {
                offset: 0,
                // TODO: Move it back to 20. Just for dev purposes.
                index: 7,
                h_offset: 0,
                scroll_state: ScrollbarState::default(),
                h_scroll_state: ScrollbarState::default(),
            },
            response_body: UIState {
                offset: 0,
                index: 0,
                h_offset: 0,
                scroll_state: ScrollbarState::default(),
                h_scroll_state: ScrollbarState::default(),
            },
        }
    }
}
