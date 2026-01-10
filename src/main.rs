mod config;
mod parser;
mod slack;

use anyhow::Result;
use config::Config;
use parser::EventStatus;
use slack::SlackNotifier;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;

    info!(
        events_count = config.events.len(),
        scrape_period = ?config.scrape_period,
        slack_enabled = config.slack_webhook_url.is_some(),
        "Starting Twipla Monitor"
    );

    if config.events.is_empty() {
        warn!("No events configured, monitoring will not perform any checks");
    }

    let notifier = config.slack_webhook_url.map(|url| SlackNotifier::new(url));

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .build()?;

    let mut interval = tokio::time::interval(config.scrape_period);

    loop {
        interval.tick().await;

        for event_url in &config.events {
            match check_event(&client, event_url).await {
                Ok(status) => {
                    info!(
                        event_url = %event_url,
                        current = status.current,
                        limit = ?status.limit,
                        has_free_slot = status.has_free_slot(),
                        "Checked event"
                    );

                    if status.has_free_slot() {
                        if let Some(ref notifier) = notifier {
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
    }
}

async fn check_event(client: &reqwest::Client, event_url: &str) -> Result<EventStatus> {
    let response = client.get(event_url).send().await?;
    let html = response.text().await?;
    parser::parse_event_html(&html)
}
