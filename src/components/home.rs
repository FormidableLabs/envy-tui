use std::collections::{BTreeSet, HashMap};
use std::error::Error;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Layout, Rect},
    prelude::{Constraint, Direction},
};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;

use crate::{
    app::{Action, ActiveBlock, Mode, RequestDetailsPane, ResponseDetailsPane, UIState},
    components::component::Component,
    components::handlers,
    components::jsonviewer,
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

#[derive(Default)]
pub struct Home {
    pub action_tx: Option<UnboundedSender<Action>>,
    pub active_block: ActiveBlock,
    pub previous_block: Option<ActiveBlock>,
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
    pub request_json_viewer: jsonviewer::JSONViewer,
    pub response_json_viewer: jsonviewer::JSONViewer,
    pub selected_trace: Option<Trace>,
    metadata: Option<handlers::HandlerMetadata>,
}

impl Home {
    pub fn new() -> Result<Home, Box<dyn Error>> {
        let config = crate::config::Config::new()?;
        let home = Home {
            key_map: config.mapping.0,
            request_json_viewer: jsonviewer::JSONViewer::new(
                ActiveBlock::RequestBody,
                4,
                "Request body",
            )?,
            response_json_viewer: jsonviewer::JSONViewer::new(
                ActiveBlock::ResponseBody,
                4,
                "Response body",
            )?,
            ..Self::default()
        };

        Ok(home)
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

impl Component for Home {
    fn on_mount(&mut self) -> Result<Option<Action>, Box<dyn Error>> {
        Ok(Some(Action::OnMount))
    }

    fn register_action_handler(
        &mut self,
        tx: UnboundedSender<Action>,
    ) -> Result<(), Box<dyn Error>> {
        self.request_json_viewer
            .register_action_handler(tx.clone())?;
        self.response_json_viewer
            .register_action_handler(tx.clone())?;
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
        self.request_json_viewer.update(action.clone())?;
        self.response_json_viewer.update(action.clone())?;

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

        let other_action = match action {
            Action::Quit => match self.active_block {
                ActiveBlock::Help | ActiveBlock::Debug => {
                    self.active_block = self.previous_block.unwrap_or(ActiveBlock::TracesBlock);

                    self.previous_block = None;

                    return Ok(None);
                }
                _ => return Ok(Some(Action::QuitApplication)),
            },
            Action::NextSection => return Ok(handlers::handle_tab(self)),
            Action::OnMount => handlers::handle_adjust_scroll_bar(self, metadata),
            Action::Help => handlers::handle_help(self),
            Action::ToggleDebug => handlers::handle_debug(self),
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
            Action::StopWebSocketServer => {
                self.wss_connected = false;
                None
            }
            Action::StartWebSocketServer => {
                self.wss_connected = true;
                None
            }
            Action::SetGeneralStatus(s) => handlers::handle_general_status(self, s),
            Action::SetWebsocketStatus(s) => {
                self.wss_state = s;
                None
            }
            Action::NavigateUp(Some(key)) => handlers::handle_up(self, key, metadata),
            Action::NavigateDown(Some(key)) => handlers::handle_down(self, key, metadata),
            Action::UpdateMeta(metadata) => {
                self.metadata = Some(metadata);
                None
            }
            Action::ClearStatusMessage => {
                self.status_message = None;
                None
            }
            Action::AddTrace(trace) => {
                self.items.replace(trace);
                handlers::handle_adjust_scroll_bar(self, metadata);
                None
            }
            Action::MarkTraceAsTimedOut(id) => {
                self.mark_trace_as_timed_out(id);
                Some(Action::SelectTrace(self.selected_trace.clone()))
            }
            Action::SelectTrace(trace) => {
                self.selected_trace = trace;
                None
            }
            _ => None,
        };

        Ok(other_action)
    }

    fn render(&self, frame: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
        match self.active_block {
            ActiveBlock::Help => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(rect);

                render::render_help(self, frame, main_layout[0]);
            }
            ActiveBlock::Debug => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(rect);

                render::render_debug(self, frame, main_layout[0]);
            }
            _ => {
                let terminal_width = frame.size().width;

                if terminal_width > 200 {
                    let main_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints([Constraint::Percentage(95), Constraint::Length(3)].as_ref())
                        .split(rect);

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

                    self.request_json_viewer.render(frame, request_layout[1])?;
                    self.response_json_viewer
                        .render(frame, response_layout[1])?;

                    render::render_traces(self, frame, split_layout[0]);

                    render::render_request_summary(self, frame, details_layout[0]);
                    render::render_response_block(self, frame, response_layout[0]);

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
                        .split(rect);

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
                    self.request_json_viewer.render(frame, request_layout[1])?;
                    self.response_json_viewer
                        .render(frame, response_layout[1])?;
                    render::render_traces(self, frame, main_layout[0]);

                    render::render_request_summary(self, frame, main_layout[1]);
                    render::render_response_block(self, frame, response_layout[0]);

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
