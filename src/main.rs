mod app;
mod config;
mod components;
mod consts;
mod mock;
mod parser;
mod render;
mod services;
mod tui;
mod utils;
mod wss;

use std::error::Error;

use app::App;

async fn tokio_main() -> Result<(), Box<dyn Error>> {
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
