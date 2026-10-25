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
