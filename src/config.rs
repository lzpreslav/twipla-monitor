use anyhow::{Context, Result};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub events: Vec<String>,
    pub scrape_period: Duration,
    pub slack_webhook_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let events = std::env::var("TWIPLA_MONITOR_EVENTS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let scrape_period =
            std::env::var("TWIPLA_MONITOR_SCRAPE_PERIOD").unwrap_or_else(|_| "5m".to_string());
        let scrape_period = humantime::parse_duration(&scrape_period)
            .context("Failed to parse TWIPLA_MONITOR_SCRAPE_PERIOD")?;

        // Check for webhook URL from file first, then direct URL
        let slack_webhook_url =
            if let Ok(file_path) = std::env::var("TWIPLA_MONITOR_SLACK_WEBHOOK_URL_FILE") {
                Some(
                    std::fs::read_to_string(&file_path)
                        .with_context(|| {
                            format!("Failed to read webhook URL from file: {}", file_path)
                        })?
                        .trim()
                        .to_string(),
                )
            } else {
                std::env::var("TWIPLA_MONITOR_SLACK_WEBHOOK_URL").ok()
            };

        Ok(Config {
            events,
            scrape_period,
            slack_webhook_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Mutex to prevent tests from running in parallel and interfering with each other
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F>(vars: Vec<(&str, Option<&str>)>, f: F)
    where
        F: FnOnce(),
    {
        let _lock = TEST_LOCK.lock().unwrap();

        // Store original values
        let original: Vec<_> = vars
            .iter()
            .map(|(key, _)| (*key, env::var(key).ok()))
            .collect();

        // Set test values
        for (key, value) in &vars {
            match value {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }

        // Run test
        f();

        // Restore original values
        for (key, value) in original {
            match value {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }
    }

    #[test]
    fn test_config_defaults() {
        with_env(
            vec![
                ("TWIPLA_MONITOR_EVENTS", None),
                ("TWIPLA_MONITOR_SCRAPE_PERIOD", None),
                ("TWIPLA_MONITOR_SLACK_WEBHOOK_URL", None),
                ("TWIPLA_MONITOR_SLACK_WEBHOOK_URL_FILE", None),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.events.len(), 0);
                assert_eq!(config.scrape_period, Duration::from_secs(300)); // 5m
                assert!(config.slack_webhook_url.is_none());
            },
        );
    }

    #[test]
    fn test_config_with_events() {
        with_env(
            vec![
                (
                    "TWIPLA_MONITOR_EVENTS",
                    Some("https://twipla.jp/events/1,https://twipla.jp/events/2"),
                ),
                ("TWIPLA_MONITOR_SCRAPE_PERIOD", Some("10m")),
                (
                    "TWIPLA_MONITOR_SLACK_WEBHOOK_URL",
                    Some("https://hooks.slack.com/test"),
                ),
                ("TWIPLA_MONITOR_SLACK_WEBHOOK_URL_FILE", None),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.events.len(), 2);
                assert_eq!(config.events[0], "https://twipla.jp/events/1");
                assert_eq!(config.events[1], "https://twipla.jp/events/2");
                assert_eq!(config.scrape_period, Duration::from_secs(600)); // 10m
                assert_eq!(
                    config.slack_webhook_url.as_deref(),
                    Some("https://hooks.slack.com/test")
                );
            },
        );
    }

    #[test]
    fn test_config_file_takes_precedence() {
        // Create a temporary file with webhook URL
        let temp_file = "/tmp/twipla_test_webhook";
        std::fs::write(temp_file, "https://hooks.slack.com/from-file\n").unwrap();

        with_env(
            vec![
                ("TWIPLA_MONITOR_EVENTS", None),
                ("TWIPLA_MONITOR_SCRAPE_PERIOD", None),
                (
                    "TWIPLA_MONITOR_SLACK_WEBHOOK_URL",
                    Some("https://hooks.slack.com/direct"),
                ),
                ("TWIPLA_MONITOR_SLACK_WEBHOOK_URL_FILE", Some(temp_file)),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(
                    config.slack_webhook_url.as_deref(),
                    Some("https://hooks.slack.com/from-file")
                );
            },
        );

        std::fs::remove_file(temp_file).ok();
    }
}
