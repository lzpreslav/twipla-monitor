use anyhow::Result;
use serde_json::json;

pub struct SlackNotifier {
    webhook_url: String,
    client: reqwest::Client,
}

impl SlackNotifier {
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn send_notification(
        &self,
        event_url: &str,
        current: usize,
        limit: usize,
    ) -> Result<()> {
        let message = format!("Free slot available: {event_url} ({current}/{limit})");

        let payload = json!({
            "text": message
        });

        self.client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_slack_notifier_creation() {
        let notifier = SlackNotifier::new("https://hooks.slack.com/test".to_string());
        assert_eq!(notifier.webhook_url, "https://hooks.slack.com/test");
    }
}
