use std::collections::{BTreeSet, HashMap};
use std::error::Error;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Layout,
    prelude::{Constraint, Direction},
};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;

use crate::app::{
    Action, ActiveBlock, Frame, Mode, RequestDetailsPane, ResponseDetailsPane, UIState,
};
use crate::components::handlers;
use crate::components::websocket::Trace;
use crate::render;
use crate::tui::Event;


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
    pub should_quit: bool,
    pub status_message: Option<String>,
    pub ws_status: String,
    pub wss_connected: bool,
    pub wss_connection_count: usize,
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_action_handler(
        &mut self,
        tx: UnboundedSender<Action>,
    ) -> Result<(), Box<dyn Error>> {
        self.action_tx = Some(tx);
        Ok(())
    }

    pub fn handle_events(
        &mut self,
        event: Option<Event>,
    ) -> Result<Option<Action>, Box<dyn Error>> {
        let r = match event {
            Some(Event::Key(key_event)) => self.handle_key_event(key_event)?,
            _ => None,
        };
        Ok(r)
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>, Box<dyn Error>> {
        if self.active_block == ActiveBlock::SearchQuery {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => return Ok(Some(Action::ExitSearch)),
                KeyCode::Backspace => return Ok(Some(Action::DeleteSearchQuery)),
                KeyCode::Char(char) => return Ok(Some(Action::UpdateSearchQuery(char))),
                _ => return Ok(None),
            }
        }
        let action = match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('?') => Action::Help,
            KeyCode::Char('p') => Action::ToggleDebug,
            KeyCode::Char('d') => Action::DeleteItem,
            KeyCode::Char('y') => Action::CopyToClipBoard,
            KeyCode::Char('>') => Action::GoToEnd,
            KeyCode::Char('<') => Action::GoToStart,
            KeyCode::Tab => Action::NextSection,
            KeyCode::BackTab => Action::PreviousSection,
            KeyCode::Char(']') | KeyCode::PageUp => Action::NextPane,
            KeyCode::Char('[') | KeyCode::PageDown => Action::PreviousPane,
            KeyCode::Char('/') => Action::NewSearch,
            KeyCode::Enter => Action::ShowTraceDetails,
            KeyCode::Esc => Action::FocusOnTraces,
            KeyCode::Up | KeyCode::Char('k') => Action::NavigateUp(Some(key)),
            KeyCode::Down | KeyCode::Char('j') => Action::NavigateDown(Some(key)),
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                Action::NavigateLeft(Some(key))
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::NavigateRight(Some(key))
            }
            KeyCode::Char('X') => Action::StopWebSocketServer,
            KeyCode::Char('x') => Action::StartWebSocketServer,
            _ => return Ok(None),
        };
        Ok(Some(action))
    }

    pub fn update(&mut self, action: Action) {
        let metadata = handlers::HandlerMetadata {
            main_height: self.main.height,
            response_body_rectangle_height: self.response_body.height,
            response_body_rectangle_width: self.response_body.width,
            request_body_rectangle_height: self.request_body.height,
            request_body_rectangle_width: self.request_body.width,
        };

        match action {
            Action::Quit => match self.active_block {
                ActiveBlock::Help | ActiveBlock::Debug => {
                    self.active_block = self.previous_block.unwrap_or(ActiveBlock::TracesBlock);

                    self.previous_block = None;
                }
                _ => self.should_quit = true,
            },
            Action::NextSection => handlers::handle_tab(self),
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
            Action::StopWebSocketServer => self.wss_connected = false,
            Action::StartWebSocketServer => self.wss_connected = true,
            Action::SetGeneralStatus(s) => handlers::handle_general_status(self, s),
            Action::SetWebsocketStatus => handlers::handle_wss_status(self),
            Action::NavigateUp(Some(key)) => handlers::handle_up(self, key, metadata),
            Action::NavigateDown(Some(key)) => handlers::handle_down(self, key, metadata),
            Action::NavigateLeft(Some(key)) => handlers::handle_left(self, key, metadata),
            Action::NavigateRight(Some(key)) => handlers::handle_right(self, key, metadata),
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame<'_>) {
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

                    render::render_footer(self, frame, main_layout[1]);

                    render::render_search(self, frame);

                    // self.response_body.height = response_layout[1].height;
                    // self.response_body.width = response_layout[1].width;
                    // self.main.height = split_layout[0].height;
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
                    render::render_footer(self, frame, main_layout[1]);

                    //self.response_body.height = response_layout[1].height;
                    //self.response_body.width = response_layout[1].width;

                    //self.request_body.height = request_layout[1].height;
                    //self.request_body.width = request_layout[1].width;

                    //self.main.height = main_layout[0].height;
                }
            }
        };
        if self.is_first_render {
            // NOTE: Index and offset needs to be set prior before we call `set_content_length`.
            // self.main.index = 0;
            // self.main.offset = 0;

            // set_content_length(self);

            //self.main.scroll_state = self
            //    .main
            //    .scroll_state
            //    .content_length(self.items.len() as u16);

            //self.is_first_render = false;
        }
    }
}
