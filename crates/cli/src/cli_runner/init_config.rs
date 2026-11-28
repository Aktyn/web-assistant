use anyhow::Result;
use config::create_default_config;
use std::path::PathBuf;

pub(crate) async fn init_config(output_path: Option<&PathBuf>) -> Result<()> {
    let mut logger = utils::logger::CliLogger::default();
    logger.info("Initializing config...");

    let config_path = create_default_config(output_path.map(PathBuf::as_path)).await?;

    let absolute_path = config_path
        .canonicalize()
        .unwrap_or_else(|_| config_path.to_path_buf());

    logger.success(&format!(
        "The default config was successfully saved to: {}",
        absolute_path.display()
    ));

    logger.reset();

    Ok(())
}
