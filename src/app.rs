use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::widgets::ScrollbarState;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

use crate::components::component::Component;
use crate::components::handlers::HandlerMetadata;
use crate::components::home::{Home, WebSockerInternalState};
use crate::services::websocket::{Client, Trace};
use crate::tui::{Event, Tui};
use crate::wss::client;

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum RequestDetailsPane {
    Query,
    #[default]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    #[serde(skip)]
    Error(String),
    CopyToClipBoard,
    NavigateLeft(Option<KeyEvent>),
    NavigateDown(Option<KeyEvent>),
    NavigateUp(Option<KeyEvent>),
    NavigateRight(Option<KeyEvent>),
    GoToRight,
    GoToLeft,
    GoToEnd,
    GoToStart,
    NextSection,
    PreviousSection,
    Quit,
    #[serde(skip)]
    QuitApplication,
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
    SetWebsocketStatus(WebSockerInternalState),
    #[serde(skip)]
    MarkTraceAsTimedOut(String),
    #[serde(skip)]
    ClearStatusMessage,
    #[serde(skip)]
    AddTrace(Trace),
    AddTraceError,
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
        // NOTE: Why we need this to be mutable?
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
            client(Some(action_to_clone)).await;
        });

        loop {
            let event = t.next().await;

            if let Some(Event::Render) = event {
                for component in self.components.iter() {
                    let c = component.lock().await;
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
