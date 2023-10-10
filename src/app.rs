use std::collections::BTreeSet;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

use ratatui::widgets::ScrollbarState;
use tokio::task::AbortHandle;

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
pub enum Mode {
    Debug,
    Normal,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActiveBlock {
    TracesBlock,
    RequestDetails,
    RequestBody,
    ResponseDetails,
    ResponseBody,
    RequestSummary,
    Help,
    Debug,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WsServerState {
    Closed,
    Open,
    HasConnections(usize),
}

#[derive(Clone, Debug)]
pub struct Trace {
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
    pub pretty_request_body: Option<String>,
    pub pretty_request_body_lines: Option<usize>,
    pub http_version: Option<http::Version>,
    pub raw: String,
}

impl PartialEq<Trace> for Trace {
    fn eq(&self, other: &Trace) -> bool {
        self.id == *other.id
    }
}

impl Eq for Trace {}

impl PartialOrd for Trace {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.timestamp.cmp(&self.timestamp))
    }
}

impl Ord for Trace {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.timestamp.cmp(&self.timestamp)
    }
}

impl Hash for Trace {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Display for Trace {
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
    pub horizontal_offset: usize,
    pub scroll_state: ScrollbarState,
    pub horizontal_scroll_state: ScrollbarState,
}

pub struct App {
    pub active_block: ActiveBlock,
    pub previous_block: Option<ActiveBlock>,
    pub request_details_block: RequestDetailsPane,
    pub response_details_block: ResponseDetailsPane,
    pub items: BTreeSet<Trace>,
    pub selected_request_header_index: usize,
    pub selected_response_header_index: usize,
    pub selected_params_index: usize,
    pub ws_server_state: WsServerState,
    pub status_message: Option<String>,
    pub abort_handlers: Vec<AbortHandle>,
    pub main: UIState,
    pub response_body: UIState,
    pub request_body: UIState,
    pub request_details: UIState,
    pub response_details: UIState,
    pub is_first_render: bool,
    pub logs: Vec<String>,
    pub mode: Mode,
}

impl App {
    pub fn new() -> App {
        App {
            mode: Mode::Debug,
            logs: vec![],
            is_first_render: true,
            active_block: ActiveBlock::TracesBlock,
            request_details_block: RequestDetailsPane::Headers,
            response_details_block: ResponseDetailsPane::Body,
            selected_params_index: 0,
            selected_request_header_index: 0,
            selected_response_header_index: 0,
            items: BTreeSet::new(),
            ws_server_state: WsServerState::Closed,
            status_message: None,
            abort_handlers: vec![],
            previous_block: None,
            main: UIState {
                offset: 0,
                index: 0,
                horizontal_offset: 0,
                scroll_state: ScrollbarState::default(),
                horizontal_scroll_state: ScrollbarState::default(),
            },
            response_body: UIState {
                offset: 0,
                index: 0,
                horizontal_offset: 0,
                scroll_state: ScrollbarState::default(),
                horizontal_scroll_state: ScrollbarState::default(),
            },
            request_details: UIState {
                offset: 0,
                index: 0,
                horizontal_offset: 0,
                scroll_state: ScrollbarState::default(),
                horizontal_scroll_state: ScrollbarState::default(),
            },
            response_details: UIState {
                offset: 0,
                index: 0,
                horizontal_offset: 0,
                scroll_state: ScrollbarState::default(),
                horizontal_scroll_state: ScrollbarState::default(),
            },
            request_body: UIState {
                offset: 0,
                index: 0,
                horizontal_offset: 0,
                scroll_state: ScrollbarState::default(),
                horizontal_scroll_state: ScrollbarState::default(),
            },
        }
    }
}
