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

use std::error::Error;

use app::App;

pub enum TraceTimeoutPayload {
    MarkForTimeout(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut app = App::new()?;

    app.run().await?;

    Ok(())
}

