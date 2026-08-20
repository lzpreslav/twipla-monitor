use crate::config::Config;
use crate::parser::EventStatus;
use crate::slack::SlackNotifier;
use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info};

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

pub struct Monitor {
    client: reqwest::Client,
    config: Config,
    notifier: Option<SlackNotifier>,
    last_closed: HashMap<String, bool>,
}

impl Monitor {
    pub fn new(config: Config, notifier: Option<SlackNotifier>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .timeout(HTTP_TIMEOUT)
            .build()?;

        Ok(Self {
            client,
            config,
            notifier,
            last_closed: HashMap::new(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(self.config.scrape_period);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let events = self.config.events.clone();

        loop {
            interval.tick().await;

            for event_url in &events {
                self.check_and_notify(event_url).await;
            }
        }
    }

    async fn check_and_notify(&mut self, event_url: &str) {
        match self.check_event(event_url).await {
            Ok(status) => {
                info!(
                    event_url = %event_url,
                    current = status.current,
                    limit = ?status.limit,
                    closed = status.closed,
                    has_free_slot = status.has_free_slot(),
                    "Checked event"
                );

                let was_closed = self.last_closed.get(event_url).copied();

                if !status.closed && was_closed == Some(true) {
                    // Keep the closed state until delivery succeeds so a
                    // failed send retries next tick.
                    if self.notify_reopened(event_url, &status).await {
                        self.last_closed.insert(event_url.to_string(), false);
                    }
                } else {
                    self.last_closed
                        .insert(event_url.to_string(), status.closed);

                    if status.has_free_slot() {
                        self.notify_free_slot(event_url, &status).await;
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

    async fn notify_free_slot(&self, event_url: &str, status: &EventStatus) {
        let Some(ref notifier) = self.notifier else {
            return;
        };

        let limit = status.limit.unwrap(); // Safe because has_free_slot checks this
        match notifier
            .send_notification(event_url, status.current, limit)
            .await
        {
            Ok(()) => {
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

    /// Returns whether the notification was delivered (trivially true when
    /// no notifier is configured).
    async fn notify_reopened(&self, event_url: &str, status: &EventStatus) -> bool {
        let Some(ref notifier) = self.notifier else {
            return true;
        };

        match notifier
            .send_reopened_notification(event_url, status.current, status.limit)
            .await
        {
            Ok(()) => {
                info!(
                    event_url = %event_url,
                    "Sent Slack reopened notification"
                );
                true
            }
            Err(e) => {
                error!(
                    event_url = %event_url,
                    error = %e,
                    "Failed to send Slack reopened notification"
                );
                false
            }
        }
    }

    async fn check_event(&self, event_url: &str) -> Result<EventStatus> {
        let response = self.client.get(event_url).send().await?;
        let html = response.text().await?;
        crate::parser::parse_event_html(&html)
    }
}
