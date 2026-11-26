use scopeguard::defer_on_unwind;
use serde::Serialize;
use std::io::{Write, stdout};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use termcolor_json::Theme;

pub fn print_lines(count: u16) {
    std::io::stdout()
        .write_all(&b"\r\n".repeat(count.into()))
        .ok();
}

pub struct CliLogger {
    stdout: StandardStream,
    stderr: StandardStream,
    info_color: ColorSpec,
    success_color: ColorSpec,
    error_color: ColorSpec,
    warning_color: ColorSpec,

    json_theme: Theme,
}

impl Default for CliLogger {
    fn default() -> CliLogger {
        let mut info_color = ColorSpec::new();
        info_color
            .set_bold(false)
            .set_fg(Some(Color::Rgb(178, 235, 242))); // Cyan 100

        let mut success_color = ColorSpec::new();
        success_color
            .set_bold(true)
            .set_fg(Some(Color::Rgb(129, 199, 132)));

        let mut error_color = ColorSpec::new();
        error_color
            .set_bold(true)
            .set_fg(Some(Color::Rgb(229, 115, 115)));

        let mut warning_color = ColorSpec::new();
        warning_color
            .set_bold(false)
            .set_fg(Some(Color::Rgb(255, 183, 77)));

        let mut json_theme = Theme::none();
        json_theme
            .null_mut()
            .set_fg(Some(Color::Rgb(159, 168, 218))) // Indigo 200
            .set_bold(true);
        json_theme
            .bool_mut()
            .set_fg(Some(Color::Rgb(255, 224, 130))) // Amber 200
            .set_bold(true);
        json_theme
            .number_mut()
            .set_fg(Some(Color::Rgb(128, 222, 234))); // Cyan 200
        json_theme
            .string_mut()
            .set_fg(Some(Color::Rgb(165, 214, 167))); // Green 200
        json_theme
            .object_key_mut()
            .set_fg(Some(Color::Rgb(176, 190, 197))); // Blue gray 200

        Self {
            stdout: StandardStream::stdout(ColorChoice::Auto),
            stderr: StandardStream::stderr(ColorChoice::Auto),
            info_color,
            success_color,
            error_color,
            warning_color,
            json_theme,
        }
    }
}

impl CliLogger {
    pub fn success(&mut self, message: &str) {
        print_line(self.stdout.by_ref(), message, &self.success_color);
    }

    pub fn error(&mut self, message: &str) {
        print_line(self.stderr.by_ref(), message, &self.error_color);
    }

    pub fn warning(&mut self, message: &str) {
        print_line(self.stdout.by_ref(), message, &self.warning_color);
    }

    pub fn info(&mut self, message: &str) {
        print_line(self.stdout.by_ref(), message, &self.info_color);
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn json<T>(&mut self, json: &T)
    where
        T: ?Sized + Serialize,
    {
        defer_on_unwind! {
            // Fallback
            let json_string = &serde_json::to_string_pretty(json).unwrap_or_default();
            stdout().write_all(json_string.as_bytes()).ok();
            print_lines(1);
        }

        #[allow(clippy::expect_used)]
        termcolor_json::to_writer_with_theme(self.stdout.by_ref(), json, &self.json_theme)
            .expect("Failed to write JSON to stdout");

        print_lines(1);
    }

    pub fn reset(&mut self) {
        self.stdout.reset().ok();
        self.stderr.reset().ok();
    }
}

fn print_line(stream: &mut StandardStream, message: &str, color: &ColorSpec) {
    let message_bytes = message.as_bytes();

    defer_on_unwind! {
        // Fallback
        stdout().write_all(message_bytes).ok();
        print_lines(1);
    }

    #[allow(clippy::expect_used)]
    stream.set_color(color).expect("Failed to set color");
    #[allow(clippy::expect_used)]
    stream
        .write_all(message_bytes)
        .expect("Failed to write to stdout");
    print_lines(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_logger_success_does_not_panic() {
        let mut logger = CliLogger::default();
        logger.success("test message");
    }

    #[test]
    fn test_cli_logger_error_does_not_panic() {
        let mut logger = CliLogger::default();
        logger.error("test error message");
    }

    #[test]
    fn test_cli_logger_warning_does_not_panic() {
        let mut logger = CliLogger::default();
        logger.warning("test warning message");
    }

    #[test]
    fn test_cli_logger_info_does_not_panic() {
        let mut logger = CliLogger::default();
        logger.info("test info message");
    }

    #[test]
    fn test_cli_logger_reset_does_not_panic() {
        let mut logger = CliLogger::default();
        logger.reset();
    }

    #[test]
    fn test_cli_logger_default_has_valid_colors() {
        let _logger = CliLogger::default();
        // Logger creation succeeds with valid color specs
    }
}
