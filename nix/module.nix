{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.twipla-monitor;
in
{
  options.services.twipla-monitor = {
    enable = lib.mkEnableOption "Twipla event monitor";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.callPackage ./package.nix { };
      description = "The twipla-monitor package to use.";
    };

    events = lib.mkOption {
      type = with lib.types; listOf str;
      default = [ ];
      description = ''
        List of Twipla event URLs to monitor.
        Example: `[ "https://twipla.jp/events/1" "https://twipla.jp/events/2" ]`
      '';
    };

    scrapePeriod = lib.mkOption {
      type = with lib.types; str;
      default = "5m";
      description = ''
        How often to check events (in humantime format).
        Examples: `5m`, `1h`, `30s`
      '';
    };

    slackWebhookFile = lib.mkOption {
      type = with lib.types; nullOr path;
      default = null;
      description = ''
        Path to a file containing the Slack webhook URL.
        This file will be passed to the systemd unit via LoadCredential
        and made available at /run/credentials/twipla-monitor/slack-webhook.
      '';
    };

    logLevel = lib.mkOption {
      type =
        with lib.types;
        enum [
          "debug"
          "info"
          "warn"
          "error"
        ];
      default = "info";
      description = ''
        Log level for the monitor.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.twipla-monitor = {
      description = "Twipla Event Monitor";
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];

      environment = {
        TWIPLA_MONITOR_EVENTS = lib.concatStringsSep "," cfg.events;
        TWIPLA_MONITOR_SCRAPE_PERIOD = cfg.scrapePeriod;
        RUST_LOG = cfg.logLevel;
      }
      // (lib.optionalAttrs (cfg.slackWebhookFile != null) {
        TWIPLA_MONITOR_SLACK_WEBHOOK_URL_FILE = "%d/slack-webhook";
      });

      serviceConfig = {
        Type = "simple";
        ExecStart = "${lib.getExe cfg.package}";
        Restart = "on-failure";
        RestartSec = 5;
        RestartSteps = 5;
        RestartMaxDelaySec = "5min";

        DynamicUser = true;

        LoadCredential = lib.mkIf (cfg.slackWebhookFile != null) [
          "slack-webhook:${cfg.slackWebhookFile}"
        ];

        # Security hardening
        AmbientCapabilities = [ ];
        CapabilityBoundingSet = [ ];
        DevicePolicy = "closed";
        KeyringMode = "private";
        LockPersonality = true;
        MemoryDenyWriteExecute = "yes";
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateIPC = true;
        PrivateTmp = true;
        ProcSubset = "pid";
        ProtectClock = true;
        ProtectControlGroups = true;
        ProtectHome = true;
        ProtectHostname = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectProc = "invisible";
        ProtectSystem = "strict";
        RemoveIPC = true;
        RestrictAddressFamilies = [
          "AF_UNIX"
          "AF_INET"
          "AF_INET6"
        ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        SecureBits = [
          "keep-caps"
          "no-setuid-fixup"
          "no-setuid-fixup-locked"
          "no-cap-ambient-raise"
          "no-cap-ambient-raise-locked"
        ];
        SystemCallArchitectures = "native";
        SystemCallErrorNumber = "EPERM";
        SystemCallFilter = [ "@system-service" ];
        UMask = "0077";

        # Logging
        StandardOutput = "journal";
        StandardError = "journal";
        SyslogIdentifier = "twipla-monitor";
      };
    };
  };
}
