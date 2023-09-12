mod app;
mod handlers;

use std::io;
use std::ops::Deref;
use std::time::Duration;
use std::{error::Error, io::Stdout};

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, terminal::enable_raw_mode};

use ratatui::layout::Layout;
use ratatui::prelude::{Alignment, Constraint, CrosstermBackend, Direction};
use ratatui::style::{Color, Modifier, Style};
use ratatui::terminal::Terminal;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table};

use app::{ActiveBlock, App};
use handlers::{handle_down, handle_up};

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

            let requests = &app.requests;

            let active_block = app.active_block.clone();

            let index = app.selection_index.clone();

            let converted_rows: Vec<(Vec<String>, bool)> = requests
                .iter()
                .map(|request| {
                    let uri = request.uri.clone();
                    let method = request.method.clone().to_string();
                    let time = request.time.clone().to_string();
                    let id = request.id.clone().to_string();
                    let selected_item = requests.get(index).clone();

                    let selected = match selected_item {
                        Some(item) => {
                            if item.deref() == request {
                                true
                            } else {
                                false
                            }
                        }
                        None => false,
                    };

                    (vec![method, uri, time, id], selected)
                })
                .collect();

            let default_style = Style::default().fg(Color::White);

            let selected_style = Style::default().fg(Color::Black).bg(Color::LightRed);

            // NOTE: Why iter or map gives back ref?
            let _mapped_over: Vec<Paragraph> = [Paragraph::new("title")]
                .iter()
                .map(|x| {
                    let cloned = x.deref().clone();

                    cloned
                })
                .collect();

            let styled_rows: Vec<Row> = converted_rows
                .iter()
                .map(|(row, selected)| {
                    let str_vec: Vec<&str> = row
                        .iter()
                        .map(|x| x.as_str())
                        .collect::<Vec<&str>>()
                        .clone();

                    Row::new(str_vec).style(if *selected {
                        selected_style
                    } else {
                        default_style
                    })
                })
                .collect();

            let requests = Table::new(styled_rows)
                // You can set the style of the entire Table.
                .style(Style::default().fg(Color::White))
                // It has an optional header, which is simply a Row always visible at the top.
                .header(
                    Row::new(vec!["Method", "Request", "Time"])
                        .style(Style::default().fg(Color::Yellow))
                        // If you want some space between the header and the rest of the rows, you can always
                        // specify some margin at the bottom.
                        .bottom_margin(1),
                )
                // As any other widget, a Table can be wrapped in a Block.
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
                )
                // Columns widths are constrained in the same way as Layout...
                .widths(&[
                    Constraint::Percentage(10),
                    Constraint::Percentage(70),
                    Constraint::Length(20),
                ])
                // ...and they can be separated by a fixed spacing.
                // .column_spacing(1)
                // If you wish to highlight a row in any specific way when it is selected...
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                // ...and potentially show a symbol in front of the selection.
                .highlight_symbol(">>");

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

            frame.render_widget(requests, main_layout[0]);

            frame.render_widget(details, main_layout[1]);
        })?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        break;
                    }
                    KeyCode::Char('k') => {
                        handle_up(&mut app, key);
                    }
                    KeyCode::Char('j') => {
                        handle_down(&mut app, key);
                    }
                    _ => {}
                }
            }
        }
    })
}
