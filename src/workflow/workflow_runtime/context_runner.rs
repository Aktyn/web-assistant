use anyhow::{Context, Result};

use super::step;
use crate::{utils, workflow::context::PropertyPath};

impl super::RunningWorkflow {
    pub(super) async fn update_context(&self, step: &step::UpdateContextStep) -> Result<()> {
        // log::info!("Context value: {:#?}", step.value);
        // log::info!(
        //     "Whole context: {:#?}",
        //     self.context.lock().await.get_item("newest-vr-video")
        // );
        let value = if let Some(value) = &step.value {
            Some(value.parse(self).await?).filter(|s| !s.is_empty())
        } else {
            None
        };

        let mut context = self.context.lock().await;
        // let mut logger = self.logger.lock().await;

        let mut segments = step.key.split('.');
        let context_name = segments
            .next()
            .with_context(|| format!("Context key is invalid: {}", step.key))?;

        let json_value = if let Some(value) = value {
            utils::json_parse(&value)
        } else {
            serde_json::Value::Null
        };

        context.update_item_value(
            context_name,
            &segments.collect::<PropertyPath>(),
            &json_value,
        )?;

        Ok(())
    }
}
