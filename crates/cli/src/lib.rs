mod cli;
mod cli_runner;
pub(crate) mod common;

pub use cli::get_cli;

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_cli_nonexistent_file() {
        // get_cli parses from argv; with no args it would use defaults
        // We just verify the function exists and returns Cli
        let cli = super::get_cli();
        assert!(cli.config_path.is_some());
    }
}
