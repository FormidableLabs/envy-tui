mod app;
mod handlers;
mod render;
mod utils;

use std::io;
use std::time::Duration;
use std::{error::Error, io::Stdout};

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, terminal::enable_raw_mode};

use ratatui::layout::Layout;
use ratatui::prelude::{Constraint, CrosstermBackend, Direction};
use ratatui::terminal::Terminal;

use app::App;
use handlers::{handle_down, handle_enter, handle_esc, handle_left, handle_right, handle_up};
use render::{
    render_footer, render_network_requests, render_request_headers, render_request_query_params,
    render_request_summary, render_response_headers,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;

    terminal.clear()?;

    run(&mut terminal)?;

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

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Box<dyn Error>> {
    let mut app = App::new();

    Ok(loop {
        terminal.draw(|frame| {
            // TODO: Make the layout responsive.
            let _terminal_width = frame.size().width;

            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(95), Constraint::Percentage(5)].as_ref())
                .split(frame.size());

            let split_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(main_layout[0]);

            let details_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(10),
                        Constraint::Percentage(30),
                        Constraint::Percentage(30),
                        Constraint::Percentage(30),
                    ]
                    .as_ref(),
                )
                .split(split_layout[1]);

            render_network_requests(&mut app, frame, split_layout[0]);

            render_request_summary(&mut app, frame, details_layout[0]);
            render_request_query_params(&mut app, frame, details_layout[1]);
            render_request_headers(&mut app, frame, details_layout[2]);
            render_response_headers(&mut app, frame, details_layout[3]);

            render_footer(&mut app, frame, main_layout[1]);
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        break;
                    }
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
