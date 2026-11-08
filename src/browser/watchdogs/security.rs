use anyhow::{Result, anyhow};
use url::Url;

#[derive(Debug, Clone, Default)]
pub struct SecurityWatchdog {
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
}

impl SecurityWatchdog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_domain(mut self, domain: impl Into<String>) -> Self {
        self.allowed_domains.push(domain.into().to_lowercase());
        self
    }

    pub fn block_domain(mut self, domain: impl Into<String>) -> Self {
        self.blocked_domains.push(domain.into().to_lowercase());
        self
    }

    pub fn is_url_allowed(&self, raw_url: &str) -> bool {
        if raw_url == "about:blank" || raw_url.starts_with("chrome:") {
            return true;
        }

        let parsed = match Url::parse(raw_url) {
            Ok(u) => u,
            Err(_) => return false,
        };

        let host = match parsed.host_str() {
            Some(h) => h.to_lowercase(),
            None => return false,
        };

        // Allowed domains take precedence over blocked (matches Python semantics).
        if !self.allowed_domains.is_empty() {
            for allowed in &self.allowed_domains {
                if Self::_domain_matches(&host, allowed) {
                    return true;
                }
            }
            // Explicit allowlist set but host not in it — deny
            return false;
        }

        // No allowlist — check blocked domains
        for blocked in &self.blocked_domains {
            if Self::_domain_matches(&host, blocked) {
                return false;
            }
        }

        true
    }

    /// Check whether `host` matches a domain pattern.
    /// Matches exact hostname or `www.` prefix variant when the pattern is a
    /// two-label domain (e.g. `"example.com"`). Arbitrary subdomains are NOT
    /// matched unless the pattern itself contains a wildcard.
    fn _domain_matches(host: &str, pattern: &str) -> bool {
        if host == pattern {
            return true;
        }
        // Root domains (exactly one dot, e.g. "example.com") also match www.
        if pattern.matches('.').count() == 1 && host == format!("www.{pattern}") {
            return true;
        }
        false
    }

    pub fn validate_url(&self, raw_url: &str) -> Result<()> {
        if !self.is_url_allowed(raw_url) {
            return Err(anyhow!(
                "Navigation to {} blocked by SecurityWatchdog policy",
                raw_url
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_allow_and_block() {
        let sec = SecurityWatchdog::new()
            .allow_domain("github.com")
            .block_domain("malicious.github.com");

        // github.com exact match is allowed
        assert!(sec.is_url_allowed("https://github.com/browser-use/browser-use"));
        // api.github.com is NOT matched by plain "github.com" pattern (Python semantics)
        assert!(!sec.is_url_allowed("https://api.github.com/repos"));
        // malicious.github.com is explicitly blocked
        assert!(!sec.is_url_allowed("https://malicious.github.com/evil"));
        // Non-whitelisted domains are denied
        assert!(!sec.is_url_allowed("https://google.com"));
        assert!(sec.is_url_allowed("about:blank"));
    }

    #[test]
    fn test_default_allows_everything() {
        let sec = SecurityWatchdog::new();
        // No restrictions — everything should pass
        assert!(sec.is_url_allowed("https://google.com"));
        assert!(sec.is_url_allowed("https://example.com/path?q=1"));
        assert!(sec.is_url_allowed("http://insecure.example.com"));
    }

    #[test]
    fn test_blocked_domain_exact_match_only() {
        let sec = SecurityWatchdog::new().block_domain("badsite.com");
        assert!(!sec.is_url_allowed("https://badsite.com"));
        // Root domains also match their www. variant (Python semantics)
        assert!(!sec.is_url_allowed("https://www.badsite.com/login"));
        // Arbitrary subdomains are NOT blocked by a plain domain pattern
        assert!(sec.is_url_allowed("https://notbadsite.com"));
    }

    #[test]
    fn test_allowed_domain_takes_precedence_over_blocked() {
        let sec = SecurityWatchdog::new()
            .allow_domain("api.github.com")
            .block_domain("github.com");
        // allowed takes precedence: api.github.com is explicitly allowed
        assert!(sec.is_url_allowed("https://api.github.com/v1/repos"));
        // github.com is blocked
        assert!(!sec.is_url_allowed("https://github.com/user/repo"));
    }

    #[test]
    fn test_chrome_internal_urls_allowed() {
        let sec = SecurityWatchdog::new();
        assert!(sec.is_url_allowed("chrome://settings"));
        assert!(sec.is_url_allowed("chrome-extension://abc/page.html"));
    }

    #[test]
    fn test_invalid_url_returns_false() {
        let sec = SecurityWatchdog::new();
        assert!(!sec.is_url_allowed("not-a-url"));
        assert!(!sec.is_url_allowed(""));
    }

    #[test]
    fn test_validate_url_returns_result() {
        let sec = SecurityWatchdog::new().block_domain("evil.com");
        assert!(sec.validate_url("https://good.com").is_ok());
        let err = sec.validate_url("https://evil.com").unwrap_err();
        assert!(err.to_string().contains("blocked by SecurityWatchdog"));
    }

    #[test]
    fn test_case_insensitive_domains() {
        let sec = SecurityWatchdog::new().block_domain("GitHub.COM");
        assert!(!sec.is_url_allowed("https://GITHUB.com/foo"));
        assert!(!sec.is_url_allowed("https://github.com/bar"));
    }
}
