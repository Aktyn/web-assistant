use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "If no command is provided, the TUI will be launched", long_about = None)]
pub struct Cli {
    // pub name: Option<String>,
    /// Path to a directory that contains assistant workflows
    #[arg(short, long, value_name = "PATH", default_value = "./workflows")]
    pub workflows_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Command description
    Foo {
        /// command argument description
        #[arg(short, long)]
        example_arg: bool,
    },
}

pub fn get_cli() -> Cli {
    Cli::parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_workflows_path() {
        let cli = Cli::try_parse_from(["test"]).unwrap();
        assert_eq!(
            cli.workflows_path.as_deref(),
            Some(PathBuf::from("./workflows").as_path())
        );
    }

    #[test]
    fn test_cli_custom_workflows_path() {
        let cli = Cli::try_parse_from(["test", "--workflows-path", "/custom/path"]).unwrap();
        assert_eq!(
            cli.workflows_path.as_deref(),
            Some(PathBuf::from("/custom/path").as_path())
        );
    }

    #[test]
    fn test_cli_short_option() {
        let cli = Cli::try_parse_from(["test", "-w", "./my-workflows"]).unwrap();
        assert_eq!(
            cli.workflows_path.as_deref(),
            Some(PathBuf::from("./my-workflows").as_path())
        );
    }

    #[test]
    fn test_cli_foo_command() {
        let cli = Cli::try_parse_from(["test", "foo", "--example-arg"]).unwrap();
        match cli.command {
            Some(Commands::Foo { example_arg }) => assert!(example_arg),
            _ => panic!("Expected Foo command"),
        }
    }

    #[test]
    fn test_cli_no_command() {
        let cli = Cli::try_parse_from(["test"]).unwrap();
        assert!(cli.command.is_none());
    }
}
