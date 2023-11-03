use std::collections::BTreeSet;

use std::error::Error;
use std::fmt::Display;
use std::sync::Arc;
use std::hash::{Hash, Hasher};

use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::Action;
use crate::mock;
use crate::parser::{Payload, parse_raw_trace};
use crate::wss::WebSocket;
use crate::wss;

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
pub struct Client {
    pub action_tx: Option<UnboundedSender<Action>>,
    pub open: bool,
    pub services: Services,
    pub items: BTreeSet<Trace>,
}

impl Client {
    pub fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<(), Box<dyn Error>> {
        self.action_tx = Some(tx);
        Ok(())
    }

    pub fn start(&mut self) {
        self.open = true;

        let collector_server = self.services.collector_server.clone();
        let tx = self.action_tx.clone();

        tokio::spawn(async move {
            collector_server.lock().await.start().await;
            wss::client(tx).await.unwrap();
        });
    }

    pub fn stop(&mut self) {
        self.open = false;

        let collector_server = self.services.collector_server.clone();

        tokio::spawn(async move {
            collector_server.lock().await.stop().await.unwrap();
        });
    }

    fn add_trace(&mut self, trace: Trace) {
        self.items.replace(trace);
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
    pub fn update(&mut self, action: Action) {
        match action {
            Action::MarkTraceAsTimedOut(id) => self.mark_trace_as_timed_out(id),
            Action::AddTrace(trace) => self.add_trace(trace),
            Action::ScheduleStartWebSocketServer => self.schedule_server_start(),
            Action::ScheduleStopWebSocketServer => self.schedule_server_stop(),
            Action::StartWebSocketServer => self.start(),
            Action::StopWebSocketServer => self.stop(),
            _ => {}
        }
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
