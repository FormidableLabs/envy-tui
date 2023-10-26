use std::collections::{BTreeSet, HashMap};
use std::fmt::Display;
use std::hash::{Hash, Hasher};

use crossterm::event::KeyCode;
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
    SearchQuery,
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
    pub state: State,
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

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub enum KeyMap {
    CopyToClipBoard,
    Esc,
    NavigateLeft,
    NavigateDown,
    NavigateUp,
    NavigateRight,
    GoToEnd,
    GoToStart,
    NextSection,
    PreviousSection,
    Quit,
    Search,
}

impl Display for KeyMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct UIState {
    pub index: usize,
    pub offset: usize,
    pub height: u16,
    pub width: u16,
    pub horizontal_offset: usize,
    pub scroll_state: ScrollbarState,
    pub horizontal_scroll_state: ScrollbarState,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum State {
    Received,
    Sent,
    Aborted,
    Blocked,
    Timeout,
    Error,
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
    pub search_query: String,
    pub main: UIState,
    pub response_body: UIState,
    pub request_body: UIState,
    pub request_details: UIState,
    pub response_details: UIState,
    pub is_first_render: bool,
    pub logs: Vec<String>,
    pub mode: Mode,
    pub key_map: HashMap<KeyMap, Vec<KeyCode>>,
    pub should_quit: bool,
}

pub struct KeyEntry {
    pub key_map: KeyMap,
    pub key_codes: Vec<KeyCode>,
}

impl PartialEq for KeyEntry {
    fn eq(&self, other: &KeyEntry) -> bool {
        self.key_map == other.key_map
    }
}

impl Eq for KeyEntry {}

impl PartialOrd for KeyEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.key_map.to_string().cmp(&self.key_map.to_string()))
    }
}

impl Ord for KeyEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.key_map.to_string().cmp(&self.key_map.to_string())
    }
}

impl App {
    pub fn new() -> App {
        let keys: HashMap<KeyMap, Vec<KeyCode>> = HashMap::from([
            (KeyMap::NavigateLeft, vec![KeyCode::Char('h')]),
            (
                KeyMap::NavigateDown,
                vec![KeyCode::Down, KeyCode::Char('j')],
            ),
            (KeyMap::NavigateUp, vec![KeyCode::Up, KeyCode::Char('k')]),
            (KeyMap::NavigateRight, vec![KeyCode::Char('l')]),
            (
                KeyMap::GoToEnd,
                vec![KeyCode::Char('>'), KeyCode::Char('K')],
            ),
            (
                KeyMap::GoToStart,
                vec![KeyCode::Char('<'), KeyCode::Char('J')],
            ),
            (KeyMap::Quit, vec![KeyCode::Char('q')]),
            (KeyMap::NextSection, vec![KeyCode::Tab]),
            (KeyMap::PreviousSection, vec![KeyCode::BackTab]),
            (KeyMap::CopyToClipBoard, vec![KeyCode::Char('y')]),
            (KeyMap::Search, vec![KeyCode::Char('/')]),
            (KeyMap::Esc, vec![KeyCode::Esc]),
        ]);

        App {
            key_map: keys,
            mode: Mode::Normal,
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
            search_query: String::with_capacity(10),
            main: UIState {
                offset: 0,
                index: 0,
                height: 0,
                width: 0,
                horizontal_offset: 0,
                scroll_state: ScrollbarState::default(),
                horizontal_scroll_state: ScrollbarState::default(),
            },
            response_body: UIState {
                offset: 0,
                index: 0,
                height: 0,
                width: 0,
                horizontal_offset: 0,
                scroll_state: ScrollbarState::default(),
                horizontal_scroll_state: ScrollbarState::default(),
            },
            request_details: UIState {
                offset: 0,
                index: 0,
                height: 0,
                width: 0,
                horizontal_offset: 0,
                scroll_state: ScrollbarState::default(),
                horizontal_scroll_state: ScrollbarState::default(),
            },
            response_details: UIState {
                offset: 0,
                index: 0,
                height: 0,
                width: 0,
                horizontal_offset: 0,
                scroll_state: ScrollbarState::default(),
                horizontal_scroll_state: ScrollbarState::default(),
            },
            request_body: UIState {
                offset: 0,
                index: 0,
                height: 0,
                width: 0,
                horizontal_offset: 0,
                scroll_state: ScrollbarState::default(),
                horizontal_scroll_state: ScrollbarState::default(),
            },
            should_quit: false,
        }
    }
}

pub enum AppDispatch {
    MarkTraceAsTimedOut(String),
    ClearStatusMessage,
}

impl App {
    pub fn log(&mut self, message: String) {
        self.logs.push(message)
    }

    pub fn dispatch(&mut self, action: AppDispatch) {
        match action {
            AppDispatch::MarkTraceAsTimedOut(id) => {
                self.mark_trace_as_timed_out(id);
            }
            _ => {}
        }
    }

    fn mark_trace_as_timed_out(&mut self, id: String) {
        let selected_trace = self.items.iter().find(|trace| trace.id == id);

        if selected_trace.is_some() {
            let mut selected_trace = selected_trace.unwrap().clone();

            if selected_trace.state == State::Sent {
                selected_trace.state = State::Timeout;
                selected_trace.status = None;
                selected_trace.response_body = Some("TIMEOUT WAITING FOR RESPONSE".to_string());
                selected_trace.pretty_response_body =
                    Some("TIMEOUT WAITING FOR RESPONSE".to_string());

                self.items.replace(selected_trace);
            };
        }
    }
}
