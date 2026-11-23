use anyhow::Result;

use super::step;
use crate::utils;

impl super::RunningWorkflow {
    pub(super) async fn run_system_action(&self, step: &step::SystemActionStep) -> Result<()> {
        let mut logger = self.logger.lock().await;

        match &step.action {
            step::SystemAction::Notify(notify_action) => {
                let message = notify_action.message.parse(self).await?;
                logger.info(&format!("Sending notification: \"{message}\""));
                notifica::notify("Web Assistant", &message)?;
            }
            step::SystemAction::OpenUrl(open_url_action) => {
                let url = open_url_action.url.parse(self).await?;
                logger.info(&format!("Opening url: {url}"));
                utils::run_open_command(&url)?;
            }
        }

        Ok(())
    }
}
