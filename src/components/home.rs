use std::error::Error;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    str::FromStr,
};

use chrono::prelude::DateTime;
use crossterm::event::{KeyCode, KeyEvent};
use http::{HeaderName, HeaderValue};
use ratatui::{
    layout::Layout,
    prelude::{Constraint, Direction, Rect},
};
use strum::IntoEnumIterator;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;

use crate::{
    app::{Action, ActiveBlock, DetailsPane, Mode, UIState},
    components::actionable_list::{ActionableList, ActionableListItem},
    components::component::Component,
    components::handlers,
    components::jsonviewer,
    config::{Colors, Config},
    render,
    services::websocket::{State, Trace},
    tui::{Event, Frame},
    utils::{parse_query_params, TraceSort},
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
    pub active_block: ActiveBlock,
    pub action_tx: Option<UnboundedSender<Action>>,
    pub previous_blocks: Vec<ActiveBlock>,
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
    pub colors: Colors,
    pub status_message: Option<String>,
    pub ws_status: String,
    pub wss_connected: bool,
    pub wss_connection_count: usize,
    pub wss_state: WebSockerInternalState,
    pub request_json_viewer: jsonviewer::JSONViewer,
    pub response_json_viewer: jsonviewer::JSONViewer,
    pub selected_trace: Option<Trace>,
    pub filter_index: usize,
    pub sort_index: usize,
    pub metadata: Option<handlers::HandlerMetadata>,
    pub filter_source: FilterSource,
    pub method_filters: HashMap<http::method::Method, MethodFilter>,
    pub status_filters: HashMap<String, StatusFilter>,
    pub order: TraceSort,
    pub details_block: DetailsPane,
    pub details_tabs: Vec<DetailsPane>,
    pub details_panes: Vec<DetailsPane>,
    pub request_details_list: ActionableList,
    pub query_params_list: ActionableList,
    pub request_headers_list: ActionableList,
    pub response_details_list: ActionableList,
    pub response_headers_list: ActionableList,
    pub timing_list: ActionableList,
}

impl Home {
    pub fn new() -> Result<Home, Box<dyn Error>> {
        let config = Config::new()?;
        let mut home = Home {
            key_map: config.mapping.0,
            colors: config.colors.clone(),
            request_json_viewer: jsonviewer::JSONViewer::new(
                ActiveBlock::RequestBody,
                4,
                "Request body",
                config.colors.clone(),
            )?,
            response_json_viewer: jsonviewer::JSONViewer::new(
                ActiveBlock::ResponseBody,
                4,
                "Response body",
                config.colors.clone(),
            )?,
            details_tabs: DetailsPane::iter().collect(),
            details_panes: vec![],
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

    fn update_details_lists(&mut self) {
        if let Some(trace) = &self.selected_trace {
            // REQUEST DETAILS PANE
            let mut rows: Vec<ActionableListItem> = vec![];

            let sent = DateTime::from_timestamp(trace.timestamp, 0)
                .unwrap_or_default()
                .format("%Y-%m-%d @ %H:%M:%S")
                .to_string();
            let host = trace.service_name.clone().unwrap_or(format!(""));
            let path = trace.http.clone().map_or("".to_string(), |http| http.path);
            let port = trace.http.clone().map_or("".to_string(), |http| http.port);

            rows.push((("sent".into(), sent), None));
            rows.push((("host".into(), host), None));
            rows.push((("path".into(), path), None));
            rows.push((("port".into(), port), None));
            // add available actions to the item list
            let actions = if self.details_tabs.contains(&DetailsPane::RequestDetails) {
                vec![(
                    ("actions".to_string(), "pop-out [↗]".to_string()),
                    Some(Action::PopOutDetailsTab(DetailsPane::RequestDetails)),
                )]
            } else {
                vec![(
                    ("actions".to_string(), "close [x]".to_string()),
                    Some(Action::CloseDetailsPane(DetailsPane::RequestDetails)),
                )]
            };

            self.request_details_list = ActionableList::with_items(rows, actions);

            // QUERY PARAMS PANE
            let mut raw_params = parse_query_params(
                trace
                    .http
                    .clone()
                    .expect("Missing http from trace")
                    .uri
                    .to_string(),
            );

            raw_params.sort_by(|a, b| {
                let (name_a, _) = a;
                let (name_b, _) = b;

                name_a.cmp(name_b)
            });

            let next_items: Vec<ActionableListItem> = raw_params
                .into_iter()
                .map(|(label, name)| ((label, name), None))
                .to_owned()
                .collect();

            // add available actions to the item list
            let next_actions: Vec<ActionableListItem> =
                if self.details_tabs.contains(&DetailsPane::QueryParams) {
                    vec![(
                        ("actions".to_string(), "pop-out [↗]".to_string()),
                        Some(Action::PopOutDetailsTab(DetailsPane::QueryParams)),
                    )]
                } else {
                    vec![(
                        ("actions".to_string(), "close [x]".to_string()),
                        Some(Action::CloseDetailsPane(DetailsPane::QueryParams)),
                    )]
                };

            self.query_params_list = ActionableList::with_items(next_items, next_actions);

            // RESPONSE DETAILS PANE
            let mut items: Vec<ActionableListItem> = vec![];

            let received = DateTime::from_timestamp(trace.timestamp, 0)
                .unwrap_or_default()
                .format("%Y-%m-%d @ %H:%M:%S")
                .to_string();
            let status = trace.http.clone().map_or(None, |http| http.status).map_or(
                "".to_string(),
                |status| {
                    format!(
                        "{} {}",
                        status.as_str(),
                        status.canonical_reason().unwrap_or_default()
                    )
                },
            );
            let duration = trace
                .http
                .clone()
                .map_or(None, |http| http.duration)
                .map_or("".to_string(), |duration| format!("{}ms", duration));

            items.push((("received".into(), received), None));
            items.push((("status".into(), status), None));
            items.push((("duration".into(), duration), None));

            let actions: Vec<ActionableListItem> =
                if self.details_tabs.contains(&DetailsPane::ResponseDetails) {
                    vec![(
                        ("actions".to_string(), "pop-out [↗]".to_string()),
                        Some(Action::PopOutDetailsTab(DetailsPane::ResponseDetails)),
                    )]
                } else {
                    vec![(
                        ("actions".to_string(), "close [x]".to_string()),
                        Some(Action::CloseDetailsPane(DetailsPane::ResponseDetails)),
                    )]
                };

            self.response_details_list = ActionableList::with_items(items, actions);

            // REQUEST HEADERS PANE
            let headers = trace.http.clone().unwrap_or_default().request_headers;
            let mut parsed_headers = headers.iter().collect::<Vec<(&HeaderName, &HeaderValue)>>();
            parsed_headers.sort_by(|a, b| {
                let (name_a, _) = a;
                let (name_b, _) = b;

                name_a.to_string().cmp(&name_b.to_string())
            });
            let next_items: Vec<ActionableListItem> = parsed_headers
                .into_iter()
                .map(|(label, name)| {
                    (
                        (
                            label.as_str().to_string(),
                            name.to_str().unwrap_or("Unknown header value").to_string(),
                        ),
                        None,
                    )
                })
                .to_owned()
                .collect();
            // add available actions to the item list
            let next_actions: Vec<ActionableListItem> =
                if self.details_tabs.contains(&DetailsPane::RequestHeaders) {
                    vec![(
                        ("actions".to_string(), "pop-out [↗]".to_string()),
                        Some(Action::PopOutDetailsTab(DetailsPane::RequestHeaders)),
                    )]
                } else {
                    vec![(
                        ("actions".to_string(), "close [x]".to_string()),
                        Some(Action::CloseDetailsPane(DetailsPane::RequestHeaders)),
                    )]
                };

            self.request_headers_list = ActionableList::with_items(next_items, next_actions);

            // RESPONSE HEADERS PANE
            let headers = trace.http.clone().unwrap_or_default().response_headers;
            let mut parsed_headers = headers.iter().collect::<Vec<(&HeaderName, &HeaderValue)>>();
            parsed_headers.sort_by(|a, b| {
                let (name_a, _) = a;
                let (name_b, _) = b;

                name_a.to_string().cmp(&name_b.to_string())
            });
            let next_items: Vec<ActionableListItem> = parsed_headers
                .into_iter()
                .map(|(label, name)| {
                    (
                        (
                            label.as_str().to_string(),
                            name.to_str().unwrap_or("Unknown header value").to_string(),
                        ),
                        None,
                    )
                })
                .to_owned()
                .collect();
            // add available actions to the item list
            let next_actions: Vec<ActionableListItem> =
                if self.details_tabs.contains(&DetailsPane::ResponseHeaders) {
                    vec![(
                        ("actions".to_string(), "pop-out [↗]".to_string()),
                        Some(Action::PopOutDetailsTab(DetailsPane::ResponseHeaders)),
                    )]
                } else {
                    vec![(
                        ("actions".to_string(), "close [x]".to_string()),
                        Some(Action::CloseDetailsPane(DetailsPane::ResponseHeaders)),
                    )]
                };

            self.response_headers_list = ActionableList::with_items(next_items, next_actions);

            // TIMING PANE
            let next_items: Vec<ActionableListItem> = vec![
                (("blocked".to_string(), "".to_string()), None),
                (("DNS".to_string(), "".to_string()), None),
                (("connecting".to_string(), "".to_string()), None),
                (("TLS".to_string(), "".to_string()), None),
                (("sending".to_string(), "".to_string()), None),
                (("waiting".to_string(), "".to_string()), None),
                (("receiving".to_string(), "".to_string()), None),
            ];
            // Timing tab cannot be moved
            let actions = vec![];

            self.timing_list = ActionableList::with_items(next_items, actions);
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

        match action {
            Action::Quit => {
                let last_block = self.previous_blocks.pop();

                if last_block.is_none() {
                    return Ok(Some(Action::QuitApplication));
                }

                self.filter_index = 0;

                self.active_block = last_block.unwrap();

                Ok(None)
            }
            Action::NextSection => Ok(handlers::handle_tab(self)),
            Action::OnMount => Ok(handlers::handle_adjust_scroll_bar(self, metadata)),
            Action::Help => Ok(handlers::handle_help(self)),
            Action::ToggleDebug => Ok(handlers::handle_debug(self)),
            Action::Select => Ok(handlers::handle_select(self)),
            Action::HandleFilter(l) => Ok(handlers::handle_general_status(self, l.to_string())),
            Action::OpenFilter => {
                let current_block = self.active_block;

                self.previous_blocks.push(current_block);

                self.active_block = ActiveBlock::Filter(crate::app::FilterScreen::FilterMain);

                Ok(None)
            }
            Action::OpenSort => {
                let current_block = self.active_block;

                self.previous_blocks.push(current_block);

                self.active_block = ActiveBlock::Sort;

                Ok(None)
            }
            Action::DeleteItem => Ok(handlers::handle_delete_item(self)),
            Action::CopyToClipBoard => Ok(handlers::handle_yank(self, self.action_tx.clone())),
            Action::GoToEnd => Ok(handlers::handle_go_to_end(self, metadata)),
            Action::GoToStart => Ok(handlers::handle_go_to_start(self)),
            Action::PreviousSection => Ok(handlers::handle_back_tab(self)),
            Action::NextDetailsTab => Ok(handlers::handle_details_tab_next(self)),
            Action::PreviousDetailsTab => Ok(handlers::handle_details_tab_prev(self)),
            Action::NewSearch => Ok(handlers::handle_new_search(self)),
            Action::UpdateSearchQuery(c) => Ok(handlers::handle_search_push(self, c)),
            Action::DeleteSearchQuery => Ok(handlers::handle_search_pop(self)),
            Action::ExitSearch => Ok(handlers::handle_search_exit(self)),
            Action::ShowTraceDetails => Ok(handlers::handle_enter(self)),
            Action::FocusOnTraces => Ok(handlers::handle_esc(self)),
            Action::StopWebSocketServer => {
                self.wss_connected = false;
                Ok(None)
            }
            Action::StartWebSocketServer => {
                self.wss_connected = true;
                Ok(None)
            }
            Action::SetGeneralStatus(s) => Ok(handlers::handle_general_status(self, s)),
            Action::SetWebsocketStatus(s) => {
                self.wss_state = s;
                Ok(None)
            }
            Action::NavigateUp(Some(key)) => Ok(handlers::handle_up(self, key, metadata)),
            Action::NavigateDown(Some(key)) => Ok(handlers::handle_down(self, key, metadata)),
            Action::UpdateMeta(metadata) => {
                self.metadata = Some(metadata);
                Ok(None)
            }
            Action::ClearStatusMessage => {
                self.status_message = None;
                Ok(None)
            }
            Action::AddTrace(trace) => {
                self.items.replace(trace);
                handlers::handle_adjust_scroll_bar(self, metadata);
                Ok(None)
            }
            Action::MarkTraceAsTimedOut(id) => {
                self.mark_trace_as_timed_out(id);
                Ok(Some(Action::SelectTrace(self.selected_trace.clone())))
            }
            Action::SelectTrace(maybe_trace) => {
                self.selected_trace = maybe_trace;

                self.update_details_lists();

                Ok(None)
            }
            Action::PopOutDetailsTab(pane) => {
                if self.details_tabs.len() > 1 {
                    handlers::handle_details_tab_next(self);

                    self.details_panes.push(pane);
                    self.details_tabs.retain(|&d| pane != d);
                }

                self.update_details_lists();

                Ok(None)
            }
            Action::CloseDetailsPane(pane) => {
                handlers::handle_details_tab_prev(self);

                self.details_panes.retain(|&d| pane != d);
                self.details_tabs.push(pane);

                self.update_details_lists();

                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn render(&mut self, frame: &mut Frame, rect: Rect) -> Result<(), Box<dyn Error>> {
        match self.active_block {
            ActiveBlock::Help => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(rect);

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
                    .split(rect);

                render::render_debug(self, frame, main_layout[0]);
            }
            _ => {
                let terminal_width = frame.size().width;

                if terminal_width > 200 {
                    let main_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints(
                            [Constraint::Percentage(95), Constraint::Percentage(5)].as_ref(),
                        )
                        .split(rect);

                    let main_columns = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(35), Constraint::Percentage(65)].as_ref(),
                        )
                        .split(main_layout[0]);

                    let [left_column, right_column, ..] = main_columns[..] else { todo!() };

                    let right_column_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [Constraint::Percentage(68), Constraint::Percentage(32)].as_ref(),
                        )
                        .split(right_column);

                    let body_layout = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
                        )
                        .split(right_column_layout[1]);

                    render::render_traces(self, frame, left_column);

                    render::details(self, frame, right_column_layout[0]);
                    self.request_json_viewer.render(frame, body_layout[1])?;
                    self.response_json_viewer.render(frame, body_layout[0])?;
                    render::render_footer(self, frame, main_layout[1]);
                    render::render_search(self, frame);

                    let _ = self.action_tx.as_ref().unwrap().send(Action::UpdateMeta(
                        handlers::HandlerMetadata {
                            main_height: left_column.height,
                            response_body_rectangle_height: body_layout[0].height,
                            response_body_rectangle_width: body_layout[0].width,
                            request_body_rectangle_height: body_layout[1].height,
                            request_body_rectangle_width: body_layout[1].width,
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

                    render::details(self, frame, request_layout[0]);
                    self.request_json_viewer.render(frame, request_layout[1])?;
                    self.response_json_viewer
                        .render(frame, response_layout[1])?;
                    render::render_traces(self, frame, main_layout[0]);
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
