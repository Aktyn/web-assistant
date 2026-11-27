mod browser;
mod dom;
mod js;
mod utils;
mod watchdogs;

pub use browser::Browser;

#[cfg(test)]
mod tests {
    use crate::Browser;

    #[test]
    fn test_browser_module_accessible() {
        // Verify Browser type is accessible
        let browser = Browser::new(&config::BrowserConfig::default());
        drop(browser);
    }
}
