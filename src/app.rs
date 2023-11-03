use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::KeyEvent;
use ratatui::widgets::ScrollbarState;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;
use ratatui::layout::Layout;
use ratatui::prelude::{Constraint, CrosstermBackend, Direction};

use crate::mock;
use crate::parser::{Payload, parse_raw_trace};
use crate::utils::set_content_length;
use crate::render;
use crate::tui;
use crate::wss::WebSocket;
use crate::wss;
use crate::components::home::Home;

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum RequestDetailsPane {
    #[default]
    Query,
    Headers,
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum ResponseDetailsPane {
    #[default]
    Body,
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum Mode {
    #[default]
    Debug,
    Normal,
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum ActiveBlock {
    #[default]
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
    pub port: Option<String>,
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

#[derive(Default)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    CopyToClipBoard,
    NavigateLeft(Option<KeyEvent>),
    NavigateDown(Option<KeyEvent>),
    NavigateUp(Option<KeyEvent>),
    NavigateRight(Option<KeyEvent>),
    GoToEnd,
    GoToStart,
    NextSection,
    PreviousSection,
    Quit,
    NewSearch,
    UpdateSearchQuery(char),
    DeleteSearchQuery,
    ExitSearch,
    Help,
    ToggleDebug,
    DeleteItem,
    FocusOnTraces,
    ShowTraceDetails,
    NextPane,
    PreviousPane,
    StopWebSocketServer,
    StartWebSocketServer,
}

pub enum AppDispatch {
    MarkTraceAsTimedOut(String),
    ClearStatusMessage,
}

#[derive(Default)]
pub struct Components {
    home: Arc<Mutex<Home>>,
    websocket_client: Arc<Mutex<WebSocketClientState>>,
}

#[derive(Default)]
pub struct Services {
    pub collector_server: Arc<Mutex<WebSocket>>,
}

#[derive(Default)]
pub struct WebSocketClientState {
    open: bool,
}

#[derive(Default)]
pub struct App {
    pub action_tx: Option<UnboundedSender<AppDispatch>>,
    pub components: Components,
    pub services: Services,
    pub active_block: ActiveBlock,
    pub previous_block: Option<ActiveBlock>,
    pub request_details_block: RequestDetailsPane,
    pub response_details_block: ResponseDetailsPane,
    pub items: BTreeSet<Trace>,
    pub selected_request_header_index: usize,
    pub selected_response_header_index: usize,
    pub selected_params_index: usize,
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
    pub key_map: HashMap<KeyEvent, Action>,
    pub should_quit: bool,
}

pub type Frame<'a> = ratatui::Frame<'a, CrosstermBackend<std::io::Stdout>>;

impl App {
    pub fn new() -> Result<App, Box<dyn Error>> {
        let config = crate::config::Config::new()?;
        let home = Arc::new(Mutex::new(Home::new()));
        let websocket_client = Arc::new(Mutex::new(WebSocketClientState::default()));
        let app = App {
            components: Components {
                home,
                websocket_client,
            },
            key_map: config.mapping.0,
            ..Self::default()
        };

        Ok(app)
    }

    pub fn schedule_server_stop(&mut self) {
        let websocket_client = self.components.websocket_client.clone();
        let collector_server = self.services.collector_server.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            collector_server.lock().await.start().await;
            if !websocket_client.lock().await.open {
                websocket_client.lock().await.open = true;
            }
        });
    }

    pub fn schedule_server_start(&mut self) {
        let websocket_client = self.components.websocket_client.clone();
        let collector_server = self.services.collector_server.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = collector_server.lock().await.stop().await;
            websocket_client.lock().await.open = false;
        });
    }

    async fn start_ws_client(&mut self) -> Result<(), Box<dyn Error>> {
        self.services.collector_server.lock().await.start().await;

        let app = self.components.home.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            wss::client(self, tx).await;
        }).await?;

        let wss_client = self.components.websocket_client.clone();
        wss_client.lock().await.open = true;

        Ok(())
    }

    fn register_action_handler(&mut self, tx: UnboundedSender<AppDispatch>) -> Result<(), Box<dyn Error>> {
        self.action_tx = Some(tx);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let (action_tx, _action_rx) = mpsc::unbounded_channel();

        self.insert_mock_data();
        if self.mode == Mode::Normal {
            self.start_ws_client().await?;
        }

        let mut t = tui::Tui::new();
        t.enter()?;

        self.register_action_handler(action_tx.clone())?;

        loop {
            let event = t.next().await;

            if let Some(tui::Event::Render) = event {
                t.terminal.draw(|frame| {
                    self.render(frame);
                })?;
            };

            // while let Ok(action) = action_rx.try_recv() {
            // match self.action_tx.try_recv() {
            //     Ok(value) => match value {
            //         Some(event) => match event {
            //             AppDispatch::MarkTraceAsTimedOut(id) => {
            //                 let mut app = self.component.lock().await;
            //                 app.dispatch(AppDispatch::MarkTraceAsTimedOut(id))
            //             }
            //             AppDispatch::ClearStatusMessage => {
            //                 let mut app = self.component.lock().await;
            //                 app.status_message = None;
            //             }
            //         },
            //         None => {}
            //     },
            //     Err(_) => {}
            // };

            // let mut ui_client = ui_client_raw.lock().await;
            let app = self.components.home.clone();
            if let Some(action) = app.lock().await.handle_events(event)? {
                app.lock().await.update(action.clone());
            }

            if app.lock().await.should_quit {
                break;
            }
        }

        t.exit()?;

        Ok(())
    }

    fn render(
        &mut self,
        frame: &mut Frame<'_>,
    ) {
        match self.active_block {
            ActiveBlock::Help => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(frame.size());

                render::render_help(self, frame, main_layout[0]);
            }
            ActiveBlock::Debug => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(frame.size());

                render::render_debug(self, frame, main_layout[0]);
            }
            _ => {
                let terminal_width = frame.size().width;

                if terminal_width > 200 {
                    let main_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints([Constraint::Percentage(95), Constraint::Length(3)].as_ref())
                        .split(frame.size());

                    let split_layout = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(30), Constraint::Percentage(70)].as_ref(),
                        )
                        .split(main_layout[0]);

                    let details_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [
                                Constraint::Length(3),
                                Constraint::Percentage(45),
                                Constraint::Percentage(45),
                            ]
                            .as_ref(),
                        )
                        .split(split_layout[1]);

                    let request_layout = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
                        )
                        .split(details_layout[1]);

                    let response_layout = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
                        )
                        .split(details_layout[2]);

                    render::render_request_block(self, frame, request_layout[0]);
                    render::render_request_body(self, frame, request_layout[1]);
                    render::render_traces(self, frame, split_layout[0]);

                    render::render_request_summary(self, frame, details_layout[0]);
                    render::render_response_block(self, frame, response_layout[0]);
                    render::render_response_body(self, frame, response_layout[1]);

                    // render::render_footer(self.component.clone(), self.services.collector_server.clone(), frame, main_layout[1]);

                    render::render_search(self, frame);

                    self.response_body.height = response_layout[1].height;
                    self.response_body.width = response_layout[1].width;
                    self.main.height = split_layout[0].height;
                } else {
                    let main_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints(
                            [
                                Constraint::Percentage(30),
                                Constraint::Min(3),
                                Constraint::Percentage(30),
                                Constraint::Percentage(30),
                                Constraint::Min(3),
                            ]
                            .as_ref(),
                        )
                        .split(frame.size());

                    let request_layout = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
                        )
                        .split(main_layout[2]);

                    let response_layout = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
                        )
                        .split(main_layout[3]);

                    render::render_request_block(self, frame, request_layout[0]);
                    render::render_request_body(self, frame, request_layout[1]);
                    render::render_traces(self, frame, main_layout[0]);

                    render::render_request_summary(self, frame, main_layout[1]);
                    render::render_response_block(self, frame, response_layout[0]);
                    render::render_response_body(self, frame, response_layout[1]);

                    render::render_search(self, frame);
                    // render::render_footer(self.component.clone(), self.services.collector_server.clone(), frame, main_layout[1]);

                    self.response_body.height = response_layout[1].height;
                    self.response_body.width = response_layout[1].width;

                    self.request_body.height = request_layout[1].height;
                    self.request_body.width = request_layout[1].width;

                    self.main.height = main_layout[0].height;
                }
            }
        };
        if self.is_first_render {
            // NOTE: Index and offset needs to be set prior before we call `set_content_length`.
            self.main.index = 0;
            self.main.offset = 0;

            set_content_length(self);

            self.main.scroll_state = self.main.scroll_state.content_length(self.items.len() as u16);

            self.is_first_render = false;
        }
    }


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

    fn insert_mock_data(&mut self) {
        vec![
            mock::TEST_JSON_1,
            mock::TEST_JSON_2,
            mock::TEST_JSON_3,
            mock::TEST_JSON_4,
            mock::TEST_JSON_5,
            mock::TEST_JSON_6,
            mock::TEST_JSON_7,
            mock::TEST_JSON_8,
            mock::TEST_JSON_9,
            mock::TEST_JSON_10,
            mock::TEST_JSON_11,
            mock::TEST_JSON_12,
            mock::TEST_JSON_13,
            mock::TEST_JSON_14,
            mock::TEST_JSON_15,
            mock::TEST_JSON_16,
            mock::TEST_JSON_17,
            mock::TEST_JSON_18,
        ]
        .iter()
        .map(|raw_json_string| parse_raw_trace(raw_json_string))
        .for_each(|x| match x {
            Ok(v) => match v {
                Payload::Trace(trace) => {
                    self.items.insert(trace);
                }
                _ => {}
            },
            Err(err) => self.logs.push(format!(
                "Something went wrong while parsing and inserting to the Tree, {:?}",
                err
            )),
        });
    }
}
