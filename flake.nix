{
  description = "Living-room display: UI kit, display server, feeds";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});
    in
    {
      packages = forAll (pkgs: { default = pkgs.callPackage ./nix/package.nix { }; });
      nixosModules.default = import ./nix/module.nix { inherit self; };
      devShells = forAll (pkgs: { default = pkgs.mkShell { packages = [ pkgs.cargo pkgs.rustc pkgs.python3 ]; }; });
    };
}
