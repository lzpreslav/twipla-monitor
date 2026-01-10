# twipla-monitor

Monitors Twipla.jp events for available slots and sends Slack notifications.

## Configuration

Environment variables:

| Variable                                | Description                                                                | Example                                                 | Default                           |
| --------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------- | --------------------------------- |
| `TWIPLA_MONITOR_EVENTS`                 | Comma-separated list of event URLs to monitor                              | `https://twipla.jp/events/1,https://twipla.jp/events/2` | empty (app runs but does nothing) |
| `TWIPLA_MONITOR_SCRAPE_PERIOD`          | How often to check events (humantime format)                               | `5m`, `1h`, `30s`                                       | `5m`                              |
| `TWIPLA_MONITOR_SLACK_WEBHOOK_URL`      | Slack webhook URL for notifications                                        | `https://hooks.slack.com/services/YOUR/WEBHOOK/URL`     | none (no notifications sent)      |
| `TWIPLA_MONITOR_SLACK_WEBHOOK_URL_FILE` | Path to file containing webhook URL (takes precedence over the direct URL) | `/run/secrets/slack-webhook`                            | none                              |

## Run

If using `direnv`, copy `.envrc.example` to `.envrc`, change the parameters and then run `direnv allow` before running it:

```bash
cargo run
```

Multi-arch images are built and pushed to `ghcr.io` on main branch commits.

## Behavior

- Checks configured events every scrape period
- Sends notification on every check when free slots are available (current < limit)
- Ignores events without limits (e.g. "参加者 (90人)")
- Writes JSON-formatted logs to stdout

## TODO

- Improve/restyle the Slack notifications
- Track individual user participation (if I find a use case)
- Deduplicate notifications (if an event has a free slot and I send a notification, do not resend for X minutes)
