use std::io;
use std::time::Duration;
use std::{error::Error, io::Stdout};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, terminal::enable_raw_mode};

use ratatui::layout::Layout;
use ratatui::prelude::{Alignment, Constraint, CrosstermBackend, Direction};
use ratatui::style::{Color, Modifier, Style};
use ratatui::terminal::Terminal;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table};

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

            let requests = &app.requests;

            let active_block = app.active_block.clone();

            let converted_rows: Vec<Vec<String>> = requests
                .iter()
                .map(|request| {
                    let uri = request.uri.clone();
                    let method = request.method.clone().to_string();
                    let time = request.time.clone().to_string();

                    vec![method, uri, time]
                })
                .collect();

            let requests = Table::new(vec![
                // Row can be created from simple strings.
                Row::new(vec![
                    "Post - 200",
                    "https://randomdomain.com/randompath",
                    "1.58s",
                ]),
                // You can style the entire row.
                Row::new(vec!["Row21", "Row22", "Row23"]).style(Style::default().fg(Color::Blue)),
                // If you need more control over the styling you may need to create Cells directly
                Row::new(vec![
                    Cell::from("Row31"),
                    Cell::from("Row32").style(Style::default().fg(Color::Yellow)),
                    Cell::from(Line::from(vec![
                        Span::raw("Row"),
                        Span::styled("33", Style::default().fg(Color::Green)),
                    ])),
                ]),
                Row::new(vec![
                    Cell::from("test"),
                    Cell::from("Row32").style(Style::default().fg(Color::Yellow)),
                    Cell::from(Line::from(vec![
                        Span::raw("Row"),
                        Span::styled("33", Style::default().fg(Color::Green)),
                    ])),
                ])
                .style(Style::default().fg(Color::White).bg(Color::Magenta)),
                // If a Row need to display some content over multiple lines, you just have to change
                // its height.
                Row::new(vec![
                    Cell::from("Row\n41"),
                    Cell::from("Row\n42"),
                    Cell::from("Row\n43"),
                ])
                .height(2),
            ])
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
                    .style(
                        Style::default().fg(if active_block == ActiveBlock::NetworkRequests {
                            Color::White
                        } else {
                            Color::DarkGray
                        }),
                    )
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

            // let requests = Paragraph::new("Requests")
            //     .style(
            //         Style::default().fg(if active_block == ActiveBlock::NetworkRequests {
            //             Color::White
            //         } else {
            //             Color::DarkGray
            //         }),
            //     )
            //     .alignment(Alignment::Center)
            //     .block(
            //         Block::default()
            //             .borders(Borders::ALL)
            //             .style(Style::default().fg(
            //                 if active_block == ActiveBlock::NetworkRequests {
            //                     Color::White
            //                 } else {
            //                     Color::DarkGray
            //                 },
            //             ))
            //             .title("Network requests")
            //             .border_type(BorderType::Plain),
            //     );

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
                        if app.active_block == ActiveBlock::RequestDetails {
                            app.active_block = ActiveBlock::NetworkRequests
                        } else {
                            app.active_block = ActiveBlock::RequestDetails
                        }
                    }
                    KeyCode::Char('j') => {
                        if key.modifiers == KeyModifiers::CONTROL {}
                        if app.active_block == ActiveBlock::RequestDetails {
                            app.active_block = ActiveBlock::NetworkRequests
                        } else {
                            app.active_block = ActiveBlock::RequestDetails
                        }
                    }

                    KeyCode::Tab => {
                        // TODO: Figure out a way to make ModifierKeyCode work here.
                        if app.active_block == ActiveBlock::RequestDetails {
                            app.active_block = ActiveBlock::NetworkRequests
                        } else {
                            app.active_block = ActiveBlock::RequestDetails
                        }
                    }
                    _ => {}
                }
            }
        }
    })
}
