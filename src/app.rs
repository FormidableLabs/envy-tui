use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use crossterm::event::KeyEvent;
use http::Method;
use ratatui::widgets::ScrollbarState;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIs, EnumIter};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

use crate::components::component::Component;
use crate::components::handlers::HandlerMetadata;
use crate::components::home::Home;
use crate::services::websocket::{Client, Trace};
use crate::tui::{Event, Tui};
use crate::wss::client;

#[derive(
    Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize, Display, EnumIs, EnumIter,
)]
#[repr(u8)]
pub enum DetailsPane {
    #[default]
    #[strum(serialize = "REQUEST DETAILS")]
    RequestDetails = 0,
    #[strum(serialize = "QUERY PARAMS")]
    QueryParams,
    #[strum(serialize = "REQUEST HEADERS")]
    RequestHeaders,
    #[strum(serialize = "RESPONSE DETAILS")]
    ResponseDetails,
    #[strum(serialize = "RESPONSE HEADERS")]
    ResponseHeaders,
    #[strum(serialize = "TIMING")]
    Timing,
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum Mode {
    #[default]
    Debug,
    Normal,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterScreen {
    #[default]
    Main,
    Method,
    Source,
    Status,
    Actions,
}

impl Display for FilterScreen {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortScreen {
    #[default]
    Source,
    Direction,
    Actions,
}

#[derive(Clone, Copy, Default, Debug, Eq, PartialEq, Serialize, Deserialize, strum_macros::EnumIs)]
pub enum ActiveBlock {
    #[default]
    Traces,
    Details,
    RequestBody,
    ResponseBody,
    Help,
    Debug,
    Filter(FilterScreen),
    Sort(SortScreen),
    SearchQuery,
}

#[derive(Default, Clone)]
pub struct UIState {
    pub index: usize,
    pub offset: usize,
    pub height: u16,
    pub width: u16,
    pub horizontal_offset: usize,
    pub scroll_state: ScrollbarState,
    pub horizontal_scroll_state: ScrollbarState,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    #[serde(skip)]
    Error(String),
    #[default]
    CopyToClipBoard,
    NavigateLeft(Option<KeyEvent>),
    NavigateDown(Option<KeyEvent>),
    NavigateUp(Option<KeyEvent>),
    NavigateRight(Option<KeyEvent>),
    GoToRight,
    GoToLeft,
    GoToEnd,
    HandleFilter(FilterScreen),
    OpenFilter,
    OpenSort,
    Select,
    GoToStart,
    NextSection,
    PreviousSection,
    Quit,
    #[serde(skip)]
    QuitApplication,
    NewSearch,
    UpdateSearchQuery(char),
    UpdateFilter,
    UpdateSort,
    SelectSortSource(SortSource),
    SelectSortDirection(SortDirection),
    DeleteSearchQuery,
    ExitSearch,
    Help,
    ToggleDebug,
    DeleteItem,
    FocusOnTraces,
    SelectTrace(Option<Trace>),
    UpdateTraceIndex(usize),
    ShowTraceDetails,
    NextDetailsTab,
    PreviousDetailsTab,
    ScheduleStartWebSocketServer,
    ScheduleStopWebSocketServer,
    StartWebSocketServer,
    #[serde(skip)]
    OnMount,
    StopWebSocketServer,
    UpdateMeta(HandlerMetadata),
    #[serde(skip)]
    SetGeneralStatus(String),
    #[serde(skip)]
    SetWebsocketStatus(WebSocketInternalState),
    #[serde(skip)]
    MarkTraceAsTimedOut(String),
    #[serde(skip)]
    ClearStatusMessage,
    #[serde(skip)]
    AddTrace(Trace),
    AddTraceError,
    ExpandAll,
    CollapseAll,
    ActivateBlock(ActiveBlock),
    PopOutDetailsTab(DetailsPane),
    CloseDetailsPane(DetailsPane),
}

#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub enum WebSocketInternalState {
    Connected(usize),
    Open,
    #[default]
    Closed,
}

#[derive(Clone, PartialEq, Debug, Eq, Default)]
pub enum SourceFilter {
    #[default]
    All,
    Applied(HashSet<String>),
}

#[derive(Clone)]
pub struct TraceFilter {
    pub source: SourceFilter,
    pub method: HashMap<Method, MethodFilter>,
    pub status: HashMap<String, StatusFilter>,
}

impl Default for TraceFilter {
    fn default() -> Self {
        let mut method: HashMap<Method, MethodFilter> = HashMap::new();
        let mut status: HashMap<String, StatusFilter> = HashMap::new();

        vec!["1xx", "2xx", "3xx", "4xx", "5xx"].iter().for_each(|http_status| {
            status.insert(
                http_status.to_string(),
                StatusFilter {
                    status: http_status.to_string(),
                    selected: false,
                    name: http_status.to_string(),
                },
            );
        });

        vec![
            Method::POST,
            Method::GET,
            Method::DELETE,
            Method::PUT,
            Method::PATCH,
            Method::OPTIONS,
        ].iter().for_each(|http_method| {
            method.insert(
                http_method.clone(),
                MethodFilter {
                    method: http_method.clone(),
                    selected: false,
                    name: http_method.to_string(),
                },
            );
        });

        Self {
            source: SourceFilter::default(),
            method,
            status,
        }
    }
}

#[derive(Clone, Default)]
pub struct MethodFilter {
    pub method: Method,
    pub name: String,
    pub selected: bool,
}

#[derive(Clone, Default)]
pub struct StatusFilter {
    pub status: String,
    pub name: String,
    pub selected: bool,
}

#[derive(
    Default,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    strum_macros::AsRefStr,
)]
pub enum SortDirection {
    Ascending,
    #[default]
    Descending,
}

#[derive(
    Default,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    strum_macros::AsRefStr,
)]
pub enum SortSource {
    Method,
    Status,
    Source,
    Url,
    Duration,
    #[default]
    Timestamp,
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct TraceSort {
    pub source: SortSource,
    pub direction: SortDirection,
}

impl Display for TraceSort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.source, self.direction)
    }
}

impl Display for SortDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortDirection::Ascending => write!(f, "↑"),
            SortDirection::Descending => write!(f, "↓"),
        }
    }
}

impl Display for SortSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortSource::Timestamp => write!(f, "Timestamp"),
            SortSource::Method => write!(f, "Method"),
            SortSource::Status => write!(f, "Status"),
            SortSource::Duration  => write!(f, "Duration"),
            SortSource::Source =>write!(f, "Source"),
            SortSource::Url => write!(f, "Url"),

        }
    }
}

#[derive(Default)]
pub struct Services {
    websocket_client: Arc<Mutex<Client>>,
}

#[derive(Default)]
pub struct App {
    pub action_tx: Option<UnboundedSender<Action>>,
    pub components: Vec<Arc<Mutex<dyn Component>>>,
    pub services: Services,
    pub is_first_render: bool,
    pub logs: Vec<String>,
    pub mode: Mode,
    pub key_map: HashMap<KeyEvent, Action>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Result<App, Box<dyn Error>> {
        let config = crate::config::Config::new()?;

        let home = Arc::new(Mutex::new(Home::new()?));

        let websocket_client = Arc::new(Mutex::new(Client::new()));

        let app = App {
            components: vec![home],
            services: Services { websocket_client },
            key_map: config.mapping.0,
            ..Self::default()
        };

        Ok(app)
    }

    fn register_action_handler(
        &mut self,
        tx: UnboundedSender<Action>,
    ) -> Result<(), Box<dyn Error>> {
        self.action_tx = Some(tx);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // NOTE: Why do we need this to be mutable?
        let (action_tx, mut action_rx) = mpsc::unbounded_channel();

        self.register_action_handler(action_tx.clone())?;

        self.services
            .websocket_client
            .lock()
            .await
            .register_action_handler(action_tx.clone())?;

        self.services.websocket_client.lock().await.start();

        let mut t = Tui::new();

        t.enter()?;

        for component in self.components.iter() {
            component
                .lock()
                .await
                .register_action_handler(action_tx.clone())?;
        }

        self.services.websocket_client.lock().await.init();

        let action_to_clone = self.action_tx.as_ref().unwrap().clone();

        tokio::spawn(async move {
            // TODO(vandosant) Propagate errors with a Result type to update the connection status
            // and optionally retry connecting
            // https://users.rust-lang.org/t/propagating-errors-from-tokio-tasks/41723/4
            client(Some(action_to_clone))
                .await
                .expect("Failed to broadcast action");
        });

        loop {
            let event = t.next().await;

            if let Some(Event::Render) = event {
                for component in self.components.iter() {
                    let mut c = component.lock().await;
                    t.terminal.draw(|frame| {
                        let r = c.render(frame, frame.size());
                        if let Err(e) = r {
                            action_tx
                                .send(Action::Error(format!("Failed to draw: {:?}", e)))
                                .unwrap();
                        }
                    })?;
                }
            };

            if let Some(Event::OnMount) = event {
                for component in self.components.iter() {
                    if let Some(action) = component.lock().await.on_mount()? {
                        action_tx.send(action.clone())?;
                    }
                }
            };

            if let Some(Event::Key(key_event)) = event {
                if let Some(action) = self.key_map.get(&key_event) {
                    let action_with_value = match action {
                        Action::NavigateUp(None) => Action::NavigateUp(Some(key_event)),
                        Action::NavigateDown(None) => Action::NavigateDown(Some(key_event)),
                        Action::NavigateLeft(None) => Action::NavigateLeft(Some(key_event)),
                        Action::NavigateRight(None) => Action::NavigateRight(Some(key_event)),
                        _ => action.clone(),
                    };
                    action_tx.send(action_with_value.clone()).unwrap();
                }
            };

            for component in self.components.iter() {
                if let Some(action) = component.lock().await.handle_events(event)? {
                    action_tx.send(action.clone())?;
                }
            }

            // Consume all actions that have been broadcast
            while let Ok(action) = action_rx.try_recv() {
                if let Action::QuitApplication = action {
                    self.should_quit = true;
                }

                for component in self.components.iter() {
                    if let Some(action) = component.lock().await.update(action.clone())? {
                        action_tx.send(action.clone())?;
                    }
                }

                self.services
                    .websocket_client
                    .clone()
                    .lock()
                    .await
                    .update(action.clone());
            }

            if self.should_quit {
                break;
            }
        }

        t.exit()?;

        Ok(())
    }
}
