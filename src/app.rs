use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::widgets::ScrollbarState;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use ratatui::prelude::CrosstermBackend;

use crate::tui::{Event, Tui};
use crate::components::{home::Home, websocket::{Client, Trace}};

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
    ScheduleStartWebSocketServer,
    ScheduleStopWebSocketServer,
    StartWebSocketServer,
    StopWebSocketServer,
    #[serde(skip)]
    SetGeneralStatus(String),
    #[serde(skip)]
    SetWebsocketStatus,
    #[serde(skip)]
    MarkTraceAsTimedOut(String),
    #[serde(skip)]
    ClearStatusMessage,
    #[serde(skip)]
    AddTrace(Trace),
    AddTraceError,
}

#[derive(Default)]
pub struct Components {
    home: Arc<Mutex<Home>>,
    websocket_client: Arc<Mutex<Client>>,
}

#[derive(Default)]
pub struct App {
    pub action_tx: Option<UnboundedSender<Action>>,
    pub components: Components,
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
        let home = Arc::new(Mutex::new(Home::new()?));
        let websocket_client = Arc::new(Mutex::new(Client::default()));
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

    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<(), Box<dyn Error>> {
        self.action_tx = Some(tx);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let (action_tx, mut action_rx) = mpsc::unbounded_channel();

        if self.mode == Mode::Normal {
            self.components.websocket_client.lock().await.start();
        }

        let mut t = Tui::new();
        t.enter()?;

        self.register_action_handler(action_tx.clone())?;
        // TODO: convert components to a vector and iterate through each to register handler
        self.components.home.lock().await.register_action_handler(action_tx.clone())?;
        self.components.websocket_client.lock().await.register_action_handler(action_tx.clone())?;

        self.components.websocket_client.lock().await.insert_mock_data();

        loop {
            let event = t.next().await;

            if let Some(Event::Render) = event {
                let home = self.components.home.lock().await;
                t.terminal.draw(|frame| {
                    home.render(frame);
                })?;
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

            if let Some(action) = self.components.home.lock().await.handle_events(event)? {
                action_tx.send(action.clone())?;
            }

            while let Ok(action) = action_rx.try_recv() {
                self.components.home.lock().await.update(action.clone());
                self.components.websocket_client.lock().await.update(action.clone());
            }

            if self.components.home.lock().await.should_quit {
                break;
            }
        }

        t.exit()?;

        Ok(())
    }
}
