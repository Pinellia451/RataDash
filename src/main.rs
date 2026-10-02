mod app;
mod cli;
mod controller;
mod error;
mod model;
mod runtime;
mod terminal;
mod ui;

use clap::Parser;

use crate::cli::Cli;

#[tokio::main]
async fn main() {
    terminal::install_panic_restore_hook();
    if let Err(error) = run().await {
        eprintln!("错误：{error}");
        std::process::exit(1);
    }
}

async fn run() -> error::Result<()> {
    let options = Cli::parse().resolve()?;
    runtime::run(options).await
}
