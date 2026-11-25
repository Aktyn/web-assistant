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

async fn run_tui(config_path: Option<&PathBuf>) -> Result<()> {
    // dotenvy::dotenv().ok();

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

fn print_error(error: &anyhow::Error) {
    let mut logger = utils::logger::CliLogger::default();
    logger.error(&error.to_string());
    logger.reset();
}
