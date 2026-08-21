use crate::config::Config;
use crate::parser::EventStatus;
use crate::slack::SlackNotifier;
use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info};

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy)]
struct EventState {
    closed: bool,
    had_free_slot: bool,
}

impl EventState {
    fn of(status: &EventStatus) -> Self {
        Self {
            closed: status.closed,
            had_free_slot: status.has_free_slot(),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Action {
    NotifyReopened,
    NotifyFreeSlot,
    Nothing,
}

fn decide_action(prev: Option<EventState>, status: &EventStatus) -> Action {
    if status.is_joinable() && prev.is_some_and(|p| p.closed) {
        Action::NotifyReopened
    } else if status.has_free_slot() && !prev.is_some_and(|p| p.had_free_slot) {
        Action::NotifyFreeSlot
    } else {
        Action::Nothing
    }
}

pub struct Monitor {
    client: reqwest::Client,
    config: Config,
    notifier: Option<SlackNotifier>,
    last_state: HashMap<String, EventState>,
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
            last_state: HashMap::new(),
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
                debug!(
                    event_url = %event_url,
                    current = status.current,
                    limit = ?status.limit,
                    closed = status.closed,
                    has_free_slot = status.has_free_slot(),
                    "Checked event"
                );

                let prev = self.last_state.get(event_url).copied();

                let delivered = match decide_action(prev, &status) {
                    Action::NotifyReopened => self.notify_reopened(event_url, &status).await,
                    Action::NotifyFreeSlot => self.notify_free_slot(event_url, &status).await,
                    Action::Nothing => true,
                };

                // Keep the previous state until delivery succeeds so a
                // failed send retries next tick.
                if delivered {
                    self.last_state
                        .insert(event_url.to_string(), EventState::of(&status));
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

    /// Returns whether the notification was delivered (trivially true when
    /// no notifier is configured).
    async fn notify_free_slot(&self, event_url: &str, status: &EventStatus) -> bool {
        let Some(ref notifier) = self.notifier else {
            return true;
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
                true
            }
            Err(e) => {
                error!(
                    event_url = %event_url,
                    error = %e,
                    "Failed to send Slack notification"
                );
                false
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
        let response = self
            .client
            .get(event_url)
            .send()
            .await?
            .error_for_status()?;
        let html = response.text().await?;
        crate::parser::parse_event_html(&html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(current: usize, limit: Option<usize>, closed: bool) -> EventStatus {
        EventStatus {
            current,
            limit,
            closed,
        }
    }

    fn state(closed: bool, had_free_slot: bool) -> Option<EventState> {
        Some(EventState {
            closed,
            had_free_slot,
        })
    }

    #[test]
    fn test_first_observation() {
        assert_eq!(
            decide_action(None, &status(5, Some(10), false)),
            Action::NotifyFreeSlot
        );
        assert_eq!(decide_action(None, &status(1, None, true)), Action::Nothing);
        assert_eq!(
            decide_action(None, &status(5, None, false)),
            Action::Nothing
        );
    }

    #[test]
    fn test_reopened_transitions() {
        assert_eq!(
            decide_action(state(true, false), &status(1, None, false)),
            Action::NotifyReopened
        );
        assert_eq!(
            decide_action(state(true, false), &status(5, Some(10), false)),
            Action::NotifyReopened
        );
        // reopened while already full
        assert_eq!(
            decide_action(state(true, false), &status(10, Some(10), false)),
            Action::Nothing
        );
        // still closed
        assert_eq!(
            decide_action(state(true, false), &status(1, None, true)),
            Action::Nothing
        );
    }

    #[test]
    fn test_free_slot_edges() {
        assert_eq!(
            decide_action(state(false, false), &status(5, Some(10), false)),
            Action::NotifyFreeSlot
        );
        // already notified for this opening
        assert_eq!(
            decide_action(state(false, true), &status(4, Some(10), false)),
            Action::Nothing
        );
        // filled up again; re-arms for the next opening
        assert_eq!(
            decide_action(state(false, true), &status(10, Some(10), false)),
            Action::Nothing
        );
        assert_eq!(
            decide_action(state(false, false), &status(5, None, false)),
            Action::Nothing
        );
        // just closed
        assert_eq!(
            decide_action(state(false, false), &status(5, Some(10), true)),
            Action::Nothing
        );
    }
}
