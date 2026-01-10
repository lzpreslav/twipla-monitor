mod config;
mod monitor;
mod parser;
mod slack;

use anyhow::Result;
use config::Config;
use monitor::Monitor;
use slack::SlackNotifier;
use tracing::{info, warn};

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

    let mut config = config;

    let notifier = config
        .slack_webhook_url
        .take()
        .map(|url| SlackNotifier::new(url));
    let monitor = Monitor::new(config, notifier)?;

    monitor.run().await
}
