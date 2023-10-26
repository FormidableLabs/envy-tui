mod app;
mod config;
mod consts;
mod handlers;
mod mock;
mod parser;
mod render;
mod tui;
mod utils;
mod wss;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::error::Error;

use crossterm::event::KeyCode;

use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use ratatui::layout::Layout;
use ratatui::prelude::{Constraint, CrosstermBackend, Direction};

use tokio::{net::TcpListener, sync::Mutex};
use tungstenite::Message;

use app::{App, AppDispatch, WsServerState};
use handlers::{
    handle_back_tab, handle_down, handle_enter, handle_esc, handle_left, handle_pane_next,
    handle_pane_prev, handle_right, handle_search, handle_tab, handle_up, handle_yank,
};
use render::{
    render_footer, render_help, render_request_block, render_request_body, render_request_summary,
    render_response_block, render_search, render_traces,
};

use wss::handle_connection;

use self::app::Mode;
use self::handlers::{handle_delete_item, handle_go_to_end, handle_go_to_start, HandlerMetadata};
use self::mock::{
    TEST_JSON_1, TEST_JSON_10, TEST_JSON_11, TEST_JSON_12, TEST_JSON_13, TEST_JSON_14,
    TEST_JSON_15, TEST_JSON_16, TEST_JSON_17, TEST_JSON_18, TEST_JSON_2, TEST_JSON_3, TEST_JSON_4,
    TEST_JSON_5, TEST_JSON_6, TEST_JSON_7, TEST_JSON_8, TEST_JSON_9,
};
use self::parser::parse_raw_trace;
use self::render::{render_debug, render_response_body};
use self::utils::set_content_length;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<std::sync::Mutex<HashMap<SocketAddr, Tx>>>;

async fn insert_mock_data(app_raw: &Arc<Mutex<App>>) {
    let mut app = app_raw.lock().await;

    vec![
        TEST_JSON_1,
        TEST_JSON_2,
        TEST_JSON_3,
        TEST_JSON_4,
        TEST_JSON_5,
        TEST_JSON_6,
        TEST_JSON_7,
        TEST_JSON_8,
        TEST_JSON_9,
        TEST_JSON_10,
        TEST_JSON_11,
        TEST_JSON_12,
        TEST_JSON_13,
        TEST_JSON_14,
        TEST_JSON_15,
        TEST_JSON_16,
        TEST_JSON_17,
        TEST_JSON_18,
    ]
    .iter()
    .map(|raw_json_string| parse_raw_trace(raw_json_string))
    .for_each(|x| match x {
        Ok(v) => {
            app.items.insert(v);

            app.logs.push(String::from("Parsing successful."));
        }
        Err(err) => app.logs.push(format!(
            "Something went wrong while parsing and inserting to the Tree, {:?}",
            err
        )),
    });
}

pub enum TraceTimeoutPayload {
    MarkForTimeout(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let app = Arc::new(Mutex::new(App::new()));

    let (tx, mut rx) = unbounded::<AppDispatch>();

    let app_for_ui = app.clone();

    let app_for_ws_server = app.clone();

    insert_mock_data(&app).await;

    let mode = app.lock().await.mode;

    let ws_client_sender = tx.clone();

    if mode == Mode::Normal {
        start_ws_server(app_for_ws_server).await;

        start_ws_client(app, ws_client_sender);
    }

    run(&app_for_ui, &mut rx, tx).await?;

    Ok(())
}

async fn start_ws_server(app: Arc<Mutex<App>>) {
    let state = PeerMap::new(std::sync::Mutex::new(HashMap::new()));

    let addr = "127.0.0.1:9999";

    let try_socket = TcpListener::bind(addr).await;
    let listener = try_socket.expect("Failed to bind");

    app.lock().await.ws_server_state = WsServerState::Open;

    tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(handle_connection(state.clone(), stream, addr, app.clone()));
        }

        ()
    });
}

fn start_ws_client(app: Arc<Mutex<App>>, tx: UnboundedSender<AppDispatch>) {
    tokio::spawn(async move {
        wss::client(&app, tx).await;

        ()
    });
}

fn update(
    app: &mut App,
    event: Option<tui::Event>,
    sender: UnboundedSender<AppDispatch>,
) {
    match event {
        Some(tui::Event::Key(key)) => {
            let metadata = HandlerMetadata {
                main_height: app.main.height,
                response_body_rectangle_height: app.response_body.height,
                response_body_rectangle_width: app.response_body.width,
                request_body_rectangle_height: app.request_body.height,
                request_body_rectangle_width: app.request_body.width,
            };
            if app.active_block == app::ActiveBlock::SearchQuery {
                handle_search(app, key);
            } else {
                match key.code {
                    KeyCode::Char('q') => match app.active_block {
                        app::ActiveBlock::Help | app::ActiveBlock::Debug => {
                            app.active_block =
                                app.previous_block.unwrap_or(app::ActiveBlock::TracesBlock);

                            app.previous_block = None;
                        }
                        _ => app.should_quit = true,
                    },
                    KeyCode::Tab => handle_tab(app, key),
                    KeyCode::Char('?') => {
                        app.previous_block = Some(app.active_block);

                        app.active_block = app::ActiveBlock::Help;
                    }
                    KeyCode::Char('p') => {
                        app.previous_block = Some(app.active_block);

                        app.active_block = app::ActiveBlock::Debug;
                    }
                    KeyCode::Char('d') => handle_delete_item(app, key),
                    KeyCode::Char('y') => handle_yank(app, key, sender),
                    KeyCode::Char('>') => handle_go_to_end(app, key, metadata),
                    KeyCode::Char('<') => handle_go_to_start(app, key, metadata),
                    KeyCode::BackTab => handle_back_tab(app, key),
                    KeyCode::Char(']') | KeyCode::PageUp => handle_pane_next(app, key),
                    KeyCode::Char('[') | KeyCode::PageDown => handle_pane_prev(app, key),
                    KeyCode::Char('/') => handle_search(app, key),
                    KeyCode::Enter => handle_enter(app, key),
                    KeyCode::Esc => handle_esc(app, key),
                    KeyCode::Up | KeyCode::Char('k') => {
                        handle_up(app, key, metadata);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        handle_down(app, key, metadata);
                    }
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                        handle_left(app, key, metadata);
                    }
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                        handle_right(app, key, metadata);
                    }
                    _ => {}
                }
            }
        },
        _ => {}
    }
}

pub type Frame<'a> = ratatui::Frame<'a, CrosstermBackend<std::io::Stdout>>;

fn render(
    frame: &mut Frame<'_>,
    app: &mut App,
) {
    match app.active_block {
        app::ActiveBlock::Help => {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(3)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(frame.size());

            render_help(app, frame, main_layout[0]);
        }
        app::ActiveBlock::Debug => {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(3)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(frame.size());

            render_debug(app, frame, main_layout[0]);
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

                render_request_block(app, frame, request_layout[0]);
                render_request_body(app, frame, request_layout[1]);
                render_traces(app, frame, split_layout[0]);

                render_request_summary(app, frame, details_layout[0]);
                render_response_block(app, frame, response_layout[0]);
                render_response_body(app, frame, response_layout[1]);

                render_footer(app, frame, main_layout[1]);

                render_search(app, frame);

                app.response_body.height = response_layout[1].height;
                app.response_body.width = response_layout[1].width;
                app.main.height = split_layout[0].height;
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

                render_request_block(app, frame, request_layout[0]);
                render_request_body(app, frame, request_layout[1]);
                render_traces(app, frame, main_layout[0]);

                render_request_summary(app, frame, main_layout[1]);
                render_response_block(app, frame, response_layout[0]);
                render_response_body(app, frame, response_layout[1]);

                render_search(app, frame);
                render_footer(app, frame, main_layout[4]);

                app.response_body.height = response_layout[1].height;
                app.response_body.width = response_layout[1].width;

                app.request_body.height = request_layout[1].height;
                app.request_body.width = request_layout[1].width;

                app.main.height = main_layout[0].height;
            }
        }
    };
}

async fn run(
    app_raw: &Arc<Mutex<App>>,
    receiver: &mut UnboundedReceiver<AppDispatch>,
    sender: UnboundedSender<AppDispatch>,
) -> Result<(), Box<dyn Error>> {
    let mut t = tui::Tui::new();
    t.enter()?;

    loop {
        let mut app = app_raw.lock().await;

        let loop_bounded_sender = sender.clone();

        let event = t.next().await;

        if let Some(tui::Event::Render) = event.clone() {
            t.terminal.draw(|frame| {
                render(frame, &mut app);
            })?;
        };

        match receiver.try_next() {
            Ok(value) => match value {
                Some(event) => match event {
                    AppDispatch::MarkTraceAsTimedOut(id) => {
                        app.dispatch(AppDispatch::MarkTraceAsTimedOut(id))
                    }
                    AppDispatch::ClearStatusMessage => app.status_message = None,
                },
                None => {}
            },
            Err(_) => {}
        };

        if app.is_first_render {
            // NOTE: Index and offset needs to be set prior before we call `set_content_length`.
            app.main.index = 0;
            app.main.offset = 0;

            set_content_length(&mut app);

            app.main.scroll_state = app.main.scroll_state.content_length(app.items.len() as u16);

            app.is_first_render = false;
        }

        update(&mut app, event, loop_bounded_sender);

        if app.should_quit {
            break;
        }
    }

    t.exit()?;

    Ok(())
}
