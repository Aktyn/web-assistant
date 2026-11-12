use crate::core::config;

use clap::{Parser, Subcommand, ValueHint::FilePath};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "If no command is provided, the TUI will be launched", long_about = None)]
pub struct Cli {
    // pub name: Option<String>,
    /// Relative or absolute path to config file
    #[arg(short, long, value_name = "FILE", default_value = config::DEFAULT_CONFIG_FILE, value_hint=FilePath)]
    pub config_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initializes default config.json
    InitConfig {
        /// Custom path to where config file will be created
        #[arg(short, long, default_value = config::DEFAULT_CONFIG_FILE, value_hint=FilePath)]
        output_path: Option<PathBuf>,
    },
}

pub fn get_cli() -> Cli {
    Cli::parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_config_path() {
        let cli = Cli::try_parse_from(["test"]).unwrap();
        assert_eq!(
            cli.config_path.as_deref(),
            Some(PathBuf::from(config::DEFAULT_CONFIG_FILE).as_path())
        );
    }

    #[test]
    fn test_cli_custom_config_path() {
        let cli = Cli::try_parse_from(["test", "--config-path", "/custom/config.json"]).unwrap();
        assert_eq!(
            cli.config_path.as_deref(),
            Some(PathBuf::from("/custom/config.json").as_path())
        );
    }

    #[test]
    fn test_cli_short_option() {
        let cli = Cli::try_parse_from(["test", "-c", "./my-config.json"]).unwrap();
        assert_eq!(
            cli.config_path.as_deref(),
            Some(PathBuf::from("./my-config.json").as_path())
        );
    }

    #[test]
    fn test_cli_init_config_command() {
        let cli = Cli::try_parse_from(["test", "init-config", "--output-path", "/tmp/config.json"])
            .unwrap();
        match cli.command {
            Some(Commands::InitConfig { output_path }) => {
                assert_eq!(
                    output_path.as_deref(),
                    Some(PathBuf::from("/tmp/config.json").as_path())
                );
            }
            _ => panic!("Expected InitConfig command"),
        }
    }

    #[test]
    fn test_cli_no_command() {
        let cli = Cli::try_parse_from(["test"]).unwrap();
        assert!(cli.command.is_none());
    }
}
