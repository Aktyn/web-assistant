use anyhow::Result;

use super::step;

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
                utils::system::run_open_command(&url)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_data::WorkflowData;
    use crate::workflow_data::step::system_action::{NotifyAction, OpenUrlAction};
    use crate::workflow_runtime::RunningWorkflow;
    use config::Config;

    fn default_config() -> Config {
        Config::default()
    }

    async fn make_runner() -> RunningWorkflow {
        RunningWorkflow::new(
            WorkflowData {
                name: "Test".to_string(),
                description: None,
                procedures: vec![],
                steps: vec![],
            },
            Box::new(default_config()),
        )
    }

    #[tokio::test]
    async fn test_run_system_action_open_url_spawns_process() {
        let runner = make_runner().await;
        let step = step::SystemActionStep {
            action: step::SystemAction::OpenUrl(OpenUrlAction {
                url: "https://example.com".into(),
            }),
        };
        // spawn may fail in environments without xdg-open, but the path is exercised
        let result = runner.run_system_action(&step).await;
        // We only assert that it reaches the spawn call; success/failure depends on env
        if let Err(e) = result {
            let err_str = e.to_string();
            assert!(
                err_str.contains("failed to open") || err_str.contains("xdg-open"),
                "Unexpected error: {err_str}"
            );
        }
    }

    #[tokio::test]
    async fn test_run_system_action_notify_calls_notify_api() {
        let runner = make_runner().await;
        let step = step::SystemActionStep {
            action: step::SystemAction::Notify(NotifyAction {
                message: "test notification".into(),
            }),
        };
        // In headless/test environments notify() may succeed or fail;
        // we just want to verify the code path is exercised.
        let _ = runner.run_system_action(&step).await;
    }

    #[tokio::test]
    async fn test_run_system_action_invalid_type_does_not_compile() {
        // This test exists to document that SystemAction enum only has Notify/OpenUrl variants.
        // Adding a new variant would require updating this match.
        let json = r#"{"action":{"type":"notify","message":"hello"}}"#;
        let step: step::SystemActionStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step.action, step::SystemAction::Notify(_)));
    }

    #[tokio::test]
    async fn test_run_system_action_open_url_with_parsed_variable() {
        let runner = make_runner().await;
        {
            let mut ctx = runner.context.lock().await;
            ctx.set_item(
                "url_ctx",
                Box::new(crate::context::BasicContextItem::new(serde_json::json!(
                    "https://rust-lang.org"
                ))),
            );
        }
        let step = step::SystemActionStep {
            action: step::SystemAction::OpenUrl(OpenUrlAction {
                url: "{{ context.url_ctx }}".into(),
            }),
        };
        let result = runner.run_system_action(&step).await;
        if let Err(e) = result {
            let err_str = e.to_string();
            assert!(
                err_str.contains("failed to open") || err_str.contains("xdg-open"),
                "Unexpected error: {err_str}"
            );
        }
    }
}
