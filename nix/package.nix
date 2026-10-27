# Everything built from this repository: the display server (UI kit, pages and fonts compiled in), the assistant (`jean`), the
# promotion pipeline's fixture gate, and `eink-serve` (the display on a panel over VNC).
# The fonts are Git LFS objects (build from a checkout with `git lfs pull`, or a flake input with ?lfs=1). Berkeley Mono is compiled in encrypted;
# the server unlocks it at startup with secrets/font.key.
{ rustPlatform, lib }:
rustPlatform.buildRustPackage {
  pname = "jean";
  version = "0.1.0";
  src = lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;
  cargoBuildFlags = [ "-p" "preview" "-p" "harness" "-p" "eink" ];
  doCheck = false;
  postInstall = ''
    mv $out/bin/preview $out/bin/display-server
  '';
  meta.mainProgram = "display-server";
}
