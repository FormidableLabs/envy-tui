use crossterm::{event::KeyEvent, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use futures_util::{FutureExt, StreamExt};
use ratatui::{prelude::CrosstermBackend, terminal::Terminal};
use std::io::{stdout, Stdout};
use std::error::Error;
use tokio::{sync::mpsc, task::JoinHandle};

pub type Frame<'a> = ratatui::Frame<'a, CrosstermBackend<std::io::Stdout>>;

#[derive(Clone, Copy, Debug)]
pub enum Event {
    Error,
    Key(KeyEvent),
    Render,
    Tick,
}

#[derive(Debug)]
pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
    pub event_tx: mpsc::UnboundedSender<Event>,
    pub event_rx: tokio::sync::mpsc::UnboundedReceiver<Event>,
    pub task: JoinHandle<()>,
    pub frame_rate: f64,
    pub tick_rate: f64,
}

impl Tui {
    pub fn new() -> Self {
        let tick_rate = 4.0;
        let frame_rate = 60.0;
        let (event_tx, event_rx) =  mpsc::unbounded_channel();
        let task = tokio::spawn(async {});
        let terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();

        Self { event_tx, event_rx, frame_rate, task, terminal, tick_rate }
    }

    pub fn enter(&mut self) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        self.terminal.clear()?;
        self.start();
        Ok(())
    }

    pub fn exit(&mut self) -> Result<(), Box<dyn Error>> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen,)?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    pub fn start(&mut self) {
        let tick_delay = std::time::Duration::from_secs_f64(1.0 / self.tick_rate);
        let render_delay = std::time::Duration::from_secs_f64(1.0 / self.frame_rate);
        let _tx = self.event_tx.clone();

        self.task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_delay);
            let mut render_interval = tokio::time::interval(render_delay);

            loop {
                let tick_delay = tick_interval.tick();
                let render_delay = render_interval.tick();
                let crossterm_event = reader.next().fuse();
                tokio::select! {
                    maybe_event = crossterm_event => {
                        match maybe_event {
                            Some(Ok(evt)) => {
                                match evt {
                                    crossterm::event::Event::Key(key) => {
                                        if key.kind == crossterm::event::KeyEventKind::Press {
                                            _tx.send(Event::Key(key)).unwrap();
                                        }
                                    },
                                    _ => {},
                                }
                            }
                            Some(Err(_)) => {
                                _tx.send(Event::Error).unwrap();
                            }
                            None => {},
                        }
                    },
                    // TODO: What is this used for?
                    _ = tick_delay => {
                        _tx.send(Event::Tick).unwrap();
                    },
                    _ = render_delay => {
                        _tx.send(Event::Render).unwrap();
                    }
                }
            }
        });
    }

    pub async fn next(&mut self) -> Option<Event> {
         self.event_rx.recv().await
    }
}

