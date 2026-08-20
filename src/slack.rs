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
        self.post_text(&format!(
            "Free slot available: {event_url} ({current}/{limit})"
        ))
        .await
    }

    pub async fn send_reopened_notification(
        &self,
        event_url: &str,
        current: usize,
        limit: Option<usize>,
    ) -> Result<()> {
        let count = match limit {
            Some(limit) => format!("{current}/{limit}"),
            None => format!("{current}人"),
        };
        self.post_text(&format!(
            "Event is accepting attendees again: {event_url} ({count})"
        ))
        .await
    }

    async fn post_text(&self, message: &str) -> Result<()> {
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
