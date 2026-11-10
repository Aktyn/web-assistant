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
        list: bool,
    },
}

pub fn get_cli() -> Cli {
    Cli::parse()
}
