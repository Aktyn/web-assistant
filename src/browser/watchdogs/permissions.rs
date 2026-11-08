use anyhow::Result;
use tracing::debug;

use crate::browser::cdp::CdpClient;

#[derive(Clone)]
pub struct PermissionsWatchdog {
    client: CdpClient,
    permissions: Vec<String>,
}

impl PermissionsWatchdog {
    pub fn new(client: CdpClient) -> Self {
        Self {
            client,
            permissions: vec!["clipboardReadWrite".into(), "notifications".into()],
        }
    }

    /// Construct a test-only instance with a dummy client.
    #[cfg(test)]
    pub(crate) fn new_test() -> Self {
        Self {
            client: CdpClient::new_test_client(),
            permissions: vec!["clipboardReadWrite".into(), "notifications".into()],
        }
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn permissions(&self) -> &[String] {
        &self.permissions
    }

    pub async fn apply(&self) -> Result<()> {
        if self.permissions.is_empty() {
            return Ok(());
        }

        let res = self
            .client
            .send_command(
                "Browser.grantPermissions",
                serde_json::json!({
                    "permissions": self.permissions
                }),
            )
            .await;

        match res {
            Ok(_) => debug!(
                "PermissionsWatchdog: Granted permissions: {:?}",
                self.permissions
            ),
            Err(e) => debug!("PermissionsWatchdog: Failed to grant permissions (non-fatal): {e}"),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_permissions() {
        let wd = PermissionsWatchdog::new_test();
        let perms = wd.permissions();
        assert_eq!(perms.len(), 2);
        assert_eq!(perms[0], "clipboardReadWrite");
        assert_eq!(perms[1], "notifications");
    }

    #[test]
    fn test_custom_permissions() {
        let custom = vec!["camera".into(), "microphone".into()];
        let wd = PermissionsWatchdog::new_test().with_permissions(custom.clone());
        assert_eq!(wd.permissions(), &custom);
    }

    #[test]
    fn test_empty_permissions_skips_cdp_call() {
        // When permissions list is empty, apply() should return Ok without sending any command
        let wd = PermissionsWatchdog {
            client: CdpClient::new_test_client(),
            permissions: Vec::new(),
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(wd.apply());
        assert!(result.is_ok());
    }

    #[test]
    fn test_permissions_builder_fluent_api() {
        let wd = PermissionsWatchdog::new_test().with_permissions(vec!["geolocation".into()]);
        assert_eq!(wd.permissions(), &vec!["geolocation"]);
    }
}
