mod app;
mod handlers;
mod render;

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
use handlers::{handle_down, handle_up};

use self::render::{render_network_requests, render_request_details};

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
            let terminal_width = frame.size().width;

            let wide_layout = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
                .split(frame.size());

            let narrow_layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
                .split(frame.size());

            let layout = if terminal_width > 200 {
                wide_layout
            } else {
                narrow_layout
            };

            render_request_details(&mut app, frame, layout[1]);
            render_network_requests(&mut app, frame, layout[0]);
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        break;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        handle_up(&mut app, key);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        handle_down(&mut app, key);
                    }
                    _ => {}
                }
            }
        }
    })
}
