use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Layout,
    prelude::{Constraint, Direction},
};
use std::error::Error;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    str::FromStr,
};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;

use crate::{
    app::{Action, ActiveBlock, Mode, RequestDetailsPane, ResponseDetailsPane, UIState},
    components::component::Component,
    components::handlers,
    render,
    services::websocket::{State, Trace},
    tui::{Event, Frame},
};

#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub enum WebSockerInternalState {
    Connected(usize),
    Open,
    #[default]
    Closed,
}

#[derive(Clone, PartialEq, Debug, Eq, Default)]
pub enum FilterSource {
    #[default]
    All,
    Applied(HashSet<String>), // Source(String),
                              // Method(http::method::Method),
                              // Status(String),
}

#[derive(Default)]
pub struct MethodFilter {
    pub method: http::method::Method,
    pub name: String,
    pub selected: bool,
}

#[derive(Default)]
pub struct StatusFilter {
    pub status: String,
    pub name: String,
    pub selected: bool,
}

#[derive(Default)]
pub struct Home {
    pub action_tx: Option<UnboundedSender<Action>>,
    pub active_block: ActiveBlock,
    pub previous_blocks: Vec<ActiveBlock>,
    pub request_details_block: RequestDetailsPane,
    pub response_details_block: ResponseDetailsPane,
    pub items: BTreeSet<Trace>,
    pub selected_request_header_index: usize,
    pub selected_response_header_index: usize,
    pub selected_params_index: usize,
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
    pub status_message: Option<String>,
    pub ws_status: String,
    pub wss_connected: bool,
    pub wss_connection_count: usize,
    pub wss_state: WebSockerInternalState,
    pub filter_index: usize,
    metadata: Option<handlers::HandlerMetadata>,
    filter_source: FilterSource,
    pub method_filters: HashMap<http::method::Method, MethodFilter>,
    pub status_filters: HashMap<String, StatusFilter>,
}

impl Home {
    pub fn new() -> Result<Home, Box<dyn Error>> {
        let config = crate::config::Config::new()?;
        let mut home = Home {
            key_map: config.mapping.0,
            ..Self::default()
        };

        let methods = vec!["POST", "GET", "DELETE", "PUT", "PATCH", "OPTION"];

        let statuses = vec!["1xx", "2xx", "3xx", "4xx", "5xx"];

        statuses.iter().for_each(|status| {
            home.status_filters.insert(
                status.clone().to_string(),
                StatusFilter {
                    status: status.to_string(),
                    selected: false,
                    name: status.to_string(),
                },
            );
        });

        methods.iter().for_each(|method| {
            if let Ok(method) = http::method::Method::from_str(method) {
                home.method_filters.insert(
                    method.clone(),
                    MethodFilter {
                        method: method.clone(),
                        selected: false,
                        name: method.to_string(),
                    },
                );
            }
        });

        Ok(home)
    }

    pub fn get_filter_source(&self) -> &FilterSource {
        &self.filter_source
    }

    pub fn set_filter_source(&mut self, f: FilterSource) {
        self.filter_source = f
    }

    fn mark_trace_as_timed_out(&mut self, id: String) {
        let selected_trace = self.items.iter().find(|trace| trace.id == id);

        if selected_trace.is_some() {
            let selected_trace = selected_trace.unwrap().clone();

            let mut http_trace = selected_trace.http.as_ref().unwrap().clone();

            if http_trace.state == State::Sent {
                http_trace.state = State::Timeout;
                http_trace.status = None;
                http_trace.response_body = Some("TIMEOUT WAITING FOR RESPONSE".to_string());
                http_trace.pretty_response_body = Some("TIMEOUT WAITING FOR RESPONSE".to_string());
                self.items.replace(selected_trace);
            };
        }
    }
}

impl Component for Home {
    fn on_mount(&mut self) -> Result<Option<Action>, Box<dyn Error>> {
        Ok(Some(Action::OnMount))
    }

    fn register_action_handler(
        &mut self,
        tx: UnboundedSender<Action>,
    ) -> Result<(), Box<dyn Error>> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>, Box<dyn Error>> {
        let r = match event {
            Some(Event::Key(key_event)) => self.handle_key_events(key_event)?,
            _ => None,
        };
        Ok(r)
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>, Box<dyn Error>> {
        // TODO: this should be handled as a separate application mode
        if self.active_block == ActiveBlock::SearchQuery {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => return Ok(Some(Action::ExitSearch)),
                KeyCode::Backspace => return Ok(Some(Action::DeleteSearchQuery)),
                KeyCode::Char(char) => return Ok(Some(Action::UpdateSearchQuery(char))),
                _ => return Ok(None),
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>, Box<dyn Error>> {
        let metadata = self
            .metadata
            .as_ref()
            .unwrap_or(&handlers::HandlerMetadata {
                main_height: 0,
                response_body_rectangle_height: 0,
                response_body_rectangle_width: 0,
                request_body_rectangle_height: 0,
                request_body_rectangle_width: 0,
            })
            .clone();

        match action {
            Action::Quit => match self.active_block {
                ActiveBlock::Help | ActiveBlock::Debug | ActiveBlock::Filter(_) => {
                    let last_block = self.previous_blocks.pop();

                    if last_block.is_none() {
                        return Ok(Some(Action::QuitApplication));
                    }

                    self.filter_index = 0;

                    self.active_block = last_block.unwrap();
                }
                _ => return Ok(Some(Action::QuitApplication)),
            },
            Action::NextSection => handlers::handle_tab(self),
            Action::OnMount => handlers::handle_adjust_scroll_bar(self, metadata),
            Action::Help => handlers::handle_help(self),
            Action::ToggleDebug => handlers::handle_debug(self),
            Action::Select => handlers::handle_select(self),
            Action::HandleFilter(l) => handlers::handle_general_status(self, l.to_string()),
            Action::OpenFilter => {
                let current_block = self.active_block;

                self.previous_blocks.push(current_block);

                self.active_block = ActiveBlock::Filter(crate::app::FilterScreen::FilterMain)
            }
            Action::OpenSort => {
                let current_block = self.active_block;

                self.previous_blocks.push(current_block);

                self.active_block = ActiveBlock::Sort;
            }
            Action::DeleteItem => handlers::handle_delete_item(self),
            Action::CopyToClipBoard => handlers::handle_yank(self, self.action_tx.clone()),
            Action::GoToEnd => handlers::handle_go_to_end(self, metadata),
            Action::GoToStart => handlers::handle_go_to_start(self),
            Action::PreviousSection => handlers::handle_back_tab(self),
            Action::NextPane => handlers::handle_pane_next(self),
            Action::PreviousPane => handlers::handle_pane_prev(self),
            Action::NewSearch => handlers::handle_new_search(self),
            Action::UpdateSearchQuery(c) => handlers::handle_search_push(self, c),
            Action::DeleteSearchQuery => handlers::handle_search_pop(self),
            Action::ExitSearch => handlers::handle_search_exit(self),
            Action::ShowTraceDetails => handlers::handle_enter(self),
            Action::FocusOnTraces => handlers::handle_esc(self),
            Action::StopWebSocketServer => self.wss_connected = false,
            Action::StartWebSocketServer => self.wss_connected = true,
            Action::SetGeneralStatus(s) => handlers::handle_general_status(self, s),
            Action::SetWebsocketStatus(s) => self.wss_state = s,
            Action::NavigateUp(Some(key)) => handlers::handle_up(self, key, metadata),
            Action::NavigateDown(Some(key)) => handlers::handle_down(self, key, metadata),
            Action::NavigateLeft(Some(key)) => handlers::handle_left(self, key, metadata),
            Action::NavigateRight(Some(key)) => handlers::handle_right(self, key, metadata),
            Action::UpdateMeta(metadata) => self.metadata = Some(metadata),
            Action::ClearStatusMessage => {
                self.status_message = None;
            }
            Action::GoToRight => handlers::handle_go_to_right(self, metadata),
            Action::GoToLeft => handlers::handle_go_to_left(self),
            Action::AddTrace(trace) => {
                self.items.replace(trace);

                handlers::handle_adjust_scroll_bar(self, metadata);
            }
            Action::MarkTraceAsTimedOut(id) => self.mark_trace_as_timed_out(id),
            _ => {}
        }

        Ok(None)
    }

    fn render(&self, frame: &mut Frame) -> Result<(), Box<dyn Error>> {
        match self.active_block {
            ActiveBlock::Help => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(frame.size());

                render::render_help(self, frame, main_layout[0]);
            }
            ActiveBlock::Filter(crate::app::FilterScreen::FilterMain) => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(frame.size());

                render::render_filters(self, frame, main_layout[0]);
            }
            ActiveBlock::Filter(crate::app::FilterScreen::FilterSource) => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(frame.size());

                render::render_filters_source(self, frame, main_layout[0]);
            }
            ActiveBlock::Filter(crate::app::FilterScreen::FilterStatus) => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(frame.size());

                render::render_filters_status(self, frame, main_layout[0]);
            }
            ActiveBlock::Filter(crate::app::FilterScreen::FilterMethod) => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(frame.size());

                render::render_filters_method(self, frame, main_layout[0]);
            }
            ActiveBlock::Sort => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(frame.size());

                render::render_sort(self, frame, main_layout[0]);
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

                    render::render_footer(self, frame, main_layout[1]);

                    render::render_search(self, frame);

                    let _ = self.action_tx.as_ref().unwrap().send(Action::UpdateMeta(
                        handlers::HandlerMetadata {
                            main_height: split_layout[0].height,
                            response_body_rectangle_height: response_layout[1].height,
                            response_body_rectangle_width: response_layout[1].width,
                            request_body_rectangle_height: request_layout[1].height,
                            request_body_rectangle_width: request_layout[1].width,
                        },
                    ));
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
                    render::render_footer(self, frame, main_layout[4]);

                    let _ = self.action_tx.as_ref().unwrap().send(Action::UpdateMeta(
                        handlers::HandlerMetadata {
                            main_height: main_layout[0].height,
                            response_body_rectangle_height: response_layout[1].height,
                            response_body_rectangle_width: response_layout[1].width,
                            request_body_rectangle_height: request_layout[1].height,
                            request_body_rectangle_width: request_layout[1].width,
                        },
                    ));
                }
            }
        };

        Ok(())
    }
}
