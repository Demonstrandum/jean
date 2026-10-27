# services.display: the display server (feeds, commands, the journal) and, optionally, a kiosk browser that shows it.
#
#   services.display = { enable = true; kiosk = true; };
#
# Everything the agent edits (feeds/, commands/, data/, config.json (your location), secrets/, the journal in state/) lives under `root`, writable,
# outside the Nix store, and is seeded from this repository the first time. The agent's changes are git commits in `state/`.
{ self }:
{ config, lib, pkgs, ... }:
let
  cfg = config.services.display;
  pkg = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
  url = "http://127.0.0.1:${toString cfg.port}/live";
in
{
  options.services.display = {
    enable = lib.mkEnableOption "the e-ink display server";
    port = lib.mkOption { type = lib.types.port; default = 8788; description = "Loopback port of the display server."; };
    root = lib.mkOption { type = lib.types.path; default = "/var/lib/display"; description = "Writable home of feeds/, commands/, data/, config.json and state/."; };
    kiosk = lib.mkEnableOption "a fullscreen Chromium (under cage) on this machine's panel, showing the display page";
    extraPath = lib.mkOption { type = lib.types.listOf lib.types.package; default = [ ]; description = "Extra programs the feeds and commands may call."; };

    agent = {
      enable = lib.mkEnableOption "the assistant: microphone, whisper.cpp, the agents, a speaker";
      apiKeyFile = lib.mkOption { type = lib.types.path; description = "A file with `ANTHROPIC_API_KEY=...` (kept outside the Nix store; systemd reads it as root)."; };
      model = lib.mkOption { type = lib.types.str; default = "claude-haiku-5-5"; description = "The voice agent's model. The builder uses a larger one."; };
      wake = lib.mkOption { type = lib.types.listOf lib.types.str; default = [ "hey claude" "hey jean" ]; description = "Phrases that address it."; };
      timezone = lib.mkOption { type = lib.types.str; default = "Europe/London"; };
      place = lib.mkOption { type = lib.types.str; default = "London"; };
      whisperModel = lib.mkOption { type = lib.types.path; description = "A ggml whisper model, e.g. ggml-base.en.bin (`nix-prefetch-url` it into the store)."; };
      piperVoice = lib.mkOption { type = lib.types.path; description = "A Piper voice (.onnx; its .onnx.json sits beside it)."; };
      approval = lib.mkOption { type = lib.types.enum [ "auto" "person" ]; default = "person"; description = "Whether the builder's finished changes go live by themselves."; };
    };

    panel = {
      enable = lib.mkEnableOption "serving the display to a panel over VNC (a reMarkable 2, or any viewer): headless Chromium, dithered";
      geometry = lib.mkOption { type = lib.types.enum [ "native" "rm2" "rm2-portrait" ]; default = "rm2"; };
      port = lib.mkOption { type = lib.types.port; default = 5900; };
      openFirewall = lib.mkOption { type = lib.types.bool; default = false; description = "VNC here has no password: only on a network you trust."; };
    };

    watchdog.enable = lib.mkOption { type = lib.types.bool; default = true; description = "Restart the display if it stops answering, and say so in the audit log."; };
  };

  config = lib.mkIf cfg.enable {
    users.users.display = { isSystemUser = true; group = "display"; home = cfg.root; createHome = true; extraGroups = [ "video" "input" "render" ]; shell = pkgs.bashInteractive; };
    users.groups.display = { };

    systemd.services.display-server = {
      description = "E-ink display server (UI, journal, feeds)";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      # what the shipped feeds call; the agent adds to this list through `extraPath` when a new feed needs a tool
      path = [ pkgs.python3 pkgs.git pkgs.coreutils pkgs.gawk pkgs.gnused pkgs.gnugrep pkgs.procps pkgs.iproute2 pkgs.iw pkgs.networkmanager ] ++ cfg.extraPath;
      preStart = ''
        mkdir -p ${cfg.root}/state
        for d in feeds commands data; do
          [ -e ${cfg.root}/$d ] || cp -r ${self}/$d ${cfg.root}/$d
        done
        [ -e ${cfg.root}/config.json ] || cp ${self}/config.example.json ${cfg.root}/config.json   # edit: your location
        chmod -R u+w ${cfg.root}
      '';
      serviceConfig = {
        User = "display";
        Group = "display";
        WorkingDirectory = cfg.root;
        ExecStart = "${pkg}/bin/display-server 127.0.0.1:${toString cfg.port} ${cfg.root}/state ${cfg.root}";
        Restart = "always";
        RestartSec = 2;
      };
    };

    # --- the assistant -------------------------------------------------------------------------------------------------------------
    systemd.services.whisper = lib.mkIf cfg.agent.enable {
      description = "whisper.cpp speech recognition";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        DynamicUser = true;
        ExecStart = "${pkgs.whisper-cpp}/bin/whisper-server -m ${cfg.agent.whisperModel} --host 127.0.0.1 --port 8790 -l en";
        Restart = "always";
      };
    };

    systemd.services.jean = lib.mkIf cfg.agent.enable {
      description = "The assistant (voice agent, builder, triggers)";
      wantedBy = [ "multi-user.target" ];
      after = [ "display-server.service" "whisper.service" ];
      wants = [ "display-server.service" "whisper.service" ];
      path = [ pkgs.sox pkgs.alsa-utils pkgs.piper-tts pkgs.git pkgs.python3 pkgs.bubblewrap pkgs.cargo pkgs.rustc pkgs.deno ];
      environment = {
        JEAN_MODE = "voice";
        JEAN_DISPLAY = "http://127.0.0.1:${toString cfg.port}";
        JEAN_DATA = "${cfg.root}/harness";
        JEAN_WHISPER_URL = "http://127.0.0.1:8790";
        JEAN_TTS = "piper";
        JEAN_PIPER_MODEL = "${cfg.agent.piperVoice}";
        JEAN_PLAYER = "aplay -q";
        JEAN_TZ = cfg.agent.timezone;
        JEAN_PLACE = cfg.agent.place;
        JEAN_MODEL = cfg.agent.model;
        JEAN_WAKE = lib.concatStringsSep "," cfg.agent.wake;
        JEAN_APPROVAL = cfg.agent.approval;
        JEAN_REPO = "${cfg.root}/repo";   # a checkout of this repository the builder works on
        JEAN_RESTART = "/run/wrappers/bin/sudo systemctl restart display-server.service";
      };
      serviceConfig = {
        User = "display";
        Group = "display";
        SupplementaryGroups = [ "audio" ];
        WorkingDirectory = cfg.root;
        EnvironmentFile = cfg.agent.apiKeyFile;
        ExecStart = "${pkg}/bin/jean";
        Restart = "always";
        RestartSec = 3;
      };
    };
    hardware.pulseaudio.enable = lib.mkDefault false;
    users.users.display.extraGroups = lib.mkIf cfg.agent.enable [ "audio" ];

    # --- the panel -----------------------------------------------------------------------------------------------------------------
    systemd.services.eink-serve = lib.mkIf cfg.panel.enable {
      description = "The display on a panel over VNC";
      wantedBy = [ "multi-user.target" ];
      after = [ "display-server.service" ];
      wants = [ "display-server.service" ];
      serviceConfig = {
        User = "display";
        Group = "display";
        ExecStart = "${pkg}/bin/eink-serve --display http://127.0.0.1:${toString cfg.port} --listen 0.0.0.0:${toString cfg.panel.port} --panel ${cfg.panel.geometry} --chrome ${pkgs.chromium}/bin/chromium";
        Restart = "always";
        RestartSec = 3;
      };
    };
    networking.firewall.allowedTCPPorts = lib.mkIf (cfg.panel.enable && cfg.panel.openFirewall) [ cfg.panel.port ];

    # --- the watchdog --------------------------------------------------------------------------------------------------------------
    systemd.services.display-watchdog = lib.mkIf cfg.watchdog.enable {
      description = "Restart the display server when it stops answering";
      serviceConfig.Type = "oneshot";
      path = [ pkgs.curl pkgs.systemd ];
      script = ''
        curl -sf --max-time 5 http://127.0.0.1:${toString cfg.port}/status >/dev/null || systemctl restart display-server.service
      '';
    };
    systemd.timers.display-watchdog = lib.mkIf cfg.watchdog.enable {
      wantedBy = [ "timers.target" ];
      timerConfig = { OnBootSec = "2min"; OnUnitActiveSec = "30s"; };
    };

    # Chromium in kiosk mode on a single-app Wayland session. The panel driver (e-ink over USB/SPI/HDMI) is hardware-specific
    # and not set up here; this just gives it a fullscreen page to mirror.
    services.cage = lib.mkIf cfg.kiosk {
      enable = true;
      user = "display";
      program = "${pkgs.chromium}/bin/chromium --kiosk --noerrdialogs --disable-infobars --force-device-scale-factor=1 --app=${url}";
    };
    systemd.services."cage-tty1".after = lib.mkIf cfg.kiosk [ "display-server.service" ];
  };
}
