{
  description = "Living-room display: UI kit, display server, feeds";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});
    in
    {
      packages = nixpkgs.lib.recursiveUpdate (forAll (pkgs: { default = pkgs.callPackage ./nix/package.nix { }; })) {
        # the netboot VM's kernel, initrd and command line (see nixosConfigurations.vm-aarch64 below)
        aarch64-linux.vm-boot = let
          cfg = self.nixosConfigurations.vm-aarch64.config;
          pkgs = nixpkgs.legacyPackages.aarch64-linux;
        in pkgs.linkFarm "jean-vm-boot" [
          { name = "Image"; path = "${cfg.system.build.kernel}/${cfg.system.boot.loader.kernelFile}"; }
          { name = "initrd"; path = "${cfg.system.build.netbootRamdisk}/initrd"; }
          { name = "cmdline"; path = pkgs.writeText "cmdline" "init=${cfg.system.build.toplevel}/init ${toString cfg.boot.kernelParams}"; }
        ];
      };
      nixosModules.default = import ./nix/module.nix { inherit self; };
      # a VM that boots the module and checks the display answers: `nix build .#checks.x86_64-linux.vm`
      checks = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (system: {
        vm = nixpkgs.legacyPackages.${system}.testers.runNixOSTest {
          name = "display";
          nodes.machine = { ... }: {
            imports = [ self.nixosModules.default ];
            services.display = { enable = true; panel.enable = true; };
            virtualisation.memorySize = 2048;
          };
          testScript = ''
            machine.wait_for_unit("display-server.service")
            machine.wait_until_succeeds("curl -sf http://127.0.0.1:8788/status")
            machine.succeed("curl -sf http://127.0.0.1:8788/docs.md | grep -q Tile")
            machine.wait_for_unit("eink-serve.service")
            machine.wait_for_open_port(5900)
          '';
        };
      });
      # a VM to poke at by hand: `nix run .#vm` (port 8788 and 5900 are forwarded to the host)
      nixosConfigurations.vm = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          self.nixosModules.default
          ({ ... }: {
            services.display = { enable = true; panel.enable = true; };
            users.users.root.initialPassword = "root";
            virtualisation.vmVariant.virtualisation = { memorySize = 3072; forwardPorts = [ { from = "host"; host.port = 8788; guest.port = 8788; } { from = "host"; host.port = 5900; guest.port = 5900; } ]; };
            system.stateVersion = "25.05";
          })
        ];
      };
      # The same system as a netboot VM for QEMU on an aarch64 machine (Apple Silicon, accelerated): a kernel, an initrd that carries the whole system and the
      # kernel command line, with no disk image to build (which needs a VM of its own at build time). `packages.aarch64-linux.vm-boot`; see docs/harness.md.
      nixosConfigurations.vm-aarch64 = nixpkgs.lib.nixosSystem {
        system = "aarch64-linux";
        modules = [
          "${nixpkgs}/nixos/modules/installer/netboot/netboot-minimal.nix"
          self.nixosModules.default
          ({ pkgs, ... }: {
            services.display = { enable = true; kiosk = true; panel = { enable = true; geometry = "rm2-turned-ccw"; }; };
            users.users.root.initialPassword = "root";
            boot.kernelParams = [ "console=ttyAMA0" ];
            hardware.graphics.enable = true;
            # no GPU in the VM: draw in software
            systemd.services."cage-tty1".environment = { WLR_RENDERER = "pixman"; WLR_NO_HARDWARE_CURSORS = "1"; LIBGL_ALWAYS_SOFTWARE = "1"; };
            environment.systemPackages = [ pkgs.curl pkgs.htop ];
            system.stateVersion = "25.05";
          })
        ];
      };
      apps.x86_64-linux.vm = { type = "app"; program = "${self.nixosConfigurations.vm.config.system.build.vm}/bin/run-nixos-vm"; };
      devShells = forAll (pkgs: { default = pkgs.mkShell { packages = [ pkgs.cargo pkgs.rustc pkgs.python3 ]; }; });
    };
}
