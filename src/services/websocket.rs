use crate::app::Action;
use crate::mock;
use crate::parser::{parse_raw_trace, Payload};
use crate::wss::WebSocket;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct Services {
    pub collector_server: Arc<Mutex<WebSocket>>,
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

#[derive(Clone, Debug)]
pub struct HTTPTrace {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GraphQLOperationType {
    Query,
    Mutation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphQLTrace {
    pub query: String,
    pub operation_name: Option<String>,
    pub variables: serde_json::Map<String, Value>,
    pub operation_type: GraphQLOperationType,
}

#[derive(Clone, Debug)]
pub struct Trace {
    pub id: String,
    pub timestamp: u64,
    pub service_name: Option<String>,
    pub http: Option<HTTPTrace>,
    pub graphql: Option<GraphQLTrace>,
}

impl PartialEq<Trace> for Trace {
    fn eq(&self, other: &Trace) -> bool {
        self.id == *other.id
    }
}

impl Eq for Trace {}

impl PartialOrd for Trace {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.id.cmp(&self.id))
    }
}

impl Ord for Trace {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.id.cmp(&self.id)
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
            "ID: {:?}, Time: {:?}",
            self.id,
            chrono::DateTime::from_timestamp((self.timestamp / 1000).try_into().unwrap(), 0)
        )
    }
}

#[derive(Default)]
pub struct Client {
    pub action_tx: Option<UnboundedSender<Action>>,
    pub open: bool,
    pub services: Services,
}

impl Client {
    pub fn new() -> Client {
        Client {
            action_tx: None,
            open: false,
            services: Services {
                collector_server: Arc::new(Mutex::new(WebSocket::new())),
            },
        }
    }
    pub fn register_action_handler(
        &mut self,

        tx: UnboundedSender<Action>,
    ) -> Result<(), Box<dyn Error>> {
        self.action_tx = Some(tx);
        Ok(())
    }

    pub fn start(&mut self) {
        let collector_server = self.services.collector_server.clone();

        let cloned_dispatcher = self.action_tx.as_ref().unwrap().clone();

        tokio::spawn(async move {
            collector_server.lock().await.start(cloned_dispatcher).await;
        });
    }

    pub fn stop(&mut self) {
        self.open = false;

        let collector_server = self.services.collector_server.clone();

        tokio::spawn(async move {
            collector_server.lock().await.stop().await.unwrap();
        });
    }

    // fn dispatch(&mut self, action: Action) {
    //     let tx = self.action_tx.clone().unwrap();
    //     tokio::spawn(async move {
    //         tx.send(action).unwrap();
    //     });
    // }

    fn schedule_server_stop(&mut self) {
        let tx = self.action_tx.clone().unwrap();
        tokio::spawn(async move {
            tx.send(Action::StopWebSocketServer).unwrap();
        });
    }

    fn schedule_server_start(&mut self) {
        let tx = self.action_tx.clone().unwrap();
        tokio::spawn(async move {
            tx.send(Action::StartWebSocketServer).unwrap();
        });
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::ScheduleStartWebSocketServer => self.schedule_server_start(),
            Action::ScheduleStopWebSocketServer => self.schedule_server_stop(),
            Action::StartWebSocketServer => self.start(),
            Action::StopWebSocketServer => self.stop(),
            _ => {}
        }
    }

    pub fn init(&mut self) {
        self.insert_mock_data();
    }

    pub fn insert_mock_data(&mut self) {
        let json_strings = vec![
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
        ];

        for json_string in json_strings {
            if let Ok(Payload::Trace(trace)) = parse_raw_trace(json_string) {
                if let Some(action_tx) = self.action_tx.clone() {
                    let _ = action_tx.send(Action::AddTrace(trace));
                }
            }
        }
    }
}
