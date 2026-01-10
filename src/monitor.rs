use crate::config::Config;
use crate::parser::EventStatus;
use crate::slack::SlackNotifier;
use anyhow::Result;
use tracing::{error, info};

pub struct Monitor {
    client: reqwest::Client,
    config: Config,
    notifier: Option<SlackNotifier>,
}

impl Monitor {
    pub fn new(config: Config, notifier: Option<SlackNotifier>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .build()?;

        Ok(Self {
            client,
            config,
            notifier,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let mut interval = tokio::time::interval(self.config.scrape_period);

        loop {
            interval.tick().await;

            for event_url in &self.config.events {
                self.check_and_notify(event_url).await;
            }
        }
    }

    async fn check_and_notify(&self, event_url: &str) {
        match self.check_event(event_url).await {
            Ok(status) => {
                info!(
                    event_url = %event_url,
                    current = status.current,
                    limit = ?status.limit,
                    has_free_slot = status.has_free_slot(),
                    "Checked event"
                );

                if status.has_free_slot() {
                    if let Some(ref notifier) = self.notifier {
                        let limit = status.limit.unwrap(); // Safe because has_free_slot checks this
                        match notifier
                            .send_notification(event_url, status.current, limit)
                            .await
                        {
                            Ok(_) => {
                                info!(
                                    event_url = %event_url,
                                    "Sent Slack notification"
                                );
                            }
                            Err(e) => {
                                error!(
                                    event_url = %event_url,
                                    error = %e,
                                    "Failed to send Slack notification"
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!(
                    event_url = %event_url,
                    error = %e,
                    "Failed to check event"
                );
            }
        }
    }

    async fn check_event(&self, event_url: &str) -> Result<EventStatus> {
        let response = self.client.get(event_url).send().await?;
        let html = response.text().await?;
        crate::parser::parse_event_html(&html)
    }
}
