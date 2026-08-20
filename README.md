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

## Flake

Example:

  ```nix
  {
    description = "My NixOS Flake";

    inputs = {
      nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
      flake-utils.url = "github:numtide/flake-utils";
      twipla-monitor.url = "github:lzpreslav/twipla-monitor";
    };

    outputs = { self, nixpkgs, flake-utils, twipla-monitor }:
      {
        nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            ./hosts/myhost/configuration.nix
            twipla-monitor.nixosModules.default
          ];
        };
      };
  }
  ```

and then in configuration.nix:

  ```nix
  sops.secrets."twipla-monitor/slackWebhook" = {};
  services.twipla-monitor = {
    enable = true;
    events = [ "https://twipla.jp/events/1" ];
    scrapePeriod = "5m";
    slackWebhookFile = config.sops.secrets."twipla-monitor/slackWebhook".path;
  };
  ```

## Behavior

- Checks configured events every scrape period
- Sends notification on every check when free slots are available (current < limit) and registration is open
- Sends notification when an event that had closed registration (参加を締め切りました) starts accepting attendees again
- Does not send free-slot notifications for events with closed registration, even if current < limit
- Ignores events without limits (e.g. "参加者 (90人)") unless they transition from closed to open
- Writes JSON-formatted logs to stdout

## TODO

- Improve/restyle the Slack notifications
- Track individual user participation (if I find a use case)
- Deduplicate notifications (if an event has a free slot and I send a notification, do not resend for X minutes)
