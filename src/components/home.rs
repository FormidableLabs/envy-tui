use std::collections::{BTreeSet, HashMap};
use std::error::Error;

use chrono::prelude::DateTime;
use crossterm::event::{KeyCode, KeyEvent};
use http::{HeaderName, HeaderValue};
use ratatui::{
    layout::Layout,
    prelude::{Constraint, Direction, Rect},
    widgets::ListState,
};
use strum::IntoEnumIterator;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;

use crate::{
    app::{
        Action, ActiveBlock, DetailsPane, FilterScreen, Mode, SortDirection, SortScreen,
        SortSource, TraceFilter, TraceSort, UIState, WebSocketInternalState,
    },
    components::actionable_list::{ActionableList, ActionableListItem},
    components::component::Component,
    components::handlers,
    components::jsonviewer,
    config::{Colors, Config},
    render,
    services::websocket::{State, Trace},
    tui::{Event, Frame},
    utils::parse_query_params,
};
#[derive(Default)]
pub struct Home {
    pub active_block: ActiveBlock,
    pub action_tx: Option<UnboundedSender<Action>>,
    pub previous_blocks: Vec<ActiveBlock>,
    pub items: BTreeSet<Trace>,
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
    pub wss_state: WebSocketInternalState,
    pub request_json_viewer: jsonviewer::JSONViewer,
    pub response_json_viewer: jsonviewer::JSONViewer,
    pub selected_trace: Option<Trace>,
    pub filter_actions: ActionableList,
    pub filters: TraceFilter,
    pub selected_filters: TraceFilter,
    pub filter_source_index: usize,
    pub filter_value_index: usize,
    pub filter_value_screen: FilterScreen,
    pub sort: TraceSort,
    pub selected_sort: TraceSort,
    pub sort_actions: ActionableList,
    pub sort_directions: ActionableList,
    pub sort_sources: ActionableList,
    pub metadata: Option<handlers::HandlerMetadata>,
    pub details_block: DetailsPane,
    pub details_tabs: Vec<DetailsPane>,
    pub details_tab_index: usize,
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

        let home = Home {
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
            filter_actions: ActionableList::with_items(vec![ActionableListItem::with_label(
                "apply",
            )
            .with_action(Action::UpdateFilter)]),
            sort_sources: ActionableList::with_items(vec![
                ActionableListItem::with_label(SortSource::Method.as_ref())
                    .with_action(Action::SelectSortSource(SortSource::Method)),
                ActionableListItem::with_label(SortSource::Status.as_ref())
                    .with_action(Action::SelectSortSource(SortSource::Status)),
                ActionableListItem::with_label(SortSource::Source.as_ref())
                    .with_action(Action::SelectSortSource(SortSource::Source)),
                ActionableListItem::with_label(SortSource::Url.as_ref())
                    .with_action(Action::SelectSortSource(SortSource::Url)),
                ActionableListItem::with_label(SortSource::Duration.as_ref())
                    .with_action(Action::SelectSortSource(SortSource::Duration)),
                ActionableListItem::with_label(SortSource::Timestamp.as_ref())
                    .with_action(Action::SelectSortSource(SortSource::Timestamp)),
            ])
            .with_scroll_state(ListState::default().with_selected(Some(0)))
            .with_select_labels(),
            sort_directions: ActionableList::with_items(vec![
                ActionableListItem::with_label(SortDirection::Ascending.as_ref())
                    .with_action(Action::SelectSortDirection(SortDirection::Ascending)),
                ActionableListItem::with_label(SortDirection::Descending.as_ref())
                    .with_action(Action::SelectSortDirection(SortDirection::Descending)),
            ])
            .with_scroll_state(ListState::default().with_selected(Some(0)))
            .with_select_labels(),
            sort_actions: ActionableList::with_items(vec![
                ActionableListItem::with_label("apply").with_action(Action::UpdateSort)
            ]),
            details_tabs: DetailsPane::iter().collect(),
            details_panes: vec![],
            ..Self::default()
        };

        Ok(home)
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

    fn reset_active_pane(&mut self, pane: DetailsPane) {
        match pane {
            DetailsPane::QueryParams => self.query_params_list.reset(),
            DetailsPane::RequestDetails => self.request_details_list.reset(),
            DetailsPane::RequestHeaders => self.request_headers_list.reset(),
            DetailsPane::ResponseDetails => self.response_details_list.reset(),
            DetailsPane::ResponseHeaders => self.response_headers_list.reset(),
            DetailsPane::Timing => {}
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

            rows.push(ActionableListItem::with_labelled_value("sent", &sent));
            rows.push(ActionableListItem::with_labelled_value("host", &host));
            rows.push(ActionableListItem::with_labelled_value("path", &path));
            rows.push(ActionableListItem::with_labelled_value("port", &port));
            // add available actions to the item list
            if self.details_tabs.contains(&DetailsPane::RequestDetails) {
                rows.push(
                    ActionableListItem::with_labelled_value("actions", "pop-out [↗]")
                        .with_action(Action::PopOutDetailsTab(DetailsPane::RequestDetails)),
                )
            } else {
                rows.push(
                    ActionableListItem::with_labelled_value("actions", "close [x]")
                        .with_action(Action::CloseDetailsPane(DetailsPane::RequestDetails)),
                )
            };

            self.request_details_list = ActionableList::with_items(rows);

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

            let mut next_items: Vec<ActionableListItem> = raw_params
                .into_iter()
                .map(|(label, value)| ActionableListItem::with_labelled_value(&label, &value))
                .to_owned()
                .collect();

            if self.details_tabs.contains(&DetailsPane::QueryParams) {
                next_items.push(
                    ActionableListItem::with_labelled_value("actions", "pop-out [↗]")
                        .with_action(Action::PopOutDetailsTab(DetailsPane::QueryParams)),
                )
            } else {
                next_items.push(
                    ActionableListItem::with_labelled_value("actions", "close [x]")
                        .with_action(Action::CloseDetailsPane(DetailsPane::QueryParams)),
                )
            };

            self.query_params_list = ActionableList::with_items(next_items);

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

            items.push(ActionableListItem::with_labelled_value(
                "received", &received,
            ));
            items.push(ActionableListItem::with_labelled_value("status", &status));
            items.push(ActionableListItem::with_labelled_value(
                "duration", &duration,
            ));

            if self.details_tabs.contains(&DetailsPane::ResponseDetails) {
                items.push(
                    ActionableListItem::with_labelled_value("actions", "pop-out [↗]")
                        .with_action(Action::PopOutDetailsTab(DetailsPane::ResponseDetails)),
                )
            } else {
                items.push(
                    ActionableListItem::with_labelled_value("actions", "close [x]")
                        .with_action(Action::CloseDetailsPane(DetailsPane::ResponseDetails)),
                )
            };

            self.response_details_list = ActionableList::with_items(items);

            // REQUEST HEADERS PANE
            let headers = trace.http.clone().unwrap_or_default().request_headers;
            let mut parsed_headers = headers.iter().collect::<Vec<(&HeaderName, &HeaderValue)>>();
            parsed_headers.sort_by(|a, b| {
                let (name_a, _) = a;
                let (name_b, _) = b;

                name_a.to_string().cmp(&name_b.to_string())
            });
            let mut next_items: Vec<ActionableListItem> = parsed_headers
                .into_iter()
                .map(|(label, value)| {
                    ActionableListItem::with_labelled_value(
                        label.as_str(),
                        value.to_str().unwrap_or("Unknown header value"),
                    )
                })
                .to_owned()
                .collect();
            // add available actions to the item list
            if self.details_tabs.contains(&DetailsPane::RequestHeaders) {
                next_items.push(
                    ActionableListItem::with_labelled_value("actions", "pop-out [↗]")
                        .with_action(Action::PopOutDetailsTab(DetailsPane::RequestHeaders)),
                )
            } else {
                next_items.push(
                    ActionableListItem::with_labelled_value("actions", "close [x]")
                        .with_action(Action::CloseDetailsPane(DetailsPane::RequestHeaders)),
                )
            };

            self.request_headers_list = ActionableList::with_items(next_items);

            // RESPONSE HEADERS PANE
            let headers = trace.http.clone().unwrap_or_default().response_headers;
            let mut parsed_headers = headers.iter().collect::<Vec<(&HeaderName, &HeaderValue)>>();
            parsed_headers.sort_by(|a, b| {
                let (name_a, _) = a;
                let (name_b, _) = b;

                name_a.to_string().cmp(&name_b.to_string())
            });
            let mut next_items: Vec<ActionableListItem> = parsed_headers
                .into_iter()
                .map(|(label, value)| {
                    ActionableListItem::with_labelled_value(
                        label.as_str(),
                        value.to_str().unwrap_or("Unknown header value"),
                    )
                })
                .to_owned()
                .collect();

            // add available actions to the item list
            if self.details_tabs.contains(&DetailsPane::ResponseHeaders) {
                next_items.push(
                    ActionableListItem::with_labelled_value("actions", "pop-out [↗]")
                        .with_action(Action::PopOutDetailsTab(DetailsPane::ResponseHeaders)),
                )
            } else {
                next_items.push(
                    ActionableListItem::with_labelled_value("actions", "close [x]")
                        .with_action(Action::CloseDetailsPane(DetailsPane::ResponseHeaders)),
                )
            };

            self.response_headers_list = ActionableList::with_items(next_items);

            // TIMING PANE
            let next_items: Vec<ActionableListItem> = vec![
                ActionableListItem::with_label("blocked"),
                ActionableListItem::with_label("DNS"),
                ActionableListItem::with_label("connecting"),
                ActionableListItem::with_label("TLS"),
                ActionableListItem::with_label("sending"),
                ActionableListItem::with_label("waiting"),
                ActionableListItem::with_label("receiving"),
            ];

            self.timing_list = ActionableList::with_items(next_items);
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
                if self.active_block.is_filter() {
                    return Ok(None);
                }

                self.filter_source_index = 0;
                self.filter_value_index = 0;
                self.selected_filters = TraceFilter::default();
                self.previous_blocks.push(self.active_block);
                self.active_block = ActiveBlock::Filter(FilterScreen::Main);

                Ok(None)
            }
            Action::OpenSort => {
                if self.active_block.is_sort() {
                    return Ok(None);
                }

                self.sort_sources.reset();
                self.sort_sources.top(0);
                self.sort_directions.reset();
                self.sort_directions.top(0);
                self.selected_sort = TraceSort::default();
                self.previous_blocks.push(self.active_block);
                self.active_block = ActiveBlock::Sort(SortScreen::Source);

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
                self.details_panes.push(pane);
                self.details_tabs.retain(|&d| pane != d);
                self.details_tab_index = self.details_tab_index.saturating_sub(1);

                self.update_details_lists();
                self.reset_active_pane(pane);

                Ok(None)
            }
            Action::CloseDetailsPane(pane) => {
                self.details_panes.retain(|&d| pane != d);
                self.details_tabs.push(pane);
                self.details_tab_index = self.details_tabs.len() - 1;

                self.update_details_lists();
                self.reset_active_pane(pane);

                Ok(None)
            }
            Action::ActivateBlock(block) => {
                if block == ActiveBlock::Sort(SortScreen::Actions) {
                    self.sort_actions.next();
                } else {
                    self.sort_actions.reset();
                }

                if block == ActiveBlock::Filter(FilterScreen::Actions) {
                    self.filter_actions.next();
                } else {
                    self.filter_actions.reset();
                }

                self.active_block = block;

                Ok(None)
            }
            Action::SelectSortDirection(direction) => {
                if let Some(next) = self
                    .sort_directions
                    .items
                    .iter()
                    .position(|item| item.label == direction.to_string())
                {
                    self.sort_directions.select(next);
                }

                self.selected_sort = TraceSort {
                    direction,
                    source: self.selected_sort.source.clone(),
                };

                Ok(Some(Action::ActivateBlock(ActiveBlock::Sort(
                    SortScreen::Actions,
                ))))
            }
            Action::SelectSortSource(source) => {
                if let Some(next) = self
                    .sort_sources
                    .items
                    .iter()
                    .position(|item| item.label == source.to_string())
                {
                    self.sort_sources.select(next);
                }

                self.selected_sort = TraceSort {
                    direction: self.selected_sort.direction.clone(),
                    source,
                };

                Ok(Some(Action::ActivateBlock(ActiveBlock::Sort(
                    SortScreen::Direction,
                ))))
            }
            Action::UpdateSort => {
                self.sort_directions.select(0);

                self.sort = self.selected_sort.clone();
                Ok(Some(Action::ActivateBlock(ActiveBlock::Traces)))
            }
            Action::UpdateFilter => {
                self.filters = self.selected_filters.clone();
                Ok(Some(Action::ActivateBlock(ActiveBlock::Traces)))
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
            ActiveBlock::Filter(_) => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(frame.size());

                render::render_filters(self, frame, main_layout[0]);
            }
            ActiveBlock::Sort(_) => {
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
                        );

                    let [left_column, right_column] = main_columns.areas(main_layout[0]);

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
