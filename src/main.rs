mod app;
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
use futures_util::{future, StreamExt, TryStreamExt};
use ratatui::layout::Layout;
use ratatui::prelude::{Constraint, CrosstermBackend, Direction};
use ratatui::terminal::Terminal;

use app::App;
use handlers::{
    handle_back_tab, handle_down, handle_enter, handle_esc, handle_left, handle_pane_next,
    handle_pane_prev, handle_right, handle_tab, handle_up,
};
use render::{
    render_footer, render_network_requests, render_request_block, render_request_summary,
    render_response_block,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tungstenite::Message;
type Tx = UnboundedSender<Message>;
type PeerMap = Arc<std::sync::Mutex<HashMap<SocketAddr, Tx>>>;

use self::render::{render_help, render_request_body};
use self::wss::handle_connection;

// async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
//     println!("Incoming TCP connection from: {}", addr);
//
//     let ws_stream = tokio_tungstenite::accept_async(raw_stream)
//         .await
//         .expect("Error during the websocket handshake occurred");
//     println!("WebSocket connection established: {}", addr);
//
//     // Insert the write part of this peer to the peer map.
//     let (tx, rx) = unbounded();
//
//     peer_map.lock().unwrap().insert(addr, tx);
//
//     let (outgoing, incoming) = ws_stream.split();
//
//     let broadcast_incoming = incoming.try_for_each(|msg| {
//         println!(
//             "Received a message from {}: {}",
//             addr,
//             msg.to_text().unwrap()
//         );
//         let peers = peer_map.lock().unwrap();
//
//         // We want to broadcast the message to everyone except ourselves.
//         let broadcast_recipients = peers
//             .iter()
//             .filter(|(peer_addr, _)| peer_addr != &&addr)
//             .map(|(_, ws_sink)| ws_sink);
//
//         for recp in broadcast_recipients {
//             recp.unbounded_send(msg.clone()).unwrap();
//         }
//
//         future::ok(())
//     });
//
//     let receive_from_others = rx.map(Ok).forward(outgoing);
//     //
//     // pin_mut!(broadcast_incoming, receive_from_others);
//     future::select(broadcast_incoming, receive_from_others).await;
//
//     println!("{} disconnected", &addr);
//     peer_map.lock().unwrap().remove(&addr);
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;

    let app = Arc::new(Mutex::new(App::new()));

    let app_clone = app.clone();

    let state = PeerMap::new(std::sync::Mutex::new(HashMap::new()));

    let addr = "127.0.0.1:9999";

    let try_socket = TcpListener::bind(addr).await;
    let listener = try_socket.expect("Failed to bind");

    tokio::spawn(async move {
        wss::client(&app).await;

        ()
    });

    tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(handle_connection(state.clone(), stream, addr));
        }

        ()
    });

    terminal.clear()?;

    let _ = run(&mut terminal, &app_clone).await;

    restore_terminal(&mut terminal)?;
    Ok(())
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
    app: &Arc<Mutex<App>>,
) -> Result<(), Box<dyn Error>> {
    Ok(loop {
        let mut app = app.lock().await;

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
                        .constraints(
                            [Constraint::Percentage(95), Constraint::Percentage(5)].as_ref(),
                        )
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
                                Constraint::Percentage(10),
                                Constraint::Percentage(40),
                                Constraint::Percentage(50),
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

                    render_request_block(&mut app, frame, request_layout[0]);
                    render_request_body(&mut app, frame, request_layout[1]);
                    render_network_requests(&mut app, frame, split_layout[0]);

                    render_request_summary(&mut app, frame, details_layout[0]);
                    render_response_block(&mut app, frame, details_layout[2]);

                    render_footer(&mut app, frame, main_layout[1]);
                } else {
                    let main_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints(
                            [
                                Constraint::Percentage(30),
                                Constraint::Percentage(5),
                                Constraint::Percentage(30),
                                Constraint::Percentage(30),
                                Constraint::Percentage(5),
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

                    render_request_block(&mut app, frame, request_layout[0]);
                    render_request_body(&mut app, frame, request_layout[1]);
                    render_network_requests(&mut app, frame, main_layout[0]);

                    render_request_summary(&mut app, frame, main_layout[1]);
                    render_response_block(&mut app, frame, main_layout[3]);

                    render_footer(&mut app, frame, main_layout[4]);
                }
            }
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => match app.active_block {
                        app::ActiveBlock::Help => {
                            app.active_block = app::ActiveBlock::NetworkRequests
                        }
                        _ => {
                            break;
                        }
                    },
                    KeyCode::Tab => handle_tab(&mut app, key),
                    KeyCode::Char('?') => {
                        app.active_block = app::ActiveBlock::Help;
                    }
                    KeyCode::BackTab => handle_back_tab(&mut app, key),
                    KeyCode::Char(']') | KeyCode::PageUp => handle_pane_next(&mut app, key),
                    KeyCode::Char('[') | KeyCode::PageDown => handle_pane_prev(&mut app, key),
                    KeyCode::Enter => handle_enter(&mut app, key),
                    KeyCode::Esc => handle_esc(&mut app, key),
                    KeyCode::Up | KeyCode::Char('k') => {
                        handle_up(&mut app, key);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        handle_down(&mut app, key);
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        handle_left(&mut app, key);
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        handle_right(&mut app, key);
                    }
                    _ => {}
                }
            }
        }
    })
}
