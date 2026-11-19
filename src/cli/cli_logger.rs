use scopeguard::defer_on_unwind;
use std::io::{Write, stdout};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::utils;

pub struct CliLogger {
    stdout: StandardStream,
    stderr: StandardStream,
    info_color: ColorSpec,
    success_color: ColorSpec,
    error_color: ColorSpec,
    warning_color: ColorSpec,
}

impl Default for CliLogger {
    fn default() -> CliLogger {
        let mut info_color = ColorSpec::new();
        info_color
            .set_bold(false)
            .set_fg(Some(Color::Rgb(236, 239, 241)));

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

        Self {
            stdout: StandardStream::stdout(ColorChoice::Auto),
            stderr: StandardStream::stderr(ColorChoice::Auto),
            info_color,
            success_color,
            error_color,
            warning_color,
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
        utils::print_lines(1);
    }

    stream.set_color(color).expect("Failed to set color");
    stream
        .write_all(message_bytes)
        .expect("Failed to write to stdout");
    utils::print_lines(1);
}
