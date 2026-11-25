use anyhow::Result;
use std::time::Duration;

use super::step;

impl super::RunningWorkflow {
    pub(super) async fn wait(&self, step: &step::WaitStep) -> Result<()> {
        let mut logger = self.logger.lock().await;
        logger.info(&format!("Waiting for {} milliseconds", step.duration));

        tokio::time::sleep(Duration::from_millis(step.duration.into())).await;

        Ok(())
    }

    #[allow(clippy::unused_self)]
    pub(super) async fn abort_workflow(&self, step: &step::AbortStep) -> Result<()> {
        Err(anyhow::anyhow!(
            "Aborting workflow. {}",
            step.message.parse(self).await?
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_data::WorkflowData;
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
    async fn test_wait_zero_duration_completes() {
        let runner = make_runner().await;
        let step = step::WaitStep { duration: 0 };
        let result = runner.wait(&step).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_small_duration_completes() {
        let runner = make_runner().await;
        let step = step::WaitStep { duration: 1 };
        let start = std::time::Instant::now();
        let result = runner.wait(&step).await;
        assert!(result.is_ok());
        // Should complete quickly (less than 1 second)
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_abort_workflow_returns_error() {
        let runner = make_runner().await;
        let step = step::AbortStep {
            message: "test abort".into(),
        };
        let result = runner.abort_workflow(&step).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Aborting workflow"));
        assert!(err.contains("test abort"));
    }

    #[tokio::test]
    async fn test_abort_workflow_with_parsed_message() {
        let mut runner = make_runner().await;
        {
            let mut ctx = runner.context.lock().await;
            ctx.set_item(
                "msg",
                Box::new(crate::context::BasicContextItem::new(serde_json::json!(
                    "custom"
                ))),
            );
        }
        let step = step::AbortStep {
            message: "{{ context.msg }} error".into(),
        };
        let result = runner.abort_workflow(&step).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("custom error"));
    }
}
