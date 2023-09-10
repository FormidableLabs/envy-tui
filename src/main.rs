use std::io;
use std::time::Duration;
use std::{error::Error, io::Stdout};

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, terminal::enable_raw_mode};

use ratatui::layout::Layout;
use ratatui::prelude::{Alignment, Constraint, CrosstermBackend, Direction};
use ratatui::style::{Color, Style};
use ratatui::terminal::Terminal;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

mod app;

use app::{ActiveBlock, App};

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
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
                .split(frame.size());

            let active_block = app.active_block.clone();

            let details = Paragraph::new("Request details")
                .style(
                    Style::default().fg(if active_block == ActiveBlock::RequestDetails {
                        Color::White
                    } else {
                        Color::DarkGray
                    }),
                )
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(
                            if active_block == ActiveBlock::RequestDetails {
                                Color::White
                            } else {
                                Color::DarkGray
                            },
                        ))
                        .title("Request details")
                        .border_type(BorderType::Plain),
                );

            let requests = Paragraph::new("Requests")
                .style(
                    Style::default().fg(if active_block == ActiveBlock::NetworkRequests {
                        Color::White
                    } else {
                        Color::DarkGray
                    }),
                )
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(
                            if active_block == ActiveBlock::NetworkRequests {
                                Color::White
                            } else {
                                Color::DarkGray
                            },
                        ))
                        .title("Network requests")
                        .border_type(BorderType::Plain),
                );

            frame.render_widget(requests, main_layout[0]);

            frame.render_widget(details, main_layout[1]);
        })?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if KeyCode::Char('q') == key.code {
                    break;
                }
            }
        }
    })
}
