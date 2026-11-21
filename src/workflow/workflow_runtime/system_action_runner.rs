use anyhow::Result;

use super::step;

impl super::RunningWorkflow {
    pub(super) async fn run_system_action(&self, step: &step::SystemActionStep) -> Result<()> {
        // let mut logger = self.logger.lock().await;
        log::info!("{step:#?}");

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        match &step.action {
            step::SystemAction::Notify(_notify_action) => {
                // logger.info(&format!("Navigating to: \"{}\"", notify_action.url));

                //TODO: implement
            }
            step::SystemAction::OpenUrl(_open_url_action) => {
                //TODO: implement
            }
        }

        Ok(())
    }
}
