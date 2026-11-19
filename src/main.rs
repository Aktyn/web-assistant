//TODO: remove `pub` from module declarations
pub mod agent;
pub mod browser;
mod cli;
mod config;
mod llm;
mod tui;
mod utils;
mod workflow;

use anyhow::Result;
use std::{path::PathBuf, process::ExitCode};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = cli::get_cli();

    match cli.run().await {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => match run_tui(cli.config_path.as_ref()).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                print_error(&e);
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            print_error(&e);
            ExitCode::FAILURE
        }
    }
}

fn print_error(error: &anyhow::Error) {
    let mut logger = cli::CliLogger::default();
    logger.error(&error.to_string());
    logger.reset();
}

async fn run_tui(config_path: Option<&PathBuf>) -> Result<()> {
    dotenvy::dotenv().ok();

    // NOTE: TUI logger is initialized here to catch early core logs
    tui_logger::init_logger(tui_logger::LevelFilter::Trace)?;
    tui_logger::set_default_level(tui_logger::LevelFilter::Trace);

    if let Some(config_path) = config_path {
        let config = config::load_config(config_path).await;
        tui::render_tui(config).await
    } else {
        anyhow::bail!("Config path is required")
    }
}
