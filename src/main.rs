mod app;
mod components;
mod config;
mod consts;
#[cfg(feature = "logger")]
mod logger;
mod mock;
mod parser;
mod render;
mod services;
mod tui;
mod utils;
mod wss;

use std::error::Error;

use structured_logger::{async_json::new_writer, Builder};
use tokio::fs::File;

use app::App;

async fn tokio_main() -> Result<(), Box<dyn Error>> {
    #[cfg(feature = "logger")]
    logger::init();

    let mut app = App::new()?;

    app.run().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Err(e) = tokio_main().await {
        eprintln!("{} error: Something went wrong", env!("CARGO_PKG_NAME"));
        Err(e)
    } else {
        Ok(())
    }
}
