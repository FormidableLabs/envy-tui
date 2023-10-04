mod app;
mod consts;
mod handlers;
mod mock;
mod parser;
mod render;
mod utils;
mod wss;

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::{error::Error, io::Stdout};

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, terminal::enable_raw_mode};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use ratatui::layout::Layout;
use ratatui::prelude::{Constraint, CrosstermBackend, Direction};
use ratatui::terminal::Terminal;

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tungstenite::Message;

use app::{App, WsServerState};
use handlers::{
    handle_back_tab, handle_down, handle_enter, handle_esc, handle_left, handle_pane_next,
    handle_pane_prev, handle_right, handle_tab, handle_up, handle_yank,
};
use render::{
    render_footer, render_help, render_request_block, render_request_body, render_request_summary,
    render_response_block, render_traces,
};
use utils::UIDispatchEvent;

use wss::handle_connection;

use self::handlers::{handle_delete_item, handle_go_to_end, handle_go_to_start, HandlerMetadata};
use self::render::render_response_body;
use self::utils::set_content_length;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<std::sync::Mutex<HashMap<SocketAddr, Tx>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;

    let app = Arc::new(Mutex::new(App::new()));

    let app_for_ui = app.clone();

    let app_for_ws_server = app.clone();

    start_ws_server(app_for_ws_server).await;

    start_ws_client(app);

    terminal.clear()?;

    let _ = run(&mut terminal, &app_for_ui).await;

    restore_terminal(&mut terminal)?;
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

fn start_ws_client(app: Arc<Mutex<App>>) {
    tokio::spawn(async move {
        wss::client(&app).await;

        ()
    });
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn Error>> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    Ok(terminal.show_cursor()?)
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app_raw: &Arc<Mutex<App>>,
) -> Result<(), Box<dyn Error>> {
    let (tx, mut rx) = unbounded::<UIDispatchEvent>();

    let mut network_requests_height = 0;

    let mut response_body_requests_height = 0;
    let mut response_body_requests_width = 0;

    Ok(loop {
        let mut app = app_raw.lock().await;

        let loop_bounded_sender = tx.clone();

        terminal.draw(|frame| {
            if app.active_block == app::ActiveBlock::Help {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(3)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(frame.size());

                render_help(&mut app, frame, main_layout[0]);
            } else {
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

                    render_request_block(&mut app, frame, request_layout[0]);
                    render_request_body(&mut app, frame, request_layout[1]);
                    render_traces(&mut app, frame, split_layout[0]);

                    render_request_summary(&mut app, frame, details_layout[0]);
                    render_response_block(&mut app, frame, response_layout[0]);
                    render_response_body(&mut app, frame, response_layout[1]);

                    render_footer(&mut app, frame, main_layout[1]);

                    response_body_requests_height = response_layout[1].height;
                    response_body_requests_width = response_layout[1].width;
                    network_requests_height = split_layout[0].height;
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

                    render_request_block(&mut app, frame, request_layout[0]);
                    render_request_body(&mut app, frame, request_layout[1]);
                    render_traces(&mut app, frame, main_layout[0]);

                    render_request_summary(&mut app, frame, main_layout[1]);
                    render_response_block(&mut app, frame, response_layout[0]);
                    render_response_body(&mut app, frame, response_layout[1]);

                    render_footer(&mut app, frame, main_layout[4]);

                    response_body_requests_height = response_layout[1].height;
                    response_body_requests_width = response_layout[1].width;
                    network_requests_height = main_layout[0].height;
                }
            }
        })?;

        match rx.try_next() {
            Ok(value) => match value {
                Some(event) => match event {
                    UIDispatchEvent::ClearStatusMessage => app.status_message = None,
                },
                None => {}
            },
            Err(_) => (),
        };

        if app.is_first_render {
            set_content_length(&mut app);

            app.main.scroll_state = app.main.scroll_state.content_length(app.items.len() as u16);

            app.main.index = 0;
            app.main.offset = 0;

            app.is_first_render = false;
        }

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                let metadata = HandlerMetadata {
                    main_height: network_requests_height,
                    response_body_rectangle_height: response_body_requests_height,
                    response_body_rectangle_width: response_body_requests_width,
                };

                match key.code {
                    KeyCode::Char('q') => match app.active_block {
                        app::ActiveBlock::Help => app.active_block = app::ActiveBlock::TracesBlock,
                        _ => {
                            break;
                        }
                    },
                    KeyCode::Tab => handle_tab(&mut app, key),
                    KeyCode::Char('?') => {
                        app.active_block = app::ActiveBlock::Help;
                    }
                    KeyCode::Char('d') => handle_delete_item(&mut app, key),
                    KeyCode::Char('y') => handle_yank(&mut app, key, loop_bounded_sender),
                    KeyCode::Char('>') => handle_go_to_end(&mut app, key, metadata),
                    KeyCode::Char('<') => handle_go_to_start(&mut app, key, metadata),
                    KeyCode::BackTab => handle_back_tab(&mut app, key),
                    KeyCode::Char(']') | KeyCode::PageUp => handle_pane_next(&mut app, key),
                    KeyCode::Char('[') | KeyCode::PageDown => handle_pane_prev(&mut app, key),
                    KeyCode::Enter => handle_enter(&mut app, key),
                    KeyCode::Esc => handle_esc(&mut app, key),
                    KeyCode::Up | KeyCode::Char('k') => {
                        handle_up(&mut app, key, metadata);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        handle_down(&mut app, key, metadata);
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        handle_left(&mut app, key, metadata);
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        handle_right(&mut app, key, metadata);
                    }
                    _ => {}
                }
            }
        }
    })
}
